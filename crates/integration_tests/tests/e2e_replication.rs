use std::process::{Child, Command, Stdio};
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

fn spawn(bin: &std::path::Path, args: &[&str]) -> Child {
    Command::new(bin)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn writes_to_leader_appear_on_two_replicas() {
    let leader_dir = tempfile::tempdir().unwrap();
    let r1_dir = tempfile::tempdir().unwrap();
    let r2_dir = tempfile::tempdir().unwrap();

    Command::new(env!("CARGO"))
        .args(["build", "-p", "server", "--bin", "mini-redis"])
        .status()
        .unwrap();
    let bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/debug/mini-redis");

    let mut leader = spawn(
        &bin,
        &[
            "--port",
            "26380",
            "--repl-port",
            "26390",
            "--data-dir",
            leader_dir.path().to_str().unwrap(),
        ],
    );
    assert!(wait_for_port(26380, Duration::from_secs(5)));
    assert!(wait_for_port(26390, Duration::from_secs(5)));

    let mut r1 = spawn(
        &bin,
        &[
            "--port",
            "26381",
            "--data-dir",
            r1_dir.path().to_str().unwrap(),
            "--replicaof",
            "127.0.0.1:26390",
        ],
    );
    let mut r2 = spawn(
        &bin,
        &[
            "--port",
            "26382",
            "--data-dir",
            r2_dir.path().to_str().unwrap(),
            "--replicaof",
            "127.0.0.1:26390",
        ],
    );
    assert!(wait_for_port(26381, Duration::from_secs(5)));
    assert!(wait_for_port(26382, Duration::from_secs(5)));

    // Wait a moment for snapshot to arrive on replicas.
    std::thread::sleep(Duration::from_millis(500));

    assert_eq!(cli(26380, &["SET", "k1", "v1"]), "OK");
    assert_eq!(cli(26380, &["SET", "k2", "v2"]), "OK");
    assert_eq!(cli(26380, &["INCR", "n"]), "1");

    // Allow replication to catch up.
    std::thread::sleep(Duration::from_millis(500));

    assert_eq!(cli(26381, &["GET", "k1"]), "v1");
    assert_eq!(cli(26381, &["GET", "n"]), "1");
    assert_eq!(cli(26382, &["GET", "k2"]), "v2");

    // Writes to replicas rejected.
    let out = cli(26382, &["SET", "rejected", "x"]);
    assert!(
        out.contains("READONLY"),
        "expected READONLY error, got: {out}"
    );

    let _ = leader.kill();
    let _ = leader.wait();
    let _ = r1.kill();
    let _ = r1.wait();
    let _ = r2.kill();
    let _ = r2.wait();
}
