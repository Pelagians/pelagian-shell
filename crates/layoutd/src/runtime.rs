use pelagian_shellctl::{
    Config, DecorationMode, LayoutMode, WindowDisposition, WindowRule as ShellWindowRule,
};

use crate::{Classification, DecorationState, WindowRule};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeSettings {
    pub automatic: bool,
    pub max_managed_windows: usize,
    pub solo_decoration: DecorationState,
    pub tiled_decoration: DecorationState,
    pub floating_decoration: DecorationState,
    pub window_rules: Vec<WindowRule>,
}

impl RuntimeSettings {
    pub fn from_shell_config(config: &Config) -> Self {
        Self {
            automatic: matches!(config.layout.mode, LayoutMode::Auto),
            max_managed_windows: usize::from(config.layout.max_managed_windows),
            solo_decoration: decoration(config.decorations.solo.clone()),
            tiled_decoration: decoration(config.decorations.tiled.clone()),
            floating_decoration: decoration(config.decorations.floating.clone()),
            window_rules: config.window_rules.iter().map(window_rule).collect(),
        }
    }
}

fn decoration(mode: DecorationMode) -> DecorationState {
    match mode {
        DecorationMode::None => DecorationState::None,
        DecorationMode::Border => DecorationState::Border,
        DecorationMode::Full => DecorationState::Full,
    }
}

fn window_rule(rule: &ShellWindowRule) -> WindowRule {
    WindowRule {
        app_id: rule.app_id.clone(),
        title: rule.title.clone(),
        disposition: match rule.disposition {
            WindowDisposition::Managed => Classification::Managed,
            WindowDisposition::Floating => Classification::Floating,
            WindowDisposition::Ignored => Classification::Ignored,
        },
    }
}
