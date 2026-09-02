use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output as CommandOutput};
use std::thread;
use std::time::Duration;

use serde_json::json;

use crate::{
    CompositorAdapter, CompositorCommand, Output, Rect, Toplevel, ToplevelEvent, ToplevelKind,
};

#[derive(Debug)]
pub struct AdapterError(String);

impl Display for AdapterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for AdapterError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WindowSnapshot {
    toplevel: Toplevel,
    rect: Rect,
    maximized: bool,
}

pub struct XwaylandEwmhAdapter {
    wmctrl: PathBuf,
    xprop: PathBuf,
    xwininfo: PathBuf,
    known: BTreeMap<String, Toplevel>,
    snapshots: BTreeMap<String, WindowSnapshot>,
    restore_rects: BTreeMap<String, Rect>,
    pending: VecDeque<ToplevelEvent>,
}

impl XwaylandEwmhAdapter {
    pub fn new() -> Self {
        let mut adapter = Self::with_commands("wmctrl", "xprop");
        adapter.xwininfo = "xwininfo".into();
        adapter
    }

    pub fn with_commands(wmctrl: impl Into<PathBuf>, xprop: impl Into<PathBuf>) -> Self {
        let wmctrl = wmctrl.into();
        Self {
            xwininfo: wmctrl.clone(),
            wmctrl,
            xprop: xprop.into(),
            known: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            restore_rects: BTreeMap::new(),
            pending: VecDeque::new(),
        }
    }

    pub fn output(&self) -> Result<Output, AdapterError> {
        let raw = self.run(&self.xwininfo, &["-root"])?;
        let dimension = |name: &str| {
            raw.lines().find_map(|line| {
                line.trim()
                    .strip_prefix(name)
                    .and_then(|value| value.trim().parse::<u32>().ok())
            })
        };
        if let (Some(width), Some(height)) = (dimension("Width:"), dimension("Height:")) {
            if width > 0 && height > 0 {
                return Ok(Output { width, height });
            }
        }
        Err(AdapterError(
            "xwininfo did not report a positive X root geometry".to_owned(),
        ))
    }

