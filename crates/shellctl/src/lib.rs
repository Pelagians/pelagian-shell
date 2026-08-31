//! Declarative Pelagian Shell configuration resolution.

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigRoots {
    pub share: PathBuf,
    pub etc: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub config: Config,
    pub sources: Vec<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub layout: Layout,
    pub decorations: Decorations,
    pub theme: Theme,
    pub capabilities: Capabilities,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub window_rules: Vec<WindowRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub mode: LayoutMode,
    pub solo: SoloMode,
    pub multiple: MultipleMode,
    pub dialogs: DialogMode,
    pub max_managed_windows: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayoutMode {
    Auto,
    Float,
}

impl LayoutMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Float => "float",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoloMode {
    Maximized,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultipleMode {
    Automatic,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DialogMode {
    Floating,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Decorations {
    pub solo: DecorationMode,
    pub tiled: DecorationMode,
    pub floating: DecorationMode,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecorationMode {
    None,
    Border,
    Full,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub variant: ThemeVariant,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemeVariant {
    Dark,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub wine: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WindowRule {
    pub app_id: String,
    #[serde(default)]
    pub title: Option<String>,
    pub disposition: WindowDisposition,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WindowDisposition {
    Managed,
    Floating,
    Ignored,
}

#[derive(Debug)]
pub struct ConfigError(String);

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ConfigError {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigPatch {
    schema_version: u32,
    #[serde(default)]
    layout: Option<LayoutPatch>,
    #[serde(default)]
    decorations: Option<DecorationsPatch>,
    #[serde(default)]
    theme: Option<ThemePatch>,
    #[serde(default)]
    capabilities: Option<CapabilitiesPatch>,
    #[serde(default)]
    window_rules: Vec<WindowRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutPatch {
    #[serde(default)]
    mode: Option<LayoutMode>,
    #[serde(default)]
    solo: Option<SoloMode>,
    #[serde(default)]
    multiple: Option<MultipleMode>,
    #[serde(default)]
    dialogs: Option<DialogMode>,
    #[serde(default)]
    max_managed_windows: Option<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DecorationsPatch {
    #[serde(default)]
    solo: Option<DecorationMode>,
    #[serde(default)]
    tiled: Option<DecorationMode>,
    #[serde(default)]
    floating: Option<DecorationMode>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemePatch {
    #[serde(default)]
    variant: Option<ThemeVariant>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilitiesPatch {
    #[serde(default)]
    wine: Option<bool>,
}

/// Resolve built-in defaults, one selected profile, then lexical local drop-ins.
pub fn resolve(roots: ConfigRoots, profile: &str) -> Result<ResolvedConfig, ConfigError> {
    validate_profile_name(profile)?;

    let defaults_path = roots.share.join("defaults.toml");
    let mut config = parse_defaults(&defaults_path)?;
    let mut sources = vec![defaults_path];

    let local_profile = roots.etc.join("profiles").join(format!("{profile}.toml"));
    let builtin_profile = roots.share.join("profiles").join(format!("{profile}.toml"));
    let profile_path = if local_profile.is_file() {
        local_profile
    } else if builtin_profile.is_file() {
        builtin_profile
    } else {
        return Err(ConfigError(format!(
            "selected profile {profile:?} was not found in {} or {}",
            roots.etc.join("profiles").display(),
            roots.share.join("profiles").display()
        )));
    };
    apply_patch(&mut config, parse_patch(&profile_path)?);
    sources.push(profile_path);

    let drop_in_dir = roots.etc.join("profile.d");
    if drop_in_dir.is_dir() {
        let entries = fs::read_dir(&drop_in_dir).map_err(|error| {
            ConfigError(format!("cannot read {}: {error}", drop_in_dir.display()))
        })?;
        let mut paths = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|error| {
                    ConfigError(format!(
                        "cannot enumerate drop-ins in {}: {error}",
                        drop_in_dir.display()
                    ))
                })?
                .path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == "toml")
            {
                paths.push(path);
            }
        }
        paths.sort();
        for path in paths {
            apply_patch(&mut config, parse_patch(&path)?);
            sources.push(path);
        }
    }

    validate_config(&config)?;
    Ok(ResolvedConfig { config, sources })
}

pub fn render_toml(resolved: &ResolvedConfig) -> Result<String, ConfigError> {
    toml::to_string_pretty(&resolved.config)
        .map_err(|error| ConfigError(format!("cannot render effective configuration: {error}")))
}

fn validate_profile_name(profile: &str) -> Result<(), ConfigError> {
    if profile.is_empty()
        || !profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ConfigError(format!(
            "invalid profile name {profile:?}; use only ASCII letters, digits, '-' or '_'"
        )));
    }
    Ok(())
}

fn parse_defaults(path: &Path) -> Result<Config, ConfigError> {
    let config = parse_file(path)?;
    let config = toml::from_str::<Config>(&config)
        .map_err(|error| ConfigError(format!("{}: {error}", path.display())))?;
    validate_config(&config)?;
    Ok(config)
}

fn parse_patch(path: &Path) -> Result<ConfigPatch, ConfigError> {
    let patch = parse_file(path)?;
    let patch = toml::from_str::<ConfigPatch>(&patch)
        .map_err(|error| ConfigError(format!("{}: {error}", path.display())))?;
    if patch.schema_version != SCHEMA_VERSION {
        return Err(ConfigError(format!(
            "{}: unsupported schema_version {}; expected {}",
            path.display(),
            patch.schema_version,
            SCHEMA_VERSION
        )));
    }
    for rule in &patch.window_rules {
        validate_rule(rule)?;
    }
    Ok(patch)
}

fn parse_file(path: &Path) -> Result<String, ConfigError> {
    fs::read_to_string(path)
        .map_err(|error| ConfigError(format!("cannot read {}: {error}", path.display())))
}

fn apply_patch(config: &mut Config, patch: ConfigPatch) {
    if let Some(layout) = patch.layout {
        if let Some(mode) = layout.mode {
            config.layout.mode = mode;
        }
        if let Some(solo) = layout.solo {
            config.layout.solo = solo;
        }
        if let Some(multiple) = layout.multiple {
            config.layout.multiple = multiple;
        }
        if let Some(dialogs) = layout.dialogs {
            config.layout.dialogs = dialogs;
        }
        if let Some(maximum) = layout.max_managed_windows {
            config.layout.max_managed_windows = maximum;
        }
    }
    if let Some(decorations) = patch.decorations {
        if let Some(solo) = decorations.solo {
            config.decorations.solo = solo;
        }
        if let Some(tiled) = decorations.tiled {
            config.decorations.tiled = tiled;
        }
        if let Some(floating) = decorations.floating {
            config.decorations.floating = floating;
        }
    }
    if let Some(theme) = patch.theme {
        if let Some(variant) = theme.variant {
            config.theme.variant = variant;
        }
    }
    if let Some(capabilities) = patch.capabilities {
        if let Some(wine) = capabilities.wine {
            config.capabilities.wine = wine;
        }
    }
    config.window_rules.extend(patch.window_rules);
}

fn validate_config(config: &Config) -> Result<(), ConfigError> {
    if config.schema_version != SCHEMA_VERSION {
        return Err(ConfigError(format!(
            "unsupported schema_version {}; expected {}",
            config.schema_version, SCHEMA_VERSION
        )));
    }
    if !(1..=6).contains(&config.layout.max_managed_windows) {
        return Err(ConfigError(
            "layout.max_managed_windows must be between 1 and 6".to_owned(),
        ));
    }
    for rule in &config.window_rules {
        validate_rule(rule)?;
    }
    Ok(())
}

fn validate_rule(rule: &WindowRule) -> Result<(), ConfigError> {
    if rule.app_id.trim().is_empty() {
        return Err(ConfigError(
            "window_rules.app_id must not be empty".to_owned(),
        ));
    }
    if rule.title.as_deref().is_some_and(|title| title.is_empty()) {
        return Err(ConfigError(
            "window_rules.title must not be empty when provided".to_owned(),
        ));
    }
    Ok(())
}

/// Return one explicit optional capability from an already resolved config.
pub fn capability_enabled(
    resolved: &ResolvedConfig,
    capability: &str,
) -> Result<bool, ConfigError> {
    match capability {
        "wine" => Ok(resolved.config.capabilities.wine),
        _ => Err(ConfigError(format!("unknown capability {capability:?}"))),
    }
}
