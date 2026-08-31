use std::collections::BTreeSet;

use pelagian_layoutd::{LayoutRequest, Output, plan};

fn configured_regions() -> BTreeSet<String> {
    include_str!("../../../labwc/rc.xml")
        .lines()
        .filter_map(|line| line.trim().strip_prefix("<region name=\""))
        .filter_map(|line| line.split_once('"').map(|(name, _)| name.to_owned()))
        .collect()
}

#[test]
fn labwc_regions_match_every_region_emitted_by_the_current_planner() {
    let output = Output {
        width: 1200,
        height: 800,
    };
    let emitted = (1..=6)
        .flat_map(|count| {
            let windows = (0..count)
                .map(|index| index.to_string())
                .collect::<Vec<_>>();
            plan(output, &windows)
        })
        .filter_map(|placement| match placement.request {
            LayoutRequest::Maximize => None,
            LayoutRequest::Snap { region } => Some(region),
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(emitted, configured_regions());
}
