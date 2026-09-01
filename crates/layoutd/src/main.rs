use std::env;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use pelagian_layoutd::{
    CompositorAdapter, RuntimeSettings, Workspace, XwaylandEwmhAdapter, runtime_status_json,
    transition_commands, write_runtime_state,
};
use pelagian_shellctl::{layoutd_running, profile_from_env, resolve, roots_from_env};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    if layoutd_running() {
        return Ok(());
    }
    let resolved = resolve(roots_from_env(), &profile_from_env())?;
    let settings = RuntimeSettings::from_shell_config(&resolved.config);
    let mut adapter = XwaylandEwmhAdapter::new();
    let mut workspace = Workspace::default();
    let mode = if settings.automatic { "auto" } else { "float" };
    let mut last_counts = (0, 0);
    let mut previous_placements = Vec::new();

    write_runtime_state(mode, 0, 0)?;
    loop {
        let observed = (|| {
            while let Some(event) = adapter.observe_toplevel()? {
                workspace.apply(event);
            }
            Ok::<(), pelagian_layoutd::AdapterError>(())
        })();
        if let Err(error) = observed {
            eprintln!("pelagian-layoutd: observation failed: {error}");
            thread::sleep(Duration::from_millis(250));
            continue;
        }

        let classified = workspace.classify(&settings.window_rules);
        if settings.automatic {
            let output = match adapter.output() {
                Ok(output) => output,
                Err(error) => {
                    eprintln!("pelagian-layoutd: output observation failed: {error}");
                    thread::sleep(Duration::from_millis(250));
                    continue;
                }
            };
            let plan = workspace.plan(output, &settings.window_rules, settings.max_managed_windows);
            let commands = transition_commands(&previous_placements, &plan);
            if let Err(error) = adapter.apply_commands(&commands) {
                eprintln!("pelagian-layoutd: reconciliation failed: {error}");
            } else {
                previous_placements = plan
                    .placements
                    .iter()
                    .map(|placement| placement.id.clone())
                    .collect();
            }
        }
        let counts = (classified.managed.len(), classified.floating.len());
        if counts != last_counts {
            write_runtime_state(mode, counts.0, counts.1)?;
            last_counts = counts;
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
