use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use pelagian_layoutd::{
    CompositorAdapter, CompositorCommand, Rect, RuntimeSettings, ToplevelEvent, XwaylandEwmhAdapter,
};
use pelagian_shellctl::{ConfigRoots, resolve};

fn script(root: &Path, name: &str, body: &str) -> PathBuf {
    let path = root.join(name);
    let temporary = root.join(format!("{name}.tmp"));
    fs::write(&temporary, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    let mut permissions = fs::metadata(&temporary).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&temporary, permissions).unwrap();
    fs::rename(temporary, &path).unwrap();
    path
}

fn fixture_root(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "pelagian-layoutd-{name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn fixture_adapter(name: &str, geometry: &str, state: &str) -> (XwaylandEwmhAdapter, PathBuf) {
    let root = fixture_root(name);
    let log = root.join("mutations.log");
    let wmctrl = script(
        &root,
        "wmctrl",
        &format!(
            r#"
if [ "$*" = "-root" ]; then
  printf '%s\n' '  Width: 1200' '  Height: 800'
elif [ "$*" = "-lpGx" ]; then
  printf '%s\n' '0x01200001  0 4242 {geometry} fixture-host fixture.Fixture Fixture Primary'
else
  printf '%s\n' "$*" >> '{}'
fi
"#,
            log.display()
        ),
    );
    let xprop = script(
        &root,
        "xprop",
        &format!(
            r#"
printf '%s\n' 'WM_CLASS(STRING) = "fixture", "Fixture"'
printf '%s\n' '_NET_WM_WINDOW_TYPE(ATOM) = _NET_WM_WINDOW_TYPE_NORMAL'
printf '%s\n' 'WM_TRANSIENT_FOR:  not found.'
printf '%s\n' '_NET_WM_STATE(ATOM) = {state}'
"#
        ),
    );
    (XwaylandEwmhAdapter::with_commands(wmctrl, xprop), log)
}

#[test]
fn adapter_observes_generic_xwayland_identity_type_parent_and_output() {
    let (mut adapter, _) = fixture_adapter(
        "observe",
        "0 0 1200 800",
        "_NET_WM_STATE_MAXIMIZED_VERT, _NET_WM_STATE_MAXIMIZED_HORZ",
    );

    assert_eq!(adapter.output().unwrap().width, 1200);
    let event = adapter.observe_toplevel().unwrap().unwrap();
    let ToplevelEvent::Upsert(toplevel) = event else {
        panic!("expected an upsert");
    };
    assert_eq!(toplevel.id, "0x01200001");
    assert_eq!(toplevel.app_id, "fixture.Fixture");
    assert_eq!(toplevel.title, "Fixture Primary");
    assert_eq!(toplevel.parent_id, None);
}

#[test]
fn output_uses_x_root_geometry_without_ewmh_desktops() {
    let root = fixture_root("root-output");
    let xwininfo = script(
        &root,
        "xwininfo",
        r#"
if [ "$*" = "-root" ]; then
  printf '%s\n' '  Width: 1200' '  Height: 800'
else
  exit 1
fi
"#,
    );
    let xprop = script(&root, "xprop", "exit 0");
    let adapter = XwaylandEwmhAdapter::with_commands(xwininfo, xprop);

    assert_eq!(
        adapter.output().unwrap(),
        pelagian_layoutd::Output {
            width: 1200,
            height: 800,
        }
    );
}

#[test]
fn malformed_nonempty_wmctrl_inventory_fails_closed() {
    let root = fixture_root("malformed-inventory");
    let inventory = root.join("inventory");
    fs::write(&inventory, "malformed nonempty inventory row\n").unwrap();
    let wmctrl = script(
        &root,
        "wmctrl",
        &format!(
            "if [ \"$*\" = \"-d\" ]; then printf '%s\\n' '0 * DG: 1200x800'; else cat '{}'; fi",
            inventory.display()
        ),
    );
    let xprop = script(&root, "xprop", "exit 0");
    let mut adapter = XwaylandEwmhAdapter::with_commands(wmctrl, xprop);

    assert!(adapter.observe_toplevel().is_err());
}