    fn refresh(&mut self) -> Result<(), AdapterError> {
        let current = self.read_inventory()?;
        for (id, snapshot) in &current {
            if self.known.get(id) != Some(&snapshot.toplevel) {
                self.pending
                    .push_back(ToplevelEvent::Upsert(snapshot.toplevel.clone()));
            }
        }
        let removed = self
            .known
            .keys()
            .filter(|id| !current.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        for id in removed {
            self.pending
                .push_back(ToplevelEvent::Remove { id: id.clone() });
            self.restore_rects.remove(&id);
        }
        self.known = current
            .iter()
            .map(|(id, snapshot)| (id.clone(), snapshot.toplevel.clone()))
            .collect();
        self.snapshots = current;
        Ok(())
    }

    fn read_inventory(&self) -> Result<BTreeMap<String, WindowSnapshot>, AdapterError> {
        let raw = self.run(&self.wmctrl, &["-lpGx"])?;
        let mut windows = BTreeMap::new();
        for line in raw.lines().filter(|line| !line.trim().is_empty()) {
            let mut fields = line.split_whitespace();
            let malformed = || {
                AdapterError(
                    "wmctrl returned a malformed nonempty inventory row; refusing partial observation"
                        .to_owned(),
                )
            };
            let id = fields.next().ok_or_else(malformed)?;
            let desktop = fields.next().ok_or_else(malformed)?;
            let pid = fields.next().ok_or_else(malformed)?;
            let x = fields.next().ok_or_else(malformed)?;
            let y = fields.next().ok_or_else(malformed)?;
            let width = fields.next().ok_or_else(malformed)?;
            let height = fields.next().ok_or_else(malformed)?;
            let _host = fields.next().ok_or_else(malformed)?;
            let wmctrl_class = fields.next().ok_or_else(malformed)?;
            let title = fields.collect::<Vec<_>>().join(" ");
            let _desktop = desktop.parse::<i32>().map_err(|_| malformed())?;
            let _pid = pid.parse::<u32>().map_err(|_| malformed())?;
            let x = x.parse::<i64>().map_err(|_| malformed())?;
            let y = y.parse::<i64>().map_err(|_| malformed())?;
            let width = width.parse::<u32>().map_err(|_| malformed())?;
            let height = height.parse::<u32>().map_err(|_| malformed())?;
            if x < 0 || y < 0 || width == 0 || height == 0 {
                return Err(AdapterError(
                    "wmctrl reported unsupported window geometry; refusing partial observation"
                        .to_owned(),
                ));
            }
            let properties = self.run(
                &self.xprop,
                &[
                    "-id",
                    id,
                    "WM_CLASS",
                    "_NET_WM_WINDOW_TYPE",
                    "WM_TRANSIENT_FOR",
                    "_NET_WM_STATE",
                ],
            )?;
            let app_id = parse_wm_class(&properties).unwrap_or_else(|| wmctrl_class.to_owned());
            let kind = parse_window_kind(&properties);
            let parent_id = parse_transient(&properties);
            let maximized = properties.contains("_NET_WM_STATE_MAXIMIZED_VERT")
                && properties.contains("_NET_WM_STATE_MAXIMIZED_HORZ");
            let toplevel = Toplevel {
                id: id.to_owned(),
                app_id,
                title,
                kind,
                parent_id,
            };
            windows.insert(
                id.to_owned(),
                WindowSnapshot {
                    toplevel,
                    rect: Rect {
                        x: x as u32,
                        y: y as u32,
                        width,
                        height,
                    },
                    maximized,
                },
            );
        }
        Ok(windows)
    }

    fn mutate(&self, args: &[&str]) -> Result<(), AdapterError> {
        self.run(&self.wmctrl, args).map(|_| ())
    }

    fn capture_restored_rect(&mut self, id: &str) -> Result<(), AdapterError> {
        // ponytail: bounded EWMH settle; widen only if live Labwc evidence requires it.
        for attempt in 0..10 {
            let current = self.read_inventory()?;
            let snapshot = current.get(id).ok_or_else(|| {
                AdapterError(format!("X11 toplevel {id} disappeared while unmaximizing"))
            })?;
            if !snapshot.maximized {
                self.restore_rects.insert(id.to_owned(), snapshot.rect);
                self.snapshots = current;
                return Ok(());
            }
            if attempt < 9 {
                thread::sleep(Duration::from_millis(25));
            }
        }
        Err(AdapterError(format!(
            "X11 toplevel {id} did not leave maximized state"
        )))
    }

    fn run(&self, command: &Path, args: &[&str]) -> Result<String, AdapterError> {
        // ponytail: four bounded attempts cover an atomic executable replacement.
        for attempt in 0..4 {
            match Command::new(command).args(args).output() {
                Ok(output) => return command_stdout(command, args, output),
                Err(error) if error.raw_os_error() == Some(26) && attempt < 3 => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => {
                    return Err(AdapterError(format!(
                        "cannot run {}: {error}",
                        command.display()
                    )));
                }
            }
        }
        unreachable!()
    }
}

impl Default for XwaylandEwmhAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositorAdapter for XwaylandEwmhAdapter {
    type Error = AdapterError;

    fn observe_toplevel(&mut self) -> Result<Option<ToplevelEvent>, Self::Error> {
        if self.pending.is_empty() {
            self.refresh()?;
        }
        Ok(self.pending.pop_front())
    }

