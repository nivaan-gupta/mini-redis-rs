use std::process::{Command, Stdio};
use std::time::Duration;

fn wait_for_port(port: u16, t: Duration) -> bool {
    let dl = std::time::Instant::now() + t;
    while std::time::Instant::now() < dl {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn cli(port: u16, args: &[&str]) -> String {
    let out = Command::new("redis-cli")
        .args(["-p", &port.to_string()])
        .args(args)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn data_survives_kill_minus_9() {
    let dir = tempfile::tempdir().unwrap();

    Command::new(env!("CARGO"))
        .args(["build", "--bin", "mini-redis"])
        .status()
        .unwrap();
    let bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/debug/mini-redis");

    let mut child = Command::new(&bin)
        .args(["--port", "16381", "--data-dir"])
        .arg(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    assert!(wait_for_port(16381, Duration::from_secs(5)));

    assert_eq!(cli(16381, &["SET", "foo", "bar"]), "OK");
    assert_eq!(cli(16381, &["INCR", "n"]), "1");
    assert_eq!(cli(16381, &["INCR", "n"]), "2");

    // Hard kill.
    unsafe {
        libc::kill(child.id() as i32, libc::SIGKILL);
    }
    let _ = child.wait();

    // Restart.
    let mut child2 = Command::new(&bin)
        .args(["--port", "16381", "--data-dir"])
        .arg(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    assert!(wait_for_port(16381, Duration::from_secs(5)));

    assert_eq!(cli(16381, &["GET", "foo"]), "bar");
    assert_eq!(cli(16381, &["GET", "n"]), "2");

    child2.kill().unwrap();
    let _ = child2.wait();
}
