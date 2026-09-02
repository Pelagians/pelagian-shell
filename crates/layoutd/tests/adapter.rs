use std::convert::Infallible;

use pelagian_layoutd::{
    CompositorAdapter, CompositorCommand, LayoutRequest, Output, ToplevelEvent, plan,
    reconcile_commands,
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
            },
            CompositorCommand::Snap {
                toplevel_id: "right".into(),
                region: "auto-2-right".into(),
            },
        ]
    );

    let mut adapter = RecordingAdapter::default();
    adapter.apply_commands(&commands).unwrap();
    assert_eq!(adapter.applied, commands);
    assert_eq!(adapter.observe_toplevel().unwrap(), None);
}