#[test]
fn missing_ewmh_client_list_is_an_empty_inventory() {
    let root = fixture_root("empty-inventory");
    let wmctrl = script(
        &root,
        "wmctrl",
        "printf '%s\n' 'Cannot get client list properties.' '(_NET_CLIENT_LIST or _WIN_CLIENT_LIST)' >&2\nexit 1",
    );
    let xprop = script(&root, "xprop", "exit 0");
    let mut adapter = XwaylandEwmhAdapter::with_commands(wmctrl, xprop);

    assert_eq!(adapter.observe_toplevel().unwrap(), None);
}

#[test]
fn snap_is_idempotent_when_geometry_is_already_correct() {
    let (mut correct, correct_log) = fixture_adapter("snap-correct", "0 0 600 800", "");
    correct
        .apply_commands(&[CompositorCommand::Snap {
            toplevel_id: "0x01200001".into(),
            region: "auto-2-left".into(),
            rect: Rect {
                x: 0,
                y: 0,
                width: 600,
                height: 800,
            },
        }])
        .unwrap();
    assert!(!correct_log.exists());
}

#[test]
fn snap_forces_northwest_gravity_for_absolute_placement() {
    let (mut adapter, log) = fixture_adapter("snap-gravity", "40 40 320 200", "");
    adapter
        .apply_commands(&[CompositorCommand::Snap {
            toplevel_id: "0x01200001".into(),
            region: "auto-2-right".into(),
            rect: Rect {
                x: 600,
                y: 0,
                width: 600,
                height: 800,
            },
        }])
        .unwrap();

    assert!(
        fs::read_to_string(log)
            .unwrap()
            .contains("-ir 0x01200001 -e 1,600,0,600,800")
    );
}

#[test]
fn snap_preserves_the_restore_geometry_of_an_initially_maximized_window() {
    let root = fixture_root("snap-initially-maximized");
    let inventory = root.join("inventory");
    let unmaximized = root.join("unmaximized");
    let log = root.join("mutations");
    fs::write(
        &inventory,
        "0x01200001 0 4242 0 0 1200 800 host fixture.Fixture Fixture\n",
    )
    .unwrap();
    let wmctrl = script(
        &root,
        "wmctrl",
        &format!(
            r#"
if [ "$*" = "-lpGx" ]; then
  cat '{}'
elif [ "$*" = "-d" ]; then
  printf '%s\n' '0 * DG: 1200x800'
else
  printf '%s\n' "$*" >> '{}'
  case "$*" in
    *remove,maximized_vert,maximized_horz*)
      printf '%s\n' '0x01200001 0 4242 100 100 320 200 host fixture.Fixture Fixture' > '{}'
      : > '{}'
      ;;
  esac
fi
"#,
            inventory.display(),
            log.display(),
            inventory.display(),
            unmaximized.display()
        ),
    );
    let xprop = script(
        &root,
        "xprop",
        &format!(
            r#"
printf '%s\n' 'WM_CLASS(STRING) = "fixture", "Fixture"'
printf '%s\n' '_NET_WM_WINDOW_TYPE(ATOM) = _NET_WM_WINDOW_TYPE_NORMAL'
printf '%s\n' 'WM_TRANSIENT_FOR: not found.'
if [ -f '{}' ]; then
  printf '%s\n' '_NET_WM_STATE(ATOM) ='
else
  printf '%s\n' '_NET_WM_STATE(ATOM) = _NET_WM_STATE_MAXIMIZED_VERT, _NET_WM_STATE_MAXIMIZED_HORZ'
fi
"#,
            unmaximized.display()
        ),
    );
    let mut adapter = XwaylandEwmhAdapter::with_commands(wmctrl, xprop);
    adapter
        .apply_commands(&[CompositorCommand::Snap {
            toplevel_id: "0x01200001".into(),
            region: "auto-2-left".into(),
            rect: Rect {
                x: 0,
                y: 0,
                width: 600,
                height: 800,
            },
        }])
        .unwrap();
    fs::write(
        &inventory,
        "0x01200001 0 4242 0 0 600 800 host fixture.Fixture Fixture\n",
    )
    .unwrap();

    adapter
        .apply_commands(&[CompositorCommand::Unsnap {
            toplevel_id: "0x01200001".into(),
        }])
        .unwrap();

    assert!(
        fs::read_to_string(log)
            .unwrap()
            .contains("-ir 0x01200001 -e 1,100,100,320,200")
    );
}

