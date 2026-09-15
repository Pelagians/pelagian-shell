use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use pelagian_shellctl::{Config, LayoutMode, WindowDisposition};
use serde::{Deserialize, Serialize};

use crate::{Classification, WindowRule};

pub struct RuntimeSettings {
    pub automatic: bool,
    pub max_managed_windows: usize,
    pub window_rules: Vec<WindowRule>,
}

impl RuntimeSettings {
    pub fn from_config(config: &Config) -> Self {
        Self {
            automatic: matches!(config.layout.mode, LayoutMode::Auto),
            max_managed_windows: usize::from(config.layout.max_managed_windows),
            window_rules: config
                .window_rules
                .iter()
                .map(|rule| WindowRule {
                    app_id: rule.app_id.clone(),
                    title: rule.title.clone(),
                    disposition: match rule.disposition {
                        WindowDisposition::Managed => Classification::Managed,
                        WindowDisposition::Floating => Classification::Floating,
                        WindowDisposition::Ignored => Classification::Ignored,
                    },
                })
                .collect(),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct RuntimeState {
    schema_version: u32,
    pid: u32,
    layout_mode: String,
    layoutd: String,
    compositor_adapter: String,
    adapter_connected: bool,
    reconciliation: String,
    managed_windows: usize,
    floating_windows: usize,
    last_error: Option<String>,
}

fn state_path() -> PathBuf {
    env::var_os("PELAGIAN_LAYOUTD_STATE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/config/.local/state"))
                .join("pelagian-shell/layoutd.json")
        })
}

pub fn write_runtime_state(
    layout_mode: &str,
    layoutd: &str,
    adapter_connected: bool,
    reconciliation: &str,
    managed_windows: usize,
    floating_windows: usize,
    last_error: Option<&str>,
) -> Result<(), std::io::Error> {
    let path = state_path();
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid layoutd state path",
        )
    })?;
    fs::create_dir_all(parent)?;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let state = RuntimeState {
        schema_version: 1,
        pid: std::process::id(),
        layout_mode: layout_mode.to_owned(),
        layoutd: layoutd.to_owned(),
        compositor_adapter: "labwc-ipc".to_owned(),
        adapter_connected,
        reconciliation: reconciliation.to_owned(),
        managed_windows,
        floating_windows,
        last_error: last_error.map(str::to_owned),
    };
    let body = serde_json::to_vec(&state)?;
    let result = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .and_then(|mut file| {
            file.write_all(&body)?;
            file.write_all(b"\n")?;
            file.sync_all()
        })
        .and_then(|_| fs::rename(&temporary, &path));
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn cmdline_executable(cmdline: &[u8]) -> Option<PathBuf> {
    let end = cmdline.iter().position(|byte| *byte == 0)?;
    (end > 0).then(|| PathBuf::from(std::ffi::OsStr::from_bytes(&cmdline[..end])))
}

fn process_matches_layoutd(
    expected: &Path,
    executable: io::Result<PathBuf>,
    cmdline: impl FnOnce() -> io::Result<Vec<u8>>,
) -> bool {
    match executable {
        Ok(path) => fs::canonicalize(path).is_ok_and(|actual| actual == expected),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => cmdline()
            .ok()
            .as_deref()
            .and_then(cmdline_executable)
            .and_then(|path| fs::canonicalize(path).ok())
            .is_some_and(|actual| actual == expected),
        Err(_) => false,
    }
}

fn process_is_layoutd(pid: u32) -> bool {
    let Some(expected) = env::current_exe()
        .ok()
        .and_then(|path| fs::canonicalize(path).ok())
    else {
        return false;
    };
    process_matches_layoutd(&expected, fs::read_link(format!("/proc/{pid}/exe")), || {
        fs::read(format!("/proc/{pid}/cmdline"))
    })
}

pub fn runtime_status_json() -> String {
    let state = fs::read_to_string(state_path())
        .ok()
        .and_then(|raw| serde_json::from_str::<RuntimeState>(&raw).ok());
    let running = state
        .as_ref()
        .is_some_and(|state| process_is_layoutd(state.pid));
    let status = RuntimeState {
        schema_version: 1,
        pid: state.as_ref().map_or(0, |state| state.pid),
        layout_mode: state
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), |state| state.layout_mode.clone()),
        layoutd: if running {
            state
                .as_ref()
                .map_or_else(|| "running".to_owned(), |state| state.layoutd.clone())
        } else {
            "stopped".to_owned()
        },
        compositor_adapter: "labwc-ipc".to_owned(),
        adapter_connected: running && state.as_ref().is_some_and(|state| state.adapter_connected),
        reconciliation: if running {
            state.as_ref().map_or_else(
                || "unknown".to_owned(),
                |state| state.reconciliation.clone(),
            )
        } else {
            "stopped".to_owned()
        },
        managed_windows: state.as_ref().map_or(0, |state| state.managed_windows),
        floating_windows: state.as_ref().map_or(0, |state| state.floating_windows),
        last_error: state.and_then(|state| state.last_error),
    };
    serde_json::to_string(&status).expect("serializing fixed runtime status cannot fail")
}

#[cfg(test)]
mod tests {
    use super::process_matches_layoutd;
    use std::io;
    use std::os::unix::ffi::OsStrExt;

    #[test]
    fn cmdline_fallback_is_permission_only() {
        let expected = std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
        let mut cmdline = expected.as_os_str().as_bytes().to_vec();
        cmdline.push(0);

        assert!(!process_matches_layoutd(
            &expected,
            Ok("/bin/sh".into()),
            || Ok(cmdline.clone())
        ));
        assert!(process_matches_layoutd(
            &expected,
            Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            || Ok(cmdline.clone())
        ));
        assert!(!process_matches_layoutd(
            &expected,
            Err(io::Error::from(io::ErrorKind::NotFound)),
            || Ok(cmdline)
        ));
    }
}
