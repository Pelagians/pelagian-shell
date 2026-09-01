use std::convert::Infallible;

use pelagian_layoutd::{
    CompositorAdapter, CompositorCommand, LayoutRequest, Output, Rect, ToplevelEvent, plan,
    reconcile_commands, transition_commands,
};

#[derive(Default)]
struct RecordingAdapter {
    applied: Vec<CompositorCommand>,
}

impl CompositorAdapter for RecordingAdapter {
    type Error = Infallible;

    fn observe_toplevel(&mut self) -> Result<Option<ToplevelEvent>, Self::Error> {
        Ok(None)
    }

    fn apply_commands(&mut self, commands: &[CompositorCommand]) -> Result<(), Self::Error> {
        self.applied.extend_from_slice(commands);
        Ok(())
    }
}

#[test]
fn planner_requests_are_translated_to_small_explicit_compositor_commands() {
    let placements = plan(
        Output {
            width: 1200,
            height: 800,
        },
        &["left".into(), "right".into()],
    );
    assert!(matches!(
        placements[0].request,
        LayoutRequest::Snap { ref region } if region == "auto-2-left"
    ));

    let commands = reconcile_commands(&placements);
    assert_eq!(
        commands,
        vec![
            CompositorCommand::Snap {
                toplevel_id: "left".into(),
                region: "auto-2-left".into(),
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 600,
                    height: 800,
                },
            },
            CompositorCommand::Snap {
                toplevel_id: "right".into(),
                region: "auto-2-right".into(),
                rect: Rect {
                    x: 600,
                    y: 0,
                    width: 600,
                    height: 800,
                },
            },
        ]
    );

    let mut adapter = RecordingAdapter::default();
    adapter.apply_commands(&commands).unwrap();
    assert_eq!(adapter.applied, commands);
    assert_eq!(adapter.observe_toplevel().unwrap(), None);
}

#[test]
fn a_previously_managed_window_is_unsnapped_when_it_becomes_floating() {
    let mut workspace = pelagian_layoutd::Workspace::default();
    workspace.upsert(pelagian_layoutd::Toplevel {
        id: "window".into(),
        app_id: "fixture".into(),
        title: "Window".into(),
        kind: pelagian_layoutd::ToplevelKind::Dialog,
        parent_id: None,
    });
    let plan = workspace.plan(
        Output {
            width: 1200,
            height: 800,
        },
        &[],
        6,
    );

    assert_eq!(
        transition_commands(&["window".into()], &plan),
        vec![CompositorCommand::Unsnap {
            toplevel_id: "window".into(),
        }]
    );
}