#[test]
fn unsnap_restores_the_geometry_captured_before_management() {
    let root = fixture_root("unsnap-restore");
    let inventory = root.join("inventory");
    let log = root.join("mutations");
    fs::write(
        &inventory,
        "0x01200001 0 4242 100 100 320 200 host fixture.Fixture Fixture\n",
    )
    .unwrap();
    let wmctrl = script(
        &root,
        "wmctrl",
        &format!(
            "if [ \"$*\" = \"-lpGx\" ]; then cat '{}'; elif [ \"$*\" = \"-d\" ]; then printf '%s\\n' '0 * DG: 1200x800'; else printf '%s\\n' \"$*\" >> '{}'; fi",
            inventory.display(),
            log.display()
        ),
    );
    let xprop = script(
        &root,
        "xprop",
        "printf '%s\n' 'WM_CLASS(STRING) = \"fixture\", \"Fixture\"' '_NET_WM_WINDOW_TYPE(ATOM) = _NET_WM_WINDOW_TYPE_NORMAL' 'WM_TRANSIENT_FOR: not found.' '_NET_WM_STATE(ATOM) ='",
    );
    let mut adapter = XwaylandEwmhAdapter::with_commands(wmctrl, xprop);
    adapter
        .apply_commands(&[CompositorCommand::Snap {
            toplevel_id: "0x01200001".into(),
            region: "auto-2-left".into(),
            rect: Rect {
                x: 0,
                y: 0,
                width: 600,
                height: 800,
            },
        }])
        .unwrap();
    fs::write(
        &inventory,
        "0x01200001 0 4242 0 0 600 800 host fixture.Fixture Fixture\n",
    )
    .unwrap();

    adapter
        .apply_commands(&[CompositorCommand::Unsnap {
            toplevel_id: "0x01200001".into(),
        }])
        .unwrap();

    assert!(
        fs::read_to_string(log)
            .unwrap()
            .contains("-ir 0x01200001 -e 1,100,100,320,200")
    );
}

#[test]
fn maximize_recovers_a_tiled_xid_and_is_idempotent_once_maximized() {
    let (mut tiled, tiled_log) = fixture_adapter("maximize-tiled", "0 0 600 800", "");
    tiled
        .apply_commands(&[CompositorCommand::Maximize {
            toplevel_id: "0x01200001".into(),
        }])
        .unwrap();
    assert!(
        fs::read_to_string(tiled_log)
            .unwrap()
            .contains("-ir 0x01200001 -b add,maximized_vert,maximized_horz")
    );

    let (mut maximized, maximized_log) = fixture_adapter(
        "maximize-correct",
        "0 0 1200 800",
        "_NET_WM_STATE_MAXIMIZED_VERT, _NET_WM_STATE_MAXIMIZED_HORZ",
    );
    maximized
        .apply_commands(&[CompositorCommand::Maximize {
            toplevel_id: "0x01200001".into(),
        }])
        .unwrap();
    assert!(!maximized_log.exists());
}

#[test]
fn runtime_uses_shells_resolved_configuration_without_parallel_defaults() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let resolved = resolve(
        ConfigRoots {
            share: repository.join("config"),
            etc: fixture_root("config"),
        },
        "browser",
    )
    .unwrap();

    let settings = RuntimeSettings::from_shell_config(&resolved.config);
    assert!(settings.automatic);
    assert_eq!(settings.max_managed_windows, 6);
    assert!(settings.window_rules.is_empty());
}

#[test]
fn resolved_float_drop_in_disables_automatic_placement() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let etc = fixture_root("float-config");
    fs::create_dir_all(etc.join("profile.d")).unwrap();
    fs::write(
        etc.join("profile.d/90-float.toml"),
        "schema_version = 1\n[layout]\nmode = \"float\"\n",
    )
    .unwrap();
    let resolved = resolve(
        ConfigRoots {
            share: repository.join("config"),
            etc,
        },
        "default",
    )
    .unwrap();

    assert!(!RuntimeSettings::from_shell_config(&resolved.config).automatic);
}
