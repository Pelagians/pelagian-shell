use pelagian_layoutd::{
    Classification, Output, Toplevel, ToplevelEvent, ToplevelKind, WindowRule, Workspace,
};

fn toplevel(id: &str, kind: ToplevelKind, parent_id: Option<&str>) -> Toplevel {
    Toplevel {
        id: id.to_owned(),
        app_id: "org.example.App".to_owned(),
        title: id.to_owned(),
        kind,
        parent_id: parent_id.map(str::to_owned),
    }
}

#[test]
fn dialogs_float_and_do_not_count_as_managed_windows() {
    let mut workspace = Workspace::default();
    workspace.upsert(toplevel("main", ToplevelKind::Normal, None));
    workspace.upsert(toplevel("dialog", ToplevelKind::Dialog, Some("main")));
    workspace.upsert(toplevel("second", ToplevelKind::Normal, None));
    workspace.upsert(toplevel("desktop", ToplevelKind::Desktop, None));

    let classified = workspace.classify(&[]);

    assert_eq!(
        classified.managed,
        vec!["main".to_owned(), "second".to_owned()]
    );
    assert_eq!(classified.floating, vec!["dialog".to_owned()]);
    assert_eq!(classified.ignored, vec!["desktop".to_owned()]);
    assert_eq!(
        workspace.classification_of("dialog", &[]),
        Some(Classification::Floating)
    );
}

#[test]
fn removal_reflows_remaining_managed_windows_in_insertion_order() {
    let mut workspace = Workspace::default();
    workspace.apply(ToplevelEvent::Upsert(toplevel(
        "first",
        ToplevelKind::Normal,
        None,
    )));
    workspace.apply(ToplevelEvent::Upsert(toplevel(
        "second",
        ToplevelKind::Normal,
        None,
    )));
    workspace.apply(ToplevelEvent::Upsert(toplevel(
        "third",
        ToplevelKind::Normal,
        None,
    )));
    workspace.apply(ToplevelEvent::Remove {
        id: "second".to_owned(),
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
        plan.placements
            .iter()
            .map(|placement| placement.id.as_str())
            .collect::<Vec<_>>(),
        ["first", "third"]
    );
    assert_eq!(plan.placements[0].rect.width, 600);
    assert_eq!(plan.placements[1].rect.x, 600);
}

#[test]
fn last_matching_title_rule_can_float_an_installer() {
    let mut workspace = Workspace::default();
    workspace.upsert(Toplevel {
        id: "installer".to_owned(),
        app_id: "wine".to_owned(),
        title: "Example Installer".to_owned(),
        kind: ToplevelKind::Normal,
        parent_id: None,
    });

    assert_eq!(
        workspace.classification_of(
            "installer",
            &[WindowRule {
                app_id: "wine".to_owned(),
                title: Some("*Installer*".to_owned()),
                disposition: Classification::Floating,
            }],
        ),
        Some(Classification::Floating)
    );
}

#[test]
fn six_window_ceiling_keeps_the_planner_in_its_supported_range() {
    let mut workspace = Workspace::default();
    for index in 1..=7 {
        workspace.upsert(toplevel(
            &format!("window-{index}"),
            ToplevelKind::Normal,
            None,
        ));
    }

    let plan = workspace.plan(
        Output {
            width: 1200,
            height: 800,
        },
        &[],
        6,
    );

    assert_eq!(plan.placements.len(), 6);
    assert_eq!(plan.floating, ["window-7"]);
}