    fn apply_commands(&mut self, commands: &[CompositorCommand]) -> Result<(), Self::Error> {
        self.snapshots = self.read_inventory()?;
        for command in commands {
            match command {
                CompositorCommand::Maximize { toplevel_id } => {
                    let snapshot = self.required(toplevel_id)?.clone();
                    if !snapshot.maximized {
                        self.restore_rects
                            .entry(toplevel_id.clone())
                            .or_insert(snapshot.rect);
                        self.mutate(&[
                            "-ir",
                            toplevel_id,
                            "-b",
                            "add,maximized_vert,maximized_horz",
                        ])?;
                    }
                }
                CompositorCommand::Unmaximize { toplevel_id }
                | CompositorCommand::Unsnap { toplevel_id } => {
                    let snapshot = self.required(toplevel_id)?.clone();
                    if snapshot.maximized {
                        self.mutate(&[
                            "-ir",
                            toplevel_id,
                            "-b",
                            "remove,maximized_vert,maximized_horz",
                        ])?;
                        if !self.restore_rects.contains_key(toplevel_id) {
                            self.capture_restored_rect(toplevel_id)?;
                        }
                    }
                    if let Some(rect) = self.restore_rects.get(toplevel_id).copied() {
                        if snapshot.rect != rect || snapshot.maximized {
                            let geometry =
                                format!("0,{},{},{},{}", rect.x, rect.y, rect.width, rect.height);
                            self.mutate(&["-ir", toplevel_id, "-e", &geometry])?;
                        }
                        self.restore_rects.remove(toplevel_id);
                    }
                }
                CompositorCommand::Snap {
                    toplevel_id,
                    region: _,
                    rect,
                } => {
                    let snapshot = self.required(toplevel_id)?.clone();
                    if !snapshot.maximized {
                        self.restore_rects
                            .entry(toplevel_id.clone())
                            .or_insert(snapshot.rect);
                    }
                    if snapshot.maximized {
                        self.mutate(&[
                            "-ir",
                            toplevel_id,
                            "-b",
                            "remove,maximized_vert,maximized_horz",
                        ])?;
                        if !self.restore_rects.contains_key(toplevel_id) {
                            self.capture_restored_rect(toplevel_id)?;
                        }
                    }
                    if snapshot.rect != *rect || snapshot.maximized {
                        let geometry =
                            format!("0,{},{},{},{}", rect.x, rect.y, rect.width, rect.height);
                        self.mutate(&["-ir", toplevel_id, "-e", &geometry])?;
                    }
                }
                CompositorCommand::SetDecoration { .. } => {
                    return Err(AdapterError(
                        "dynamic decoration mutation is not available through EWMH".to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }
}

impl XwaylandEwmhAdapter {
    fn required(&self, id: &str) -> Result<&WindowSnapshot, AdapterError> {
        self.snapshots.get(id).ok_or_else(|| {
            AdapterError(format!(
                "X11 toplevel {id} is unavailable or unsupported; refusing to guess a replacement"
            ))
        })
    }
}

fn command_stdout(
    command: &Path,
    args: &[&str],
    output: CommandOutput,
) -> Result<String, AdapterError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if args == ["-lpGx"]
            && stderr.contains("Cannot get client list properties.")
            && stderr.contains("_NET_CLIENT_LIST or _WIN_CLIENT_LIST")
        {
            return Ok(String::new());
        }
        return Err(AdapterError(format!(
            "{} {} failed: {}",
            command.display(),
            args.join(" "),
            stderr.trim()
        )));
    }
    String::from_utf8(output.stdout).map_err(|error| {
        AdapterError(format!(
            "{} returned non-UTF-8 output: {error}",
            command.display()
        ))
    })
}

fn parse_wm_class(properties: &str) -> Option<String> {
    let line = properties
        .lines()
        .find(|line| line.starts_with("WM_CLASS"))?;
    let values = line.split_once('=')?.1;
    let quoted = values
        .split(',')
        .filter_map(|value| {
            let value = value.trim();
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
        })
        .collect::<Vec<_>>();
    match quoted.as_slice() {
        [instance, class, ..] => Some(format!("{instance}.{class}")),
        [class] => Some((*class).to_owned()),
        _ => None,
    }
}

fn parse_window_kind(properties: &str) -> ToplevelKind {
    let line = properties
        .lines()
        .find(|line| line.starts_with("_NET_WM_WINDOW_TYPE"))
        .unwrap_or("");
    if line.contains("_NET_WM_WINDOW_TYPE_DIALOG") {
        ToplevelKind::Dialog
    } else if line.contains("_NET_WM_WINDOW_TYPE_UTILITY") {
        ToplevelKind::Utility
    } else if line.contains("_NET_WM_WINDOW_TYPE_DESKTOP") {
        ToplevelKind::Desktop
    } else if line.contains("_NET_WM_WINDOW_TYPE_NORMAL") {
        ToplevelKind::Normal
    } else {
        ToplevelKind::Other
    }
}

fn parse_transient(properties: &str) -> Option<String> {
    let line = properties
        .lines()
        .find(|line| line.starts_with("WM_TRANSIENT_FOR"))?;
    if line.contains("not found") {
        return None;
    }
    line.split_whitespace()
        .find(|part| part.starts_with("0x") && *part != "0x0")
        .map(|part| part.trim_end_matches(',').to_owned())
}

pub fn runtime_state_path() -> Option<PathBuf> {
    pelagian_shellctl::layoutd_state_path()
}

pub fn write_runtime_state(
    layout_mode: &str,
    managed: usize,
    floating: usize,
) -> Result<(), AdapterError> {
    let path = runtime_state_path()
        .ok_or_else(|| AdapterError("layoutd state path is not configured".to_owned()))?;
    if env::var_os("PELAGIAN_LAYOUTD_STATE").is_none() {
        let parent = path
            .parent()
            .ok_or_else(|| AdapterError("invalid layoutd state path".to_owned()))?;
        fs::create_dir_all(parent).map_err(|error| {
            AdapterError(format!("cannot create {}: {error}", parent.display()))
        })?;
        let metadata = fs::symlink_metadata(parent).map_err(|error| {
            AdapterError(format!("cannot inspect {}: {error}", parent.display()))
        })?;
        if !metadata.file_type().is_dir() {
            return Err(AdapterError(format!(
                "layoutd state parent {} is not a directory",
                parent.display()
            )));
        }
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|error| {
            AdapterError(format!("cannot secure {}: {error}", parent.display()))
        })?;
    }
    write_runtime_state_at(&path, layout_mode, managed, floating)
}

fn write_runtime_state_at(
    path: &Path,
    layout_mode: &str,
    managed: usize,
    floating: usize,
) -> Result<(), AdapterError> {
    let parent = path
        .parent()
        .ok_or_else(|| AdapterError("invalid layoutd state path".to_owned()))?;
    fs::create_dir_all(parent)
        .map_err(|error| AdapterError(format!("cannot create {}: {error}", parent.display())))?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let body = json!({
        "schema_version": 1,
        "pid": std::process::id(),
        "layout_mode": layout_mode,
        "layoutd": "running",
        "compositor_adapter": "xwayland-ewmh",
        "adapter_scope": "xwayland",
        "native_wayland_control": false,
        "managed_windows": managed,
        "floating_windows": floating,
    });
    let result = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .and_then(|mut file| {
            file.write_all(format!("{body}\n").as_bytes())?;
            file.sync_all()
        })
        .and_then(|_| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| AdapterError(format!("cannot write {}: {error}", path.display())))
}

pub fn runtime_status_json() -> String {
    let running = pelagian_shellctl::layoutd_running();
    let layout_mode = runtime_state_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|value| {
            value
                .get("layout_mode")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    json!({
        "schema_version": 1,
        "layout_mode": layout_mode,
        "layoutd": if running { "running" } else { "stopped" },
        "compositor_adapter": "xwayland-ewmh",
        "adapter_scope": "xwayland",
        "native_wayland_control": false,
        "decoration_control": "static-labwc-only",
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::{PermissionsExt, symlink};

    use super::*;

    #[test]
    fn runtime_and_status_use_the_same_state_path() {
        assert_eq!(
            runtime_state_path(),
            pelagian_shellctl::layoutd_state_path()
        );
    }

    #[test]
    fn command_execution_retries_a_transient_text_file_busy_error() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "pelagian-layoutd-command-{}-{nonce}",
            std::process::id()
        ));
        let command = root.join("command");
        fs::create_dir_all(&root).unwrap();
        fs::write(&command, "#!/bin/sh\nprintf 'ready\\n'\n").unwrap();
        let mut permissions = fs::metadata(&command).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&command, permissions).unwrap();
        let writer = OpenOptions::new().write(true).open(&command).unwrap();
        let release = thread::spawn(move || {
            thread::sleep(Duration::from_millis(25));
            drop(writer);
        });

        let adapter = XwaylandEwmhAdapter::with_commands(&command, &command);
        assert_eq!(adapter.run(&command, &[]).unwrap().trim(), "ready");

        release.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn state_write_does_not_follow_temporary_symlinks() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "pelagian-layoutd-state-{}-{nonce}",
            std::process::id()
        ));
        let state = root.join("status.json");
        let victim = root.join("victim");
        fs::create_dir_all(&root).unwrap();
        fs::write(&victim, "keep\n").unwrap();
        symlink(&victim, state.with_extension("tmp")).unwrap();

        write_runtime_state_at(&state, "auto", 1, 0).unwrap();

        assert_eq!(fs::read_to_string(&victim).unwrap(), "keep\n");
        symlink(
            &victim,
            state.with_extension(format!("tmp-{}", std::process::id())),
        )
        .unwrap();
        assert!(write_runtime_state_at(&state, "auto", 2, 0).is_err());
        assert_eq!(fs::read_to_string(victim).unwrap(), "keep\n");
        fs::remove_dir_all(root).unwrap();
    }
}
