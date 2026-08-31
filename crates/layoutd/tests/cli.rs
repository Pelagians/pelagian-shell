use std::process::Command;

#[test]
fn status_is_explicit_that_no_live_compositor_adapter_exists() {
    let output = Command::new(env!("CARGO_BIN_EXE_pelagian-layoutd"))
        .arg("status")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "{\"schema_version\":1,\"mode\":\"planner_only\",\"compositor_adapter\":\"unavailable\"}\n"
    );
}
