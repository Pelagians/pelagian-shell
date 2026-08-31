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
[wine]
enabled = false
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
fn status_reports_resolved_profile_without_claiming_a_live_adapter() {
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
[wine]
enabled = false
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
    assert!(stdout.contains("\"compositor_adapter\":\"unavailable\""));
}
