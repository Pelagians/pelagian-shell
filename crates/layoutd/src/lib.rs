//! Deterministic workspace model, layout planner, and narrow XWayland adapter.

mod runtime;
mod xwayland;

pub use runtime::RuntimeSettings;
pub use xwayland::{
    AdapterError, XwaylandEwmhAdapter, runtime_state_path, runtime_status_json, write_runtime_state,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Output {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutRequest {
    /// A compositor maximize request, deliberately distinct from fullscreen.
    Maximize,
    /// A future compositor adapter should snap this to the named Labwc region.
    Snap { region: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub id: String,
    pub rect: Rect,
    pub request: LayoutRequest,
}

/// A compositor-reported toplevel. `id` is stable only for this session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toplevel {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub kind: ToplevelKind,
    pub parent_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToplevelKind {
    Normal,
    Dialog,
    Utility,
    Desktop,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Classification {
    Managed,
    Floating,
    Ignored,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowRule {
    pub app_id: String,
    pub title: Option<String>,
    pub disposition: Classification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToplevelEvent {
    Upsert(Toplevel),
    Remove { id: String },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClassifiedWindows {
    pub managed: Vec<String>,
    pub floating: Vec<String>,
    pub ignored: Vec<String>,
}

/// Session-local model. Insertion order is the stable automatic-layout order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Workspace {
    toplevels: Vec<Toplevel>,
}

impl Workspace {
    pub fn apply(&mut self, event: ToplevelEvent) {
        match event {
            ToplevelEvent::Upsert(toplevel) => self.upsert(toplevel),
            ToplevelEvent::Remove { id } => self.remove(&id),
        }
    }

    pub fn upsert(&mut self, toplevel: Toplevel) {
        if let Some(existing) = self
            .toplevels
            .iter_mut()
            .find(|existing| existing.id == toplevel.id)
        {
            *existing = toplevel;
        } else {
            self.toplevels.push(toplevel);
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.toplevels.retain(|toplevel| toplevel.id != id);
    }

    pub fn classify(&self, rules: &[WindowRule]) -> ClassifiedWindows {
        let mut windows = ClassifiedWindows::default();
        for toplevel in &self.toplevels {
            match classify_toplevel(toplevel, rules) {
                Classification::Managed => windows.managed.push(toplevel.id.clone()),
                Classification::Floating => windows.floating.push(toplevel.id.clone()),
                Classification::Ignored => windows.ignored.push(toplevel.id.clone()),
            }
        }
        windows
    }

    pub fn classification_of(&self, id: &str, rules: &[WindowRule]) -> Option<Classification> {
        self.toplevels
            .iter()
            .find(|toplevel| toplevel.id == id)
            .map(|toplevel| classify_toplevel(toplevel, rules))
    }

    pub fn plan(&self, output: Output, rules: &[WindowRule], maximum: usize) -> WorkspacePlan {
        let mut classified = self.classify(rules);
        let overflow = classified
            .managed
            .split_off(maximum.min(classified.managed.len()));
        classified.floating.extend(overflow);
        let placements = plan(output, &classified.managed);
        WorkspacePlan {
            placements,
            floating: classified.floating,
            ignored: classified.ignored,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspacePlan {
    pub placements: Vec<Placement>,
    pub floating: Vec<String>,
    pub ignored: Vec<String>,
}

/// A supported adapter may apply only these narrow, per-toplevel operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositorCommand {
    Maximize {
        toplevel_id: String,
    },
    Unmaximize {
        toplevel_id: String,
    },
    Snap {
        toplevel_id: String,
        region: String,
        rect: Rect,
    },
    Unsnap {
        toplevel_id: String,
    },
    SetDecoration {
        toplevel_id: String,
        decoration: DecorationState,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationState {
    None,
    Border,
    Full,
}

/// The only live-compositor dependency.
pub trait CompositorAdapter {
    type Error;

    fn observe_toplevel(&mut self) -> Result<Option<ToplevelEvent>, Self::Error>;
    fn apply_commands(&mut self, commands: &[CompositorCommand]) -> Result<(), Self::Error>;
}

/// Translate pure planner output into the small semantic adapter contract.
pub fn reconcile_commands(placements: &[Placement]) -> Vec<CompositorCommand> {
    placements
        .iter()
        .map(|placement| match &placement.request {
            LayoutRequest::Maximize => CompositorCommand::Maximize {
                toplevel_id: placement.id.clone(),
            },
            LayoutRequest::Snap { region } => CompositorCommand::Snap {
                toplevel_id: placement.id.clone(),
                region: region.clone(),
                rect: placement.rect,
            },
        })
        .collect()
}

pub fn transition_commands(
    previous_placements: &[String],
    plan: &WorkspacePlan,
) -> Vec<CompositorCommand> {
    let mut commands = previous_placements
        .iter()
        .filter(|id| plan.floating.contains(id) || plan.ignored.contains(id))
        .map(|id| CompositorCommand::Unsnap {
            toplevel_id: id.clone(),
        })
        .collect::<Vec<_>>();
    commands.extend(reconcile_commands(&plan.placements));
    commands
}

pub fn classify_toplevel(toplevel: &Toplevel, rules: &[WindowRule]) -> Classification {
    let mut classification = match toplevel.kind {
        ToplevelKind::Normal if toplevel.parent_id.is_none() => Classification::Managed,
        ToplevelKind::Normal | ToplevelKind::Dialog | ToplevelKind::Utility => {
            Classification::Floating
        }
        ToplevelKind::Desktop | ToplevelKind::Other => Classification::Ignored,
    };
    for rule in rules {
        if (rule.app_id == "*" || rule.app_id == toplevel.app_id)
            && rule
                .title
                .as_deref()
                .is_none_or(|pattern| glob_matches(pattern, &toplevel.title))
        {
            classification = rule.disposition;
        }
    }
    classification
}

fn glob_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    let anchored_start = !pattern.starts_with('*');
    let anchored_end = !pattern.ends_with('*');
    let mut offset = 0;
    for (index, piece) in pattern
        .split('*')
        .filter(|piece| !piece.is_empty())
        .enumerate()
    {
        let Some(found) = value[offset..].find(piece) else {
            return false;
        };
        if index == 0 && anchored_start && found != 0 {
            return false;
        }
        offset += found + piece.len();
    }
    !anchored_end || offset == value.len()
}

/// Plan the desired placement of already-classified managed toplevels.
pub fn plan(output: Output, windows: &[String]) -> Vec<Placement> {
    match windows {
        [id] => vec![Placement {
            id: id.clone(),
            rect: Rect {
                x: 0,
                y: 0,
                width: output.width,
                height: output.height,
            },
            request: LayoutRequest::Maximize,
        }],
        [left, right] => vec![
            Placement {
                id: left.clone(),
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: output.width / 2,
                    height: output.height,
                },
                request: LayoutRequest::Snap {
                    region: "auto-2-left".to_owned(),
                },
            },
            Placement {
                id: right.clone(),
                rect: Rect {
                    x: output.width / 2,
                    y: 0,
                    width: output.width - (output.width / 2),
                    height: output.height,
                },
                request: LayoutRequest::Snap {
                    region: "auto-2-right".to_owned(),
                },
            },
        ],
        [primary, top, bottom] => {
            let half_width = output.width / 2;
            let half_height = output.height / 2;
            vec![
                Placement {
                    id: primary.clone(),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        width: half_width,
                        height: output.height,
                    },
                    request: LayoutRequest::Snap {
                        region: "auto-3-left".to_owned(),
                    },
                },
                Placement {
                    id: top.clone(),
                    rect: Rect {
                        x: half_width,
                        y: 0,
                        width: output.width - half_width,
                        height: half_height,
                    },
                    request: LayoutRequest::Snap {
                        region: "auto-3-right-top".to_owned(),
                    },
                },
                Placement {
                    id: bottom.clone(),
                    rect: Rect {
                        x: half_width,
                        y: half_height,
                        width: output.width - half_width,
                        height: output.height - half_height,
                    },
                    request: LayoutRequest::Snap {
                        region: "auto-3-right-bottom".to_owned(),
                    },
                },
            ]
        }
        [top_left, top_right, bottom_left, bottom_right] => {
            let half_width = output.width / 2;
            let half_height = output.height / 2;
            vec![
                Placement {
                    id: top_left.clone(),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        width: half_width,
                        height: half_height,
                    },
                    request: LayoutRequest::Snap {
                        region: "auto-4-top-left".to_owned(),
                    },
                },
                Placement {
                    id: top_right.clone(),
                    rect: Rect {
                        x: half_width,
                        y: 0,
                        width: output.width - half_width,
                        height: half_height,
                    },
                    request: LayoutRequest::Snap {
                        region: "auto-4-top-right".to_owned(),
                    },
                },
                Placement {
                    id: bottom_left.clone(),
                    rect: Rect {
                        x: 0,
                        y: half_height,
                        width: half_width,
                        height: output.height - half_height,
                    },
                    request: LayoutRequest::Snap {
                        region: "auto-4-bottom-left".to_owned(),
                    },
                },
                Placement {
                    id: bottom_right.clone(),
                    rect: Rect {
                        x: half_width,
                        y: half_height,
                        width: output.width - half_width,
                        height: output.height - half_height,
                    },
                    request: LayoutRequest::Snap {
                        region: "auto-4-bottom-right".to_owned(),
                    },
                },
            ]
        }
        windows if (5..=6).contains(&windows.len()) => plan_small_grid(output, windows),
        _ => Vec::new(),
    }
}

fn plan_small_grid(output: Output, windows: &[String]) -> Vec<Placement> {
    // ponytail: supports the configured 5/6-window ceiling; add rows/regions only when a profile needs more.
    let mut placements = Vec::with_capacity(windows.len());
    let mut index = 0;
    for row in 0..2 {
        let columns = if windows.len() == 5 && row == 1 { 2 } else { 3 };
        let y = output.height * row / 2;
        let next_y = output.height * (row + 1) / 2;
        for column in 0..columns {
            let x = output.width * column / columns;
            let next_x = output.width * (column + 1) / columns;
            placements.push(Placement {
                id: windows[index].clone(),
                rect: Rect {
                    x,
                    y,
                    width: next_x - x,
                    height: next_y - y,
                },
                request: LayoutRequest::Snap {
                    region: format!("auto-{}-r{row}-c{column}", windows.len()),
                },
            });
            index += 1;
        }
    }
    placements
}
