use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pelagian_layoutd::{
    CompositorAdapter, CompositorCommand, LabwcIpcAdapter, ToplevelEvent, ToplevelKind,
};

#[test]
fn labwc_ipc_observes_native_windows_and_applies_layout_actions() {
    let socket = std::env::temp_dir().join(format!(
        "pelagian-layoutd-test-{}-{}.sock",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let listener = UnixListener::bind(&socket).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let observed_requests = Arc::clone(&requests);
    let server = thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut request)
                .unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_millis(25)))
                .unwrap();
            let mut trailing = [0_u8; 1];
            assert!(matches!(
                stream.read(&mut trailing),
                Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
            ));
            observed_requests.lock().unwrap().push(request.clone());
            if request == "LIST\n" {
                stream
                    .write_all(
                        br#"{"views":[{"id":1,"pid":100,"app_id":"org.test.One","title":"One","type":"normal","parent_id":null},{"id":2,"pid":100,"app_id":"org.test.One","title":"Dialog","type":"dialog","parent_id":1}],"outputs":[{"name":"HEADLESS-1","usable_area":{"x":0,"y":0,"width":1920,"height":1080}}]}
"#,
                    )
                    .unwrap();
            } else {
                stream.write_all(b"{\"ok\":true}\n").unwrap();
            }
        }
    });

    let mut adapter = LabwcIpcAdapter::new(&socket);
    let first = adapter.observe_toplevel().unwrap().unwrap();
    let second = adapter.observe_toplevel().unwrap().unwrap();
    assert_eq!(
        first,
        ToplevelEvent::Upsert(pelagian_layoutd::Toplevel {
            id: "1".to_owned(),
            app_id: "org.test.One".to_owned(),
            title: "One".to_owned(),
            kind: ToplevelKind::Normal,
            parent_id: None,
        })
    );
    assert_eq!(
        second,
        ToplevelEvent::Upsert(pelagian_layoutd::Toplevel {
            id: "2".to_owned(),
            app_id: "org.test.One".to_owned(),
            title: "Dialog".to_owned(),
            kind: ToplevelKind::Dialog,
            parent_id: Some("1".to_owned()),
        })
    );
    assert_eq!(adapter.output().unwrap().width, 1920);
    adapter
        .apply_commands(&[
            CompositorCommand::Maximize {
                toplevel_id: "1".to_owned(),
            },
            CompositorCommand::Snap {
                toplevel_id: "1".to_owned(),
                region: "auto-2-left".to_owned(),
            },
        ])
        .unwrap();

    server.join().unwrap();
    assert_eq!(
        *requests.lock().unwrap(),
        [
            "LIST\n",
            "ACTION 1 MAXIMIZE\n",
            "ACTION 1 SNAP auto-2-left\n",
        ]
    );
    fs::remove_file(socket).unwrap();
}

#[test]
fn labwc_ipc_preserves_numeric_creation_order() {
    let socket = std::env::temp_dir().join(format!(
        "pelagian-layoutd-order-{}-{}.sock",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let listener = UnixListener::bind(&socket).unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut request)
            .unwrap();
        assert_eq!(request, "LIST\n");
        stream.write_all(br#"{"views":[{"id":10,"app_id":"same","title":"Ten","type":"normal","parent_id":null},{"id":9,"app_id":"same","title":"Nine","type":"normal","parent_id":null}],"outputs":[{"usable_area":{"width":1280,"height":720}}]}"#).unwrap();
    });

    let mut adapter = LabwcIpcAdapter::new(&socket);
    let first = adapter.observe_toplevel().unwrap().unwrap();
    let second = adapter.observe_toplevel().unwrap().unwrap();
    assert!(matches!(first, ToplevelEvent::Upsert(view) if view.id == "9"));
    assert!(matches!(second, ToplevelEvent::Upsert(view) if view.id == "10"));
    server.join().unwrap();
    fs::remove_file(socket).unwrap();
}

