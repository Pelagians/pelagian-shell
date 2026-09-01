use std::env;
use std::process::ExitCode;

use pelagian_shellctl::{
    ConfigError, capability_enabled, layoutd_running, profile_from_env, render_toml, resolve,
    roots_from_env,
};

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
        "{{\"schema_version\":1,\"profile\":\"{profile}\",\"layout_mode\":\"{}\",\"capabilities\":{{\"wine\":{}}},\"compositor_adapter\":\"xwayland-ewmh\",\"adapter_scope\":\"xwayland\",\"native_wayland_control\":false,\"layoutd\":\"{}\"}}",
        resolved.config.layout.mode.as_str(),
        resolved.config.capabilities.wine,
        if layoutd_running() {
            "running"
        } else {
            "stopped"
        },
    );
    Ok(())
}

fn show_capability(capability: &str) -> Result<(), ConfigError> {
    let resolved = resolve(roots_from_env(), &profile_from_env())?;
    println!("{}", capability_enabled(&resolved, capability)?);
    Ok(())
}

fn main() -> ExitCode {
    let command = env::args().skip(1).collect::<Vec<_>>();
    let result = match command.as_slice() {
        [command, action] if command == "config" && action == "show" => show_config(),
        [command] if command == "status" => show_status(),
        [command, capability] if command == "capability" => show_capability(capability),
        _ => {
            eprintln!("usage: pelagian-shellctl config show | status | capability <name>");
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
