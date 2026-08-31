use std::fs;
use std::path::PathBuf;

use pelagian_shellctl::{ConfigRoots, resolve};

fn test_root(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("pelagian-shellctl-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn config_roots(root: &std::path::Path, maximum: u8, theme: &str) -> ConfigRoots {
    let share = root.join("share");
    let etc = root.join("etc");
    fs::create_dir_all(share.join("profiles")).unwrap();
    fs::create_dir_all(&etc).unwrap();
    fs::write(
        share.join("defaults.toml"),
        format!(
            r#"
schema_version = 1
[layout]
mode = "auto"
solo = "maximized"
multiple = "automatic"
dialogs = "floating"
max_managed_windows = {maximum}
[decorations]
solo = "none"
tiled = "border"
floating = "full"
[theme]
variant = "{theme}"
[capabilities]
wine = false
"#
        ),
    )
    .unwrap();
    fs::write(share.join("profiles/default.toml"), "schema_version = 1\n").unwrap();
    ConfigRoots { share, etc }
}

#[test]
fn accepts_only_planner_supported_managed_window_counts() {
    for (maximum, accepted) in [(0, false), (1, true), (6, true), (7, false)] {
        let root = test_root(&format!("managed-window-limit-{maximum}"));
        let result = resolve(config_roots(&root, maximum, "dark"), "default");

        if accepted {
            assert_eq!(result.unwrap().config.layout.max_managed_windows, maximum);
        } else {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("layout.max_managed_windows must be between 1 and 6")
            );
        }
    }
}

#[test]
fn rejects_unsupported_light_theme() {
    let root = test_root("light-theme");
    let error = resolve(config_roots(&root, 6, "light"), "default").unwrap_err();

    assert!(error.to_string().contains("defaults.toml"));
    assert!(error.to_string().contains("unknown variant `light`"));
}

#[test]
fn resolves_selected_profile_after_builtin_defaults() {
    let root = test_root("selected-profile");
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
        r#"
schema_version = 1
[layout]
max_managed_windows = 4
[capabilities]
wine = true
"#,
    )
    .unwrap();

    let resolved = resolve(ConfigRoots { share, etc }, "legacy-apps").unwrap();

    assert_eq!(resolved.config.layout.max_managed_windows, 4);
    assert_eq!(resolved.config.layout.mode.as_str(), "auto");
    assert!(resolved.config.capabilities.wine);
}

#[test]
fn applies_drop_ins_in_lexical_order_and_appends_window_rules() {
    let root = test_root("drop-in-order");
    let share = root.join("share");
    let etc = root.join("etc");
    fs::create_dir_all(share.join("profiles")).unwrap();
    fs::create_dir_all(etc.join("profile.d")).unwrap();
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
    fs::write(
        etc.join("profile.d/10-consumer.toml"),
        r#"
schema_version = 1
[layout]
max_managed_windows = 3
[[window_rules]]
app_id = "org.example.First"
disposition = "managed"
"#,
    )
    .unwrap();
    fs::write(
        etc.join("profile.d/80-app.toml"),
        r#"
schema_version = 1
[layout]
max_managed_windows = 5
[[window_rules]]
app_id = "org.example.Second"
disposition = "floating"
"#,
    )
    .unwrap();

    let resolved = resolve(ConfigRoots { share, etc }, "default").unwrap();

    assert_eq!(resolved.config.layout.max_managed_windows, 5);
    assert_eq!(
        resolved
            .config
            .window_rules
            .iter()
            .map(|rule| rule.app_id.as_str())
            .collect::<Vec<_>>(),
        ["org.example.First", "org.example.Second"]
    );
    assert!(resolved.sources[2].ends_with("10-consumer.toml"));
    assert!(resolved.sources[3].ends_with("80-app.toml"));
}

#[test]
fn rejects_unknown_profile_keys_with_the_source_path() {
    let root = test_root("unknown-key");
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
        share.join("profiles/default.toml"),
        "schema_version = 1\n[layout]\nunexpected = true\n",
    )
    .unwrap();

    let error = resolve(ConfigRoots { share, etc }, "default").unwrap_err();

    assert!(error.to_string().contains("profiles/default.toml"));
    assert!(error.to_string().contains("unknown field `unexpected`"));
}
