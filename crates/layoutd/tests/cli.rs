use std::process::Command;

#[test]
fn status_reports_live_adapter_capability_without_claiming_a_stopped_daemon_is_running() {
    let output = Command::new(env!("CARGO_BIN_EXE_pelagian-layoutd"))
        .arg("status")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"layoutd\":\"stopped\""));
    assert!(stdout.contains("\"compositor_adapter\":\"xwayland-ewmh\""));
    assert!(stdout.contains("\"adapter_scope\":\"xwayland\""));
    assert!(stdout.contains("\"native_wayland_control\":false"));
}
