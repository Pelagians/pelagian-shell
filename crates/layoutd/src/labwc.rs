use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

use crate::{
    CompositorAdapter, CompositorCommand, DecorationState, Output, Toplevel, ToplevelEvent,
    ToplevelKind,
};

const IPC_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub struct AdapterError(String);

impl Display for AdapterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for AdapterError {}

#[derive(Deserialize)]
struct Inventory {
    views: Vec<IpcView>,
    outputs: Vec<IpcOutput>,
}

#[derive(Deserialize)]
struct IpcView {
    id: u64,
    app_id: String,
    title: String,
    #[serde(rename = "type")]
    kind: String,
    parent_id: Option<u64>,
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    minimized: bool,
    #[serde(default)]
    fullscreen: bool,
    #[serde(default)]
    maximized: bool,
    #[serde(default)]
    tiled: bool,
    #[serde(default)]
    region: String,
    #[serde(default)]
    decoration: String,
    #[serde(default)]
    titlebar_visible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IpcLayoutState {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    minimized: bool,
    fullscreen: bool,
    maximized: bool,
    tiled: bool,
    region: String,
    decoration: String,
    titlebar_visible: bool,
}

#[derive(Deserialize)]
struct IpcOutput {
    usable_area: IpcRect,
}

#[derive(Deserialize)]
struct IpcRect {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct ActionResponse {
    ok: bool,
    #[serde(default)]
    error: String,
}

pub struct LabwcIpcAdapter {
    socket: PathBuf,
    known: BTreeMap<u64, Toplevel>,
    layout_state: BTreeMap<u64, IpcLayoutState>,
    pending: VecDeque<ToplevelEvent>,
    output: Option<Output>,
}

impl LabwcIpcAdapter {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
            known: BTreeMap::new(),
            layout_state: BTreeMap::new(),
            pending: VecDeque::new(),
            output: None,
        }
    }

    pub fn output(&self) -> Result<Output, AdapterError> {
        self.output
            .ok_or_else(|| AdapterError("Labwc IPC did not report a usable output".to_owned()))
    }

