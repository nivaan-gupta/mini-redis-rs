use std::process::{Command, Stdio};
use std::time::Duration;

fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn redis_cli_basic_commands() {
    // Build first so the binary exists.
    let build = Command::new(env!("CARGO"))
        .args(["build", "-p", "server", "--bin", "mini-redis"])
        .status()
        .unwrap();
    assert!(build.success());

    let bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/debug/mini-redis");

    let mut server = Command::new(&bin)
        .args(["--port", "16380"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    assert!(
        wait_for_port(16380, Duration::from_secs(5)),
        "server didn't start"
    );

    let run = |args: &[&str]| -> Option<String> {
        Command::new("redis-cli")
            .args(["-p", "16380"])
            .args(args)
            .output()
            .ok()
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
    };

    // Skip test if redis-cli is not available
    if run(&["PING"]).is_none() {
        eprintln!("redis-cli not found, skipping test");
        server.kill().ok();
        let _ = server.wait();
        return;
    }

    assert_eq!(run(&["PING"]), Some("PONG".into()));
    assert_eq!(run(&["SET", "k", "v"]), Some("OK".into()));
    assert_eq!(run(&["GET", "k"]), Some("v".into()));
    assert_eq!(run(&["INCR", "n"]), Some("1".into()));
    assert_eq!(run(&["INCR", "n"]), Some("2".into()));
    assert_eq!(run(&["DEL", "k"]), Some("1".into()));
    assert_eq!(run(&["GET", "k"]), Some("".into()));

    server.kill().unwrap();
    let _ = server.wait();
}
