use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn test_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "pelagian-shellctl-cli-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn config_show_prints_the_resolved_toml() {
    let root = test_root("config-show");
    let share = root.join("share");
    let etc = root.join("etc");
    fs::create_dir_all(share.join("profiles")).unwrap();
    fs::create_dir_all(&etc).unwrap();
    fs::write(
        share.join("defaults.toml"),
        r#"
schema_version = 1
[layout]
mode = "auto"
solo = "maximized"
multiple = "automatic"
dialogs = "floating"
max_managed_windows = 6
[decorations]
solo = "none"
tiled = "border"
floating = "full"
[theme]
variant = "dark"
[capabilities]
wine = false
"#,
    )
    .unwrap();
    fs::write(
        share.join("profiles/browser.toml"),
        "schema_version = 1\n[layout]\nmax_managed_windows = 4\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_pelagian-shellctl"))
        .args(["config", "show"])
        .env("PELAGIAN_SHELL_DATA_DIR", &share)
        .env("PELAGIAN_SHELL_ETC_DIR", &etc)
        .env("PELAGIAN_SHELL_PROFILE", "browser")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("max_managed_windows = 4"));
    assert!(stdout.contains("# source: "));
}

#[test]
fn status_reports_resolved_profile_and_live_adapter_capability() {
    let root = test_root("status");
    let share = root.join("share");
    let etc = root.join("etc");
    fs::create_dir_all(share.join("profiles")).unwrap();
    fs::create_dir_all(&etc).unwrap();
    fs::write(
        share.join("defaults.toml"),
        r#"
schema_version = 1
[layout]
mode = "auto"
solo = "maximized"
multiple = "automatic"
dialogs = "floating"
max_managed_windows = 6
[decorations]
solo = "none"
tiled = "border"
floating = "full"
[theme]
variant = "dark"
[capabilities]
wine = false
"#,
    )
    .unwrap();
    fs::write(share.join("profiles/default.toml"), "schema_version = 1\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_pelagian-shellctl"))
        .arg("status")
        .env("PELAGIAN_SHELL_DATA_DIR", &share)
        .env("PELAGIAN_SHELL_ETC_DIR", &etc)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"profile\":\"default\""));
    assert!(stdout.contains("\"capabilities\":{\"wine\":false}"));
    assert!(stdout.contains("\"compositor_adapter\":\"xwayland-ewmh\""));
    assert!(stdout.contains("\"adapter_scope\":\"xwayland\""));
    assert!(stdout.contains("\"native_wayland_control\":false"));
    assert!(stdout.contains("\"layoutd\":\"stopped\""));
}

#[test]
fn capability_reads_the_selected_workload_profile() {
    let root = test_root("capability");
    let share = root.join("share");
    let etc = root.join("etc");
    fs::create_dir_all(share.join("profiles")).unwrap();
    fs::create_dir_all(&etc).unwrap();
    fs::write(
        share.join("defaults.toml"),
        r#"
schema_version = 1
[layout]
mode = "auto"
solo = "maximized"
multiple = "automatic"
dialogs = "floating"
max_managed_windows = 6
[decorations]
solo = "none"
tiled = "border"
floating = "full"
[theme]
variant = "dark"
[capabilities]
wine = false
"#,
    )
    .unwrap();
    fs::write(
        share.join("profiles/legacy-apps.toml"),
        "schema_version = 1\n[capabilities]\nwine = true\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_pelagian-shellctl"))
        .args(["capability", "wine"])
        .env("PELAGIAN_SHELL_DATA_DIR", &share)
        .env("PELAGIAN_SHELL_ETC_DIR", &etc)
        .env("PELAGIAN_SHELL_PROFILE", "legacy-apps")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "true\n");
}
