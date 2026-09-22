use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use pelagian_layoutd::{
    CompositorAdapter, LabwcIpcAdapter, RuntimeSettings, Workspace, WorkspacePlan,
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
    let mut last_plan = None;
    let mut pending_since = None;
    let mut last_attempt = None;
    let mut force_apply = true;
    let mut managed_windows = 0;
    let mut floating_windows = 0;
    let mut published = None;
    let mut report = |health: &'static str,
                      connected: bool,
                      reconciliation: &'static str,
                      managed: usize,
                      floating: usize,
                      error: Option<String>| {
        let next = (health, connected, reconciliation, managed, floating, error);
        if published.as_ref() != Some(&next) {
            if let Some(error) = &next.5 {
                eprintln!("pelagian-layoutd: {error}");
            }
            write_runtime_state(
                mode,
                health,
                connected,
                reconciliation,
                managed,
                floating,
                next.5.as_deref(),
            )?;
            published = Some(next);
        }
        Ok::<_, std::io::Error>(())
    };
    loop {
        thread::sleep(Duration::from_millis(250));
        if let Err(error) = adapter.observe_workspace(&mut workspace) {
            report(
                "degraded",
                error.adapter_connected(),
                "error",
                managed_windows,
                floating_windows,
                Some(error.to_string()),
            )?;
            force_apply = true;
            continue;
        }
        let plan = if settings.automatic {
            workspace.plan(
                adapter.output()?,
                &settings.window_rules,
                settings.max_managed_windows,
            )
        } else {
            let mut classified = workspace.classify(&settings.window_rules);
            classified.floating.extend(classified.managed);
            WorkspacePlan {
                placements: Vec::new(),
                floating: classified.floating,
                ignored: classified.ignored,
            }
        };
        managed_windows = plan.placements.len();
        floating_windows = plan.floating.len();
        let now = Instant::now();
        if last_plan.as_ref() != Some(&plan) {
            last_plan = Some(plan.clone());
            pending_since = Some(now);
            last_attempt = None;
            force_apply = true;
        }
        let mismatch = adapter.convergence_error(&plan);
        if !force_apply && mismatch.is_none() {
            pending_since = None;
            report(
                "healthy",
                true,
                "healthy",
                managed_windows,
                floating_windows,
                None,
            )?;
            continue;
        }
        // Geometry/state churn and repeated ACKs do not reset this deadline.
        let started = *pending_since.get_or_insert(now);
        if now.duration_since(started) >= Duration::from_secs(5) {
            report(
                "degraded",
                true,
                "error",
                managed_windows,
                floating_windows,
                Some(format!(
                    "layout did not converge within 5 seconds: {}",
                    mismatch.as_deref().unwrap_or("commands await verification")
                )),
            )?;
        } else {
            report(
                "starting",
                true,
                "pending",
                managed_windows,
                floating_windows,
                None,
            )?;
        }
        // Allow asynchronous configure/commit to complete; retry at most once
        // per second, including after timeout, so reconnection can recover.
        if last_attempt
            .is_none_or(|last: Instant| now.duration_since(last) >= Duration::from_secs(1))
        {
            last_attempt = Some(now);
            match adapter.apply_commands(&reconcile_workspace_commands(&plan)) {
                Ok(()) => force_apply = false,
                Err(error) => {
                    force_apply = true;
                    report(
                        "degraded",
                        error.adapter_connected(),
                        "error",
                        managed_windows,
                        floating_windows,
                        Some(error.to_string()),
                    )?;
                }
            }
        }
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