    fn request(&self, request: &str) -> Result<String, AdapterError> {
        let mut stream = UnixStream::connect(&self.socket).map_err(|error| {
            AdapterError(format!(
                "cannot connect to {}: {error}",
                self.socket.display()
            ))
        })?;
        stream
            .set_read_timeout(Some(IPC_TIMEOUT))
            .and_then(|()| stream.set_write_timeout(Some(IPC_TIMEOUT)))
            .map_err(|error| AdapterError(format!("cannot bound Labwc IPC request: {error}")))?;
        stream
            .write_all(request.as_bytes())
            .map_err(|error| AdapterError(format!("cannot write Labwc IPC request: {error}")))?;
        let mut response = Vec::new();
        stream
            .take(MAX_RESPONSE_BYTES as u64 + 1)
            .read_to_end(&mut response)
            .map_err(|error| {
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) {
                    AdapterError("Labwc IPC response timed out".to_owned())
                } else {
                    AdapterError(format!("cannot read Labwc IPC response: {error}"))
                }
            })?;
        if response.len() > MAX_RESPONSE_BYTES {
            return Err(AdapterError(format!(
                "Labwc IPC response exceeds {MAX_RESPONSE_BYTES} bytes"
            )));
        }
        String::from_utf8(response)
            .map_err(|error| AdapterError(format!("Labwc IPC response is not UTF-8: {error}")))
    }

    fn refresh(&mut self) -> Result<(), AdapterError> {
        let raw = self.request("LIST\n")?;
        let inventory: Inventory = serde_json::from_str(&raw)
            .map_err(|error| AdapterError(format!("invalid Labwc IPC inventory: {error}")))?;
        let output = inventory
            .outputs
            .first()
            .map(|output| Output {
                width: output.usable_area.width,
                height: output.usable_area.height,
            })
            .filter(|output| output.width > 0 && output.height > 0)
            .ok_or_else(|| AdapterError("Labwc IPC did not report a usable output".to_owned()))?;
        let mut current = BTreeMap::new();
        let mut layout_state = BTreeMap::new();
        for view in inventory.views {
            let id = view.id;
            let kind = match view.kind.as_str() {
                "normal" => ToplevelKind::Normal,
                "dialog" => ToplevelKind::Dialog,
                "utility" => ToplevelKind::Utility,
                "desktop" => ToplevelKind::Desktop,
                _ => ToplevelKind::Other,
            };
            let toplevel = Toplevel {
                id: id.to_string(),
                app_id: view.app_id,
                title: view.title,
                kind,
                parent_id: view.parent_id.map(|parent_id| parent_id.to_string()),
            };
            layout_state.insert(
                id,
                IpcLayoutState {
                    x: view.x,
                    y: view.y,
                    width: view.width,
                    height: view.height,
                    minimized: view.minimized,
                    fullscreen: view.fullscreen,
                    maximized: view.maximized,
                    tiled: view.tiled,
                    region: view.region,
                    decoration: view.decoration,
                    titlebar_visible: view.titlebar_visible,
                },
            );
            current.insert(id, toplevel);
        }

        let mut topology_changed = false;
        for (id, toplevel) in &current {
            if self.known.get(id) != Some(toplevel) {
                self.pending
                    .push_back(ToplevelEvent::Upsert(toplevel.clone()));
                topology_changed = true;
            }
        }
        for id in self.known.keys().filter(|id| !current.contains_key(*id)) {
            self.pending
                .push_back(ToplevelEvent::Remove { id: id.to_string() });
            topology_changed = true;
        }
        let layout_changed = self.output.is_some_and(|previous| previous != output)
            || self.layout_state != layout_state;
        if !topology_changed && layout_changed {
            self.pending.push_back(ToplevelEvent::Reconcile);
        }
        self.known = current;
        self.layout_state = layout_state;
        self.output = Some(output);
        Ok(())
    }

    fn action(&self, id: &str, action: &str) -> Result<(), AdapterError> {
        let numeric_id = id
            .parse::<u64>()
            .map_err(|_| AdapterError(format!("invalid Labwc toplevel id: {id}")))?;
        if !self.known.contains_key(&numeric_id) {
            return Err(AdapterError(format!(
                "Labwc toplevel {id} disappeared before reconciliation"
            )));
        }
        let raw = self.request(&format!("ACTION {id} {action}\n"))?;
        let response: ActionResponse = serde_json::from_str(&raw)
            .map_err(|error| AdapterError(format!("invalid Labwc IPC action response: {error}")))?;
        if response.ok {
            Ok(())
        } else {
            Err(AdapterError(format!(
                "Labwc rejected {action}: {}",
                response.error
            )))
        }
    }
}

impl CompositorAdapter for LabwcIpcAdapter {
    type Error = AdapterError;

    fn observe_toplevel(&mut self) -> Result<Option<ToplevelEvent>, Self::Error> {
        if self.pending.is_empty() {
            self.refresh()?;
        }
        Ok(self.pending.pop_front())
    }

    fn apply_commands(&mut self, commands: &[CompositorCommand]) -> Result<(), Self::Error> {
        for command in commands {
            match command {
                CompositorCommand::Maximize { toplevel_id } => {
                    self.action(toplevel_id, "MAXIMIZE")?
                }
                CompositorCommand::Snap {
                    toplevel_id,
                    region,
                } => self.action(toplevel_id, &format!("SNAP {region}"))?,
                CompositorCommand::Unmaximize { toplevel_id }
                | CompositorCommand::Unsnap { toplevel_id } => self.action(toplevel_id, "FLOAT")?,
                CompositorCommand::SetDecoration {
                    toplevel_id,
                    decoration,
                } => {
                    let decoration = match decoration {
                        DecorationState::None => "none",
                        DecorationState::Border => "border",
                        DecorationState::Full => "full",
                    };
                    self.action(toplevel_id, &format!("DECORATION {decoration}"))?
                }
            }
        }
        Ok(())
    }
}

impl Default for LabwcIpcAdapter {
    fn default() -> Self {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        Self::new(runtime.join("labwc.sock"))
    }
}
