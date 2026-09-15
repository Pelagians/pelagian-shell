use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use pelagian_layoutd::{
    CompositorAdapter, LabwcIpcAdapter, RuntimeSettings, Workspace, reconcile_float_mode_commands,
    reconcile_workspace_commands, runtime_status_json, write_runtime_state,
};
use pelagian_shellctl::{ConfigRoots, resolve};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let roots = ConfigRoots {
        share: env::var_os("PELAGIAN_SHELL_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr/share/pelagian-shell")),
        etc: env::var_os("PELAGIAN_SHELL_ETC_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/etc/pelagian-shell")),
    };
    let profile = env::var("PELAGIAN_SHELL_PROFILE").unwrap_or_else(|_| "default".to_owned());
    let settings = RuntimeSettings::from_config(&resolve(roots, &profile)?.config);
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "XDG_RUNTIME_DIR is required for the session-private Labwc IPC socket",
        )
    })?;
    let mut adapter = LabwcIpcAdapter::new(PathBuf::from(runtime_dir).join("labwc.sock"));
    let mut workspace = Workspace::default();
    let mode = if settings.automatic { "auto" } else { "float" };

    write_runtime_state(mode, "starting", false, "pending", 0, 0, None)?;
    let mut retry_reconciliation = false;
    let mut adapter_ready = false;
    let mut managed_windows = 0;
    let mut floating_windows = 0;
    loop {
        let mut changed = false;
        let mut observation_failed = false;
        loop {
            match adapter.observe_toplevel() {
                Ok(Some(event)) => {
                    workspace.apply(event);
                    changed = true;
                }
                Ok(None) => break,
                Err(error) => {
                    eprintln!("pelagian-layoutd: observation failed: {error}");
                    write_runtime_state(
                        mode,
                        "degraded",
                        error.adapter_connected(),
                        "error",
                        managed_windows,
                        floating_windows,
                        Some(&error.to_string()),
                    )?;
                    adapter_ready = false;
                    observation_failed = true;
                    break;
                }
            }
        }
        if observation_failed {
            retry_reconciliation = true;
            thread::sleep(Duration::from_millis(250));
            continue;
        }
        if !adapter_ready || changed || retry_reconciliation {
            let classified = workspace.classify(&settings.window_rules);
            if settings.automatic {
                let output = match adapter.output() {
                    Ok(output) => output,
                    Err(error) => {
                        eprintln!("pelagian-layoutd: reconciliation failed: {error}");
                        write_runtime_state(
                            mode,
                            "degraded",
                            false,
                            "error",
                            managed_windows,
                            floating_windows,
                            Some(&error.to_string()),
                        )?;
                        retry_reconciliation = true;
                        thread::sleep(Duration::from_millis(250));
                        continue;
                    }
                };
                let plan =
                    workspace.plan(output, &settings.window_rules, settings.max_managed_windows);
                if let Err(error) = adapter.apply_commands(&reconcile_workspace_commands(&plan)) {
                    eprintln!("pelagian-layoutd: reconciliation failed: {error}");
                    write_runtime_state(
                        mode,
                        "degraded",
                        error.adapter_connected(),
                        "error",
                        managed_windows,
                        floating_windows,
                        Some(&error.to_string()),
                    )?;
                    retry_reconciliation = true;
                    thread::sleep(Duration::from_millis(250));
                    continue;
                }
                managed_windows = plan.placements.len();
                floating_windows = plan.floating.len();
            } else {
                managed_windows = classified.managed.len();
                floating_windows = classified.floating.len();
                if let Err(error) =
                    adapter.apply_commands(&reconcile_float_mode_commands(&classified))
                {
                    eprintln!("pelagian-layoutd: reconciliation failed: {error}");
                    write_runtime_state(
                        mode,
                        "degraded",
                        error.adapter_connected(),
                        "error",
                        managed_windows,
                        floating_windows,
                        Some(&error.to_string()),
                    )?;
                    retry_reconciliation = true;
                    thread::sleep(Duration::from_millis(250));
                    continue;
                }
            }
            retry_reconciliation = false;
            adapter_ready = true;
            write_runtime_state(
                mode,
                "healthy",
                true,
                "healthy",
                managed_windows,
                floating_windows,
                None,
            )?;
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn main() -> ExitCode {
    match env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [command] if command == "status" => {
            println!("{}", runtime_status_json());
            ExitCode::SUCCESS
        }
        [] => match run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("pelagian-layoutd: {error}");
                ExitCode::from(1)
            }
        },
        _ => {
            eprintln!("usage: pelagian-layoutd [status]");
            ExitCode::from(2)
        }
    }
}
