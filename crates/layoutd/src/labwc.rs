use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Deserialize;
use socket2::{Domain, SockAddr, Socket, Type};

use crate::{
    CompositorAdapter, CompositorCommand, DecorationState, LayoutRequest, Output, Toplevel,
    ToplevelEvent, ToplevelKind, Workspace, WorkspacePlan,
};

const IPC_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub struct AdapterError {
    message: String,
    adapter_connected: bool,
}

impl AdapterError {
    fn disconnected(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            adapter_connected: false,
        }
    }

    fn connected(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            adapter_connected: true,
        }
    }

    pub fn adapter_connected(&self) -> bool {
        self.adapter_connected
    }
}

impl Display for AdapterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
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
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
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
    output_origin: (i32, i32),
}

impl LabwcIpcAdapter {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
            known: BTreeMap::new(),
            layout_state: BTreeMap::new(),
            pending: VecDeque::new(),
            output: None,
            output_origin: (0, 0),
        }
    }

    pub fn output(&self) -> Result<Output, AdapterError> {
        self.output
            .ok_or_else(|| AdapterError::connected("Labwc IPC did not report a usable output"))
    }

    /// Read exactly one inventory per daemon tick. Continuously changing
    /// clients must not keep an event-draining loop from reaching its deadline.
    pub fn observe_workspace(&mut self, workspace: &mut Workspace) -> Result<(), AdapterError> {
        self.refresh()?;
        for event in self.pending.drain(..) {
            workspace.apply(event);
        }
        Ok(())
    }

    /// Verify observed outer geometry and compositor state, never ACTION ACKs.
    pub fn convergence_error(&self, plan: &WorkspacePlan) -> Option<String> {
        for placement in &plan.placements {
            let view = placement
                .id
                .parse::<u64>()
                .ok()
                .and_then(|id| self.layout_state.get(&id));
            let Some(view) = view else {
                return Some(format!("window {} is absent", placement.id));
            };
            let rect = placement.rect;
            let geometry_matches = i64::from(view.x)
                == i64::from(self.output_origin.0) + i64::from(rect.x)
                && i64::from(view.y) == i64::from(self.output_origin.1) + i64::from(rect.y)
                && view.width == rect.width
                && view.height == rect.height;
            let placement_matches = match &placement.request {
                LayoutRequest::Maximize => view.maximized && !view.tiled && view.region.is_empty(),
                LayoutRequest::Snap { region } => {
                    !view.maximized && view.tiled && view.region == *region
                }
            };
            if !geometry_matches
                || !placement_matches
                || view.minimized
                || view.fullscreen
                || view.decoration != "full"
                || !view.titlebar_visible
            {
                return Some(format!(
                    "window {} has not reached its planned geometry/state",
                    placement.id
                ));
            }
        }
        for id in &plan.floating {
            let view = id
                .parse::<u64>()
                .ok()
                .and_then(|id| self.layout_state.get(&id));
            if !view.is_some_and(|view| {
                view.decoration == "full"
                    && !view.maximized
                    && !view.tiled
                    && view.region.is_empty()
            }) {
                return Some(format!("window {id} has not reached floating state"));
            }
        }
        None
    }

    fn request(&self, request: &str) -> Result<String, AdapterError> {
        let deadline = Instant::now() + IPC_TIMEOUT;
        let socket = Socket::new(Domain::UNIX, Type::STREAM, None).map_err(|error| {
            AdapterError::disconnected(format!("cannot create Labwc IPC socket: {error}"))
        })?;
        let address = SockAddr::unix(&self.socket).map_err(|error| {
            AdapterError::disconnected(format!(
                "invalid Labwc IPC socket path {}: {error}",
                self.socket.display()
            ))
        })?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(AdapterError::disconnected(
                "Labwc IPC request timeout: deadline exceeded",
            ));
        }
        socket
            .connect_timeout(&address, remaining)
            .map_err(|error| {
                AdapterError::disconnected(format!(
                    "cannot connect to {} within {} ms: {error}",
                    self.socket.display(),
                    IPC_TIMEOUT.as_millis()
                ))
            })?;
        let mut stream = UnixStream::from(OwnedFd::from(socket));
        let mut request = request.as_bytes();
        while !request.is_empty() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(AdapterError::disconnected(
                    "Labwc IPC request timeout: deadline exceeded",
                ));
            }
            stream.set_write_timeout(Some(remaining)).map_err(|error| {
                AdapterError::disconnected(format!("cannot bound Labwc IPC request: {error}"))
            })?;
            let written = stream.write(request).map_err(|error| {
                AdapterError::disconnected(format!("cannot write Labwc IPC request: {error}"))
            })?;
            if written == 0 {
                return Err(AdapterError::disconnected(
                    "cannot write Labwc IPC request: connection closed",
                ));
            }
            request = &request[written..];
        }
        let mut response = Vec::new();
        let mut chunk = [0_u8; 8192];
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(AdapterError::disconnected(
                    "Labwc IPC response timeout: deadline exceeded",
                ));
            }
            stream.set_read_timeout(Some(remaining)).map_err(|error| {
                AdapterError::disconnected(format!("cannot bound Labwc IPC response: {error}"))
            })?;
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(length) => {
                    if response.len() + length > MAX_RESPONSE_BYTES {
                        return Err(AdapterError::connected(format!(
                            "Labwc IPC response exceeds {MAX_RESPONSE_BYTES} bytes"
                        )));
                    }
                    response.extend_from_slice(&chunk[..length]);
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
                {
                    return Err(AdapterError::disconnected(
                        "Labwc IPC response timeout: deadline exceeded",
                    ));
                }
                Err(error) => {
                    return Err(AdapterError::disconnected(format!(
                        "cannot read Labwc IPC response: {error}"
                    )));
                }
            }
        }
        String::from_utf8(response).map_err(|error| {
            AdapterError::connected(format!("Labwc IPC response is not UTF-8: {error}"))
        })
    }

    fn refresh(&mut self) -> Result<(), AdapterError> {
        let raw = self.request("LIST\n")?;
        let inventory: Inventory = serde_json::from_str(&raw).map_err(|error| {
            AdapterError::connected(format!("invalid Labwc IPC inventory: {error}"))
        })?;
        let output = inventory
            .outputs
            .first()
            .map(|output| Output {
                width: output.usable_area.width,
                height: output.usable_area.height,
            })
            .filter(|output| output.width > 0 && output.height > 0)
            .ok_or_else(|| AdapterError::connected("Labwc IPC did not report a usable output"))?;
        let area = &inventory.outputs[0].usable_area;
        self.output_origin = (area.x, area.y);
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
            .map_err(|_| AdapterError::connected(format!("invalid Labwc toplevel id: {id}")))?;
        if !self.known.contains_key(&numeric_id) {
            return Err(AdapterError::connected(format!(
                "Labwc toplevel {id} disappeared before reconciliation"
            )));
        }
        let raw = self.request(&format!("ACTION {id} {action}\n"))?;
        let response: ActionResponse = serde_json::from_str(&raw).map_err(|error| {
            AdapterError::connected(format!("invalid Labwc IPC action response: {error}"))
        })?;
        if response.ok {
            Ok(())
        } else {
            Err(AdapterError::connected(format!(
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
                CompositorCommand::Unmanage { toplevel_id } => {
                    self.action(toplevel_id, "UNMANAGE")?
                }
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