#[test]
fn output_and_client_geometry_changes_request_bounded_reconciliation() {
    let socket = std::env::temp_dir().join(format!(
        "pelagian-layoutd-reflow-{}-{}.sock",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let listener = UnixListener::bind(&socket).unwrap();
    let responses = [
        br#"{"views":[{"id":1,"app_id":"same","title":"One","type":"normal","parent_id":null,"x":0,"y":0,"width":1280,"height":720,"minimized":false,"fullscreen":false,"maximized":true,"tiled":false,"region":"","focused":false}],"outputs":[{"usable_area":{"width":1280,"height":720}}]}"#.as_slice(),
        br#"{"views":[{"id":1,"app_id":"same","title":"One","type":"normal","parent_id":null,"x":0,"y":0,"width":1920,"height":1080,"minimized":false,"fullscreen":false,"maximized":true,"tiled":false,"region":"","focused":false}],"outputs":[{"usable_area":{"width":1920,"height":1080}}]}"#.as_slice(),
        br#"{"views":[{"id":1,"app_id":"same","title":"One","type":"normal","parent_id":null,"x":25,"y":25,"width":900,"height":600,"minimized":false,"fullscreen":false,"maximized":false,"tiled":false,"region":"","focused":false}],"outputs":[{"usable_area":{"width":1920,"height":1080}}]}"#.as_slice(),
        br#"{"views":[{"id":1,"app_id":"same","title":"One","type":"normal","parent_id":null,"x":25,"y":25,"width":900,"height":600,"minimized":false,"fullscreen":false,"maximized":false,"tiled":false,"region":"","focused":true}],"outputs":[{"usable_area":{"width":1920,"height":1080}}]}"#.as_slice(),
        br#"{"views":[{"id":1,"app_id":"same","title":"One","type":"normal","parent_id":null,"x":25,"y":25,"width":900,"height":600,"minimized":false,"fullscreen":false,"maximized":false,"tiled":false,"region":"","decoration":"border","titlebar_visible":false,"focused":true}],"outputs":[{"usable_area":{"width":1920,"height":1080}}]}"#.as_slice(),
    ];
    let server = thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut request)
                .unwrap();
            assert_eq!(request, "LIST\n");
            stream.write_all(response).unwrap();
        }
    });

    let mut adapter = LabwcIpcAdapter::new(&socket);
    assert!(matches!(
        adapter.observe_toplevel().unwrap(),
        Some(ToplevelEvent::Upsert(_))
    ));
    assert_eq!(
        adapter.observe_toplevel().unwrap(),
        Some(ToplevelEvent::Reconcile)
    );
    assert_eq!(
        adapter.observe_toplevel().unwrap(),
        Some(ToplevelEvent::Reconcile)
    );
    assert_eq!(adapter.observe_toplevel().unwrap(), None);
    assert_eq!(
        adapter.observe_toplevel().unwrap(),
        Some(ToplevelEvent::Reconcile)
    );
    server.join().unwrap();
    fs::remove_file(socket).unwrap();
}

#[test]
fn trickling_labwc_ipc_cannot_extend_the_request_deadline() {
    let socket = std::env::temp_dir().join(format!(
        "pelagian-layoutd-trickle-{}-{}.sock",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let listener = UnixListener::bind(&socket).unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut request)
            .unwrap();
        assert_eq!(request, "LIST\n");
        for _ in 0..8 {
            if stream.write_all(b"{").is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    let mut adapter = LabwcIpcAdapter::new(&socket);
    let started = Instant::now();
    let error = adapter.observe_toplevel().unwrap_err().to_string();
    assert!(started.elapsed() < Duration::from_millis(600));
    assert!(
        error.contains("deadline") || error.contains("timed out"),
        "{error}"
    );
    server.join().unwrap();
    fs::remove_file(socket).unwrap();
}

#[test]
fn unresponsive_labwc_ipc_is_time_bounded() {
    let socket = std::env::temp_dir().join(format!(
        "pelagian-layoutd-timeout-{}-{}.sock",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let listener = UnixListener::bind(&socket).unwrap();
    let server = thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_millis(750));
    });

    let mut adapter = LabwcIpcAdapter::new(&socket);
    let started = Instant::now();
    let error = adapter.observe_toplevel().unwrap_err().to_string();
    assert!(started.elapsed() < Duration::from_millis(600));
    assert!(
        error.contains("timeout") || error.contains("timed out"),
        "{error}"
    );
    server.join().unwrap();
    fs::remove_file(socket).unwrap();
}
