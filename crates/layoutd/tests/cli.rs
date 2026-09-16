use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn wait_for_state(path: &std::path::Path, marker: &str) -> String {
    for _ in 0..50 {
        if let Ok(raw) = fs::read_to_string(path) {
            if raw.contains(marker) {
                return raw;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!(
        "state marker {marker:?} was not written: {}",
        path.display()
    );
}

#[test]
fn status_reports_live_labwc_adapter_and_never_planner_only() {
    let output = Command::new(env!("CARGO_BIN_EXE_pelagian-layoutd"))
        .arg("status")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("planner_only"), "{stdout}");
    assert!(
        stdout.contains("\"compositor_adapter\":\"labwc-ipc\""),
        "{stdout}"
    );
    assert!(stdout.contains("\"layoutd\":\"stopped\""), "{stdout}");
}

#[test]
fn daemon_without_labwc_reports_disconnected_degraded_health() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime = std::env::temp_dir().join(format!(
        "pelagian-layoutd-disconnected-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(runtime.join("etc")).unwrap();
    let state = runtime.join("state.json");
    let mut child = Command::new(env!("CARGO_BIN_EXE_pelagian-layoutd"))
        .env("PELAGIAN_SHELL_DATA_DIR", root.join("config"))
        .env("PELAGIAN_SHELL_ETC_DIR", runtime.join("etc"))
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("PELAGIAN_LAYOUTD_STATE", &state)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let raw = wait_for_state(&state, "degraded");
    let status: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(status["layoutd"], "degraded");
    assert_eq!(status["adapter_connected"], false);
    assert_eq!(status["reconciliation"], "error");
    assert!(status["last_error"].as_str().unwrap().contains("connect"));

    let live = Command::new(env!("CARGO_BIN_EXE_pelagian-layoutd"))
        .arg("status")
        .env("PELAGIAN_LAYOUTD_STATE", &state)
        .output()
        .unwrap();
    let live: serde_json::Value = serde_json::from_slice(&live.stdout).unwrap();
    assert_eq!(live["layoutd"], "degraded");
    assert_eq!(live["adapter_connected"], false);
    assert_eq!(live["reconciliation"], "error");

    child.kill().unwrap();
    child.wait().unwrap();
    fs::remove_dir_all(runtime).unwrap();
}

#[test]
fn action_connection_failure_reports_disconnected_health() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime = std::env::temp_dir().join(format!(
        "pelagian-layoutd-action-disconnect-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(runtime.join("etc")).unwrap();
    let state = runtime.join("state.json");
    let listener = UnixListener::bind(runtime.join("labwc.sock")).unwrap();
    let server = thread::spawn(move || {
        for request_number in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut request)
                .unwrap();
            if request_number < 2 {
                assert_eq!(request, "LIST\n");
                stream
                    .write_all(
                        br#"{"views":[{"id":1,"app_id":"fixture","title":"Fixture","type":"normal","parent_id":null}],"outputs":[{"usable_area":{"width":1280,"height":720}}]}"#,
                    )
                    .unwrap();
            } else {
                assert_eq!(request, "ACTION 1 DECORATION full\n");
                stream.write_all(br#"{"ok":true}"#).unwrap();
            }
        }
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_pelagian-layoutd"))
        .env("PELAGIAN_SHELL_DATA_DIR", root.join("config"))
        .env("PELAGIAN_SHELL_ETC_DIR", runtime.join("etc"))
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("PELAGIAN_LAYOUTD_STATE", &state)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    server.join().unwrap();
    let raw = wait_for_state(&state, "cannot connect");
    let status: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(status["layoutd"], "degraded");
    assert_eq!(status["adapter_connected"], false);
    assert_eq!(status["reconciliation"], "error");

    child.kill().unwrap();
    child.wait().unwrap();
    fs::remove_dir_all(runtime).unwrap();
}

#[test]
fn daemon_survives_a_window_disappearing_during_reconciliation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime = std::env::temp_dir().join(format!(
        "pelagian-layoutd-cli-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(runtime.join("etc")).unwrap();
    let listener = UnixListener::bind(runtime.join("labwc.sock")).unwrap();
    let server = thread::spawn(move || {
        let mut list_count = 0;
        let mut maximize_count = 0;
        for _ in 0..7 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut request)
                .unwrap();
            if request == "LIST\n" {
                list_count += 1;
                stream
                    .write_all(
                        br#"{"views":[{"id":1,"app_id":"fixture","title":"Fixture","type":"normal","parent_id":null}],"outputs":[{"usable_area":{"width":1280,"height":720}}]}"#,
                    )
                    .unwrap();
            } else if request == "ACTION 1 MAXIMIZE\n" {
                maximize_count += 1;
                stream
                    .write_all(if maximize_count == 1 {
                        br#"{"ok":false,"error":"view disappeared"}"#
                    } else {
                        br#"{"ok":true}"#
                    })
                    .unwrap();
            } else {
                assert_eq!(request, "ACTION 1 DECORATION full\n");
                stream.write_all(br#"{"ok":true}"#).unwrap();
            }
        }
        assert_eq!(list_count, 3);
        assert_eq!(maximize_count, 2);
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_pelagian-layoutd"))
        .env("PELAGIAN_SHELL_DATA_DIR", root.join("config"))
        .env("PELAGIAN_SHELL_ETC_DIR", runtime.join("etc"))
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("PELAGIAN_LAYOUTD_STATE", runtime.join("state.json"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    server.join().unwrap();
    thread::sleep(Duration::from_millis(350));
    let status = child.try_wait().unwrap();
    if status.is_none() {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    fs::remove_dir_all(runtime).unwrap();
    assert!(
        status.is_none(),
        "layout daemon exited after a transient action failure"
    );
}

#[test]
fn daemon_survives_malformed_inventory_during_event_drain() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime = std::env::temp_dir().join(format!(
        "pelagian-layoutd-malformed-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(runtime.join("etc")).unwrap();
    let listener = UnixListener::bind(runtime.join("labwc.sock")).unwrap();
    let server = thread::spawn(move || {
        let mut list_count = 0;
        for _ in 0..5 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut request)
                .unwrap();
            if request == "LIST\n" {
                list_count += 1;
                if list_count == 2 {
                    stream.write_all(b"not-json").unwrap();
                } else {
                    stream
                        .write_all(
                            br#"{"views":[{"id":1,"app_id":"fixture","title":"Fixture","type":"normal","parent_id":null}],"outputs":[{"usable_area":{"width":1280,"height":720}}]}"#,
                        )
                        .unwrap();
                }
            } else {
                assert!(
                    request == "ACTION 1 DECORATION full\n" || request == "ACTION 1 MAXIMIZE\n",
                    "{request:?}"
                );
                stream.write_all(br#"{"ok":true}"#).unwrap();
            }
        }
        assert_eq!(list_count, 3);
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_pelagian-layoutd"))
        .env("PELAGIAN_SHELL_DATA_DIR", root.join("config"))
        .env("PELAGIAN_SHELL_ETC_DIR", runtime.join("etc"))
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("PELAGIAN_LAYOUTD_STATE", runtime.join("state.json"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    server.join().unwrap();
    thread::sleep(Duration::from_millis(350));
    let status = child.try_wait().unwrap();
    if status.is_none() {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    fs::remove_dir_all(runtime).unwrap();
    assert!(
        status.is_none(),
        "layout daemon exited after malformed inventory"
    );
}
