use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use pelagian_shellctl::{ConfigError, ConfigRoots, render_toml, resolve};

fn roots_from_env() -> ConfigRoots {
    ConfigRoots {
        share: env::var_os("PELAGIAN_SHELL_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr/share/pelagian-shell")),
        etc: env::var_os("PELAGIAN_SHELL_ETC_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/etc/pelagian-shell")),
    }
}

fn profile_from_env() -> String {
    env::var("PELAGIAN_SHELL_PROFILE").unwrap_or_else(|_| "default".to_owned())
}

fn show_config() -> Result<(), ConfigError> {
    let resolved = resolve(roots_from_env(), &profile_from_env())?;
    for source in &resolved.sources {
        println!("# source: {}", source.display());
    }
    print!("{}", render_toml(&resolved)?);
    Ok(())
}

fn show_status() -> Result<(), ConfigError> {
    let profile = profile_from_env();
    let resolved = resolve(roots_from_env(), &profile)?;
    println!(
        "{{\"schema_version\":1,\"profile\":\"{profile}\",\"layout_mode\":\"{}\",\"wine_enabled\":{},\"compositor_adapter\":\"unavailable\",\"layoutd\":\"planner_only\"}}",
        resolved.config.layout.mode.as_str(),
        resolved.config.wine.enabled,
    );
    Ok(())
}

fn main() -> ExitCode {
    let command = env::args().skip(1).collect::<Vec<_>>();
    let result = match command.as_slice() {
        [command, action] if command == "config" && action == "show" => show_config(),
        [command] if command == "status" => show_status(),
        _ => {
            eprintln!("usage: pelagian-shellctl config show | status");
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pelagian-shellctl: {error}");
            ExitCode::from(1)
        }
    }
}
