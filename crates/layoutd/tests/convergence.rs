use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pelagian_layoutd::{LabwcIpcAdapter, Workspace};
use serde_json::{Value, json};

struct Session {
    root: PathBuf,
    inventory: Arc<Mutex<Value>>,
    stop: Arc<AtomicBool>,
    server: Option<JoinHandle<()>>,
    child: Option<Child>,
}

impl Session {
    fn new(inventory: Value) -> Self {
        let root = std::env::temp_dir().join(format!(
            "layoutd-convergence-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("etc")).unwrap();
        let listener = UnixListener::bind(root.join("labwc.sock")).unwrap();
        listener.set_nonblocking(true).unwrap();
        let inventory = Arc::new(Mutex::new(inventory));
        let current = Arc::clone(&inventory);
        let stop = Arc::new(AtomicBool::new(false));
        let server_stop = Arc::clone(&stop);
        let server = thread::spawn(move || {
            while !server_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(1)))
                            .unwrap();
                        let mut request = String::new();
                        BufReader::new(stream.try_clone().unwrap())
                            .read_line(&mut request)
                            .unwrap();
                        let response = if request == "LIST\n" {
                            current.lock().unwrap().to_string()
                        } else {
                            assert!(request.starts_with("ACTION "), "{request}");
                            // Deliberately ACK without changing the observed window.
                            json!({"ok": true}).to_string()
                        };
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            }
        });
        Self {
            root,
            inventory,
            stop,
            server: Some(server),
            child: None,
        }
    }

    fn start_daemon(&mut self) {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        self.child = Some(
            Command::new(env!("CARGO_BIN_EXE_pelagian-layoutd"))
                .env("PELAGIAN_SHELL_DATA_DIR", repo.join("config"))
                .env("PELAGIAN_SHELL_ETC_DIR", self.root.join("etc"))
                .env("XDG_RUNTIME_DIR", &self.root)
                .env("PELAGIAN_LAYOUTD_STATE", self.root.join("state.json"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
    }

    fn status(&self) -> Option<Value> {
        fs::read_to_string(self.root.join("state.json"))
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
    }

    fn wait_for(&self, reconciliation: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            if let Some(state) = self.status() {
                if state["reconciliation"] == reconciliation {
                    return state;
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("expected {reconciliation}: {:?}", self.status());
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.stop.store(true, Ordering::Relaxed);
        self.server.take().unwrap().join().unwrap();
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn solo_inventory() -> Value {
    json!({"views":[{"id":1,"app_id":"fixture","title":"One","type":"normal",
        "parent_id":null,"x":0,"y":0,"width":800,"height":626,
        "minimized":false,"fullscreen":false,"maximized":true,"tiled":false,
        "region":"","decoration":"full","titlebar_visible":true}],
        "outputs":[{"usable_area":{"x":0,"y":0,"width":1920,"height":1080}}]})
}

#[test]
fn ack_without_convergence_times_out_despite_churn_then_recovers() {
    let mut session = Session::new(solo_inventory());
    session.start_daemon();
    session.wait_for("pending");
    let deadline = Instant::now() + Duration::from_secs(7);
    let mut width = 800;
    loop {
        let state = session.status().unwrap();
        assert_ne!(state["reconciliation"], "healthy", "ACK is not convergence");
        if state["reconciliation"] == "error" {
            assert_eq!(state["layoutd"], "degraded");
            assert_eq!(state["adapter_connected"], true);
            assert!(
                state["last_error"]
                    .as_str()
                    .unwrap()
                    .contains("did not converge")
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "geometry churn reset the deadline"
        );
        width = if width == 800 { 801 } else { 800 };
        session.inventory.lock().unwrap()["views"][0]["width"] = json!(width);
        thread::sleep(Duration::from_millis(100));
    }
    {
        let mut inventory = session.inventory.lock().unwrap();
        inventory["views"][0]["width"] = json!(1920);
        inventory["views"][0]["height"] = json!(1080);
    }
    assert_eq!(session.wait_for("healthy")["last_error"], Value::Null);
    session.inventory.lock().unwrap()["views"][0]["fullscreen"] = json!(true);
    session.wait_for("pending");
    session.inventory.lock().unwrap()["views"][0]["fullscreen"] = json!(false);
    session.wait_for("healthy");
}

#[test]
fn convergence_checks_geometry_regions_decorations_and_floating_state() {
    let mut inventory = solo_inventory();
    inventory["outputs"][0]["usable_area"] = json!({"x":20,"y":30,"width":1366,"height":768});
    inventory["views"] = json!([]);
    // Installed 33/34/33 percent regions on an odd-width output.
    for (index, (x, width)) in [
        (20, 450),
        (470, 465),
        (935, 451),
        (20, 450),
        (470, 465),
        (935, 451),
    ]
    .into_iter()
    .enumerate()
    {
        inventory["views"].as_array_mut().unwrap().push(json!({
            "id": index+1, "app_id":"fixture", "title":"tile", "type":"normal", "parent_id":null,
            "x":x,"y":if index < 3 {30} else {414},"width":width,"height":384,
            "maximized":false,"tiled":true,"region":format!("auto-6-r{}-c{}",index/3,index%3),
            "decoration":"full","titlebar_visible":true
        }));
    }
    inventory["views"].as_array_mut().unwrap().push(json!({
        "id":7,"app_id":"fixture","title":"dialog","type":"dialog","parent_id":1,
        "decoration":"full","titlebar_visible":true
    }));
    let session = Session::new(inventory.clone());
    let mut adapter = LabwcIpcAdapter::new(session.root.join("labwc.sock"));
    let mut workspace = Workspace::default();
    adapter.observe_workspace(&mut workspace).unwrap();
    let plan = workspace.plan(adapter.output().unwrap(), &[], 6);
    assert_eq!(adapter.convergence_error(&plan), None);
    for (index, field, bad) in [
        (0, "width", json!(449)),
        (0, "region", json!("auto-6-r0-c1")),
        (0, "minimized", json!(true)),
        (0, "fullscreen", json!(true)),
        (0, "decoration", json!("border")),
        (0, "titlebar_visible", json!(false)),
        (6, "tiled", json!(true)),
        (6, "maximized", json!(true)),
    ] {
        let mut altered = inventory.clone();
        altered["views"][index][field] = bad;
        *session.inventory.lock().unwrap() = altered;
        adapter.observe_workspace(&mut workspace).unwrap();
        assert!(adapter.convergence_error(&plan).is_some(), "missed {field}");
    }
}
