use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    if env::args().skip(1).collect::<Vec<_>>().as_slice() == ["status"] {
        println!(
            "{{\"schema_version\":1,\"mode\":\"planner_only\",\"compositor_adapter\":\"unavailable\"}}"
        );
        ExitCode::SUCCESS
    } else {
        eprintln!("usage: pelagian-layoutd status");
        ExitCode::from(2)
    }
}
