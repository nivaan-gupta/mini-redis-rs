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
    // Check if redis-cli is available
    if Command::new("which").arg("redis-cli").status().is_err() {
        eprintln!("redis-cli not found, skipping test");
        return;
    }

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

    let run = |args: &[&str]| -> String {
        let out = Command::new("redis-cli")
            .args(["-p", "16380"])
            .args(args)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    assert_eq!(run(&["PING"]), "PONG");
    assert_eq!(run(&["SET", "k", "v"]), "OK");
    assert_eq!(run(&["GET", "k"]), "v");
    assert_eq!(run(&["INCR", "n"]), "1");
    assert_eq!(run(&["INCR", "n"]), "2");
    assert_eq!(run(&["DEL", "k"]), "1");
    assert_eq!(run(&["GET", "k"]), "");

    server.kill().unwrap();
    let _ = server.wait();
}
