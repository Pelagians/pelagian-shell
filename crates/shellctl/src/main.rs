use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use pelagian_shellctl::{ConfigError, ConfigRoots, capability_enabled, render_toml, resolve};

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
    let window_chrome_policy = match env::var("PELAGIAN_SHELL_WINDOW_CHROME").as_deref() {
        Ok("server") => "server",
        _ => "unset",
    };
    let layoutd = env::var_os("PELAGIAN_LAYOUTD_BIN").unwrap_or_else(|| "pelagian-layoutd".into());
    let runtime = Command::new(layoutd)
        .arg("status")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|output| output.trim().to_owned())
        .filter(|output| output.starts_with('{') && output.ends_with('}'))
        .unwrap_or_else(|| {
            r#"{"layoutd":"unavailable","adapter_connected":false,"reconciliation":"unknown"}"#
                .to_owned()
        });
    println!(
        "{{\"schema_version\":1,\"profile\":\"{profile}\",\"layout_mode\":\"{}\",\"capabilities\":{{\"wine\":{}}},\"compositor_adapter\":\"labwc-ipc\",\"window_chrome_policy\":\"{window_chrome_policy}\",\"runtime\":{runtime}}}",
        resolved.config.layout.mode.as_str(),
        resolved.config.capabilities.wine,
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
