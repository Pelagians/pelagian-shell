use pelagian_layoutd::{LayoutRequest, Output, Rect, plan};

#[test]
fn one_normal_window_maximizes_without_requesting_fullscreen() {
    let placements = plan(
        Output {
            width: 1200,
            height: 800,
        },
        &["primary".to_owned()],
    );

    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].id, "primary");
    assert_eq!(
        placements[0].rect,
        Rect {
            x: 0,
            y: 0,
            width: 1200,
            height: 800,
        }
    );
    assert_eq!(placements[0].request, LayoutRequest::Maximize);
}

#[test]
fn two_windows_split_the_output_in_equal_halves() {
    let placements = plan(
        Output {
            width: 1200,
            height: 800,
        },
        &["left".to_owned(), "right".to_owned()],
    );

    assert_eq!(
        placements,
        [
            pelagian_layoutd::Placement {
                id: "left".to_owned(),
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 600,
                    height: 800,
                },
                request: LayoutRequest::Snap {
                    region: "auto-2-left".to_owned(),
                },
            },
            pelagian_layoutd::Placement {
                id: "right".to_owned(),
                rect: Rect {
                    x: 600,
                    y: 0,
                    width: 600,
                    height: 800,
                },
                request: LayoutRequest::Snap {
                    region: "auto-2-right".to_owned(),
                },
            },
        ]
    );
}

#[test]
fn three_windows_keep_a_primary_left_surface_and_stack_the_right_side() {
    let placements = plan(
        Output {
            width: 1200,
            height: 800,
        },
        &["primary".to_owned(), "top".to_owned(), "bottom".to_owned()],
    );

    assert_eq!(placements.len(), 3);
    assert_eq!(
        placements[0].rect,
        Rect {
            x: 0,
            y: 0,
            width: 600,
            height: 800
        }
    );
    assert_eq!(
        placements[1].rect,
        Rect {
            x: 600,
            y: 0,
            width: 600,
            height: 400
        }
    );
    assert_eq!(
        placements[2].rect,
        Rect {
            x: 600,
            y: 400,
            width: 600,
            height: 400,
        }
    );
    assert_eq!(
        placements[1].request,
        LayoutRequest::Snap {
            region: "auto-3-right-top".to_owned(),
        }
    );
}

#[test]
fn four_windows_use_a_two_by_two_grid() {
    let placements = plan(
        Output {
            width: 1200,
            height: 800,
        },
        &[
            "top-left".to_owned(),
            "top-right".to_owned(),
            "bottom-left".to_owned(),
            "bottom-right".to_owned(),
        ],
    );

    assert_eq!(placements.len(), 4);
    assert_eq!(
        placements[0].rect,
        Rect {
            x: 0,
            y: 0,
            width: 600,
            height: 400,
        }
    );
    assert_eq!(
        placements[1].rect,
        Rect {
            x: 600,
            y: 0,
            width: 600,
            height: 400,
        }
    );
    assert_eq!(
        placements[2].rect,
        Rect {
            x: 0,
            y: 400,
            width: 600,
            height: 400,
        }
    );
    assert_eq!(
        placements[3].request,
        LayoutRequest::Snap {
            region: "auto-4-bottom-right".to_owned(),
        }
    );
}

#[test]
fn five_windows_fill_balanced_rows_without_gaps() {
    let placements = plan(
        Output {
            width: 1200,
            height: 800,
        },
        &[
            "one".to_owned(),
            "two".to_owned(),
            "three".to_owned(),
            "four".to_owned(),
            "five".to_owned(),
        ],
    );

    assert_eq!(placements.len(), 5);
    assert_eq!(
        placements[0].rect,
        Rect {
            x: 0,
            y: 0,
            width: 400,
            height: 400
        }
    );
    assert_eq!(
        placements[2].rect,
        Rect {
            x: 800,
            y: 0,
            width: 400,
            height: 400
        }
    );
    assert_eq!(
        placements[3].rect,
        Rect {
            x: 0,
            y: 400,
            width: 600,
            height: 400
        }
    );
    assert_eq!(
        placements[4].rect,
        Rect {
            x: 600,
            y: 400,
            width: 600,
            height: 400
        }
    );
    assert_eq!(
        placements[4].request,
        LayoutRequest::Snap {
            region: "auto-5-r1-c1".to_owned(),
        }
    );
}
