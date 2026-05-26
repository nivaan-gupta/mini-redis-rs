use bytes::Bytes;
use command::{parse_command, Command, SetCond};
use protocol::RespValue;
use std::time::Duration;

fn arr(items: &[&[u8]]) -> RespValue {
    RespValue::Array(Some(
        items
            .iter()
            .map(|b| RespValue::BulkString(Some(Bytes::copy_from_slice(b))))
            .collect(),
    ))
}

#[test]
fn parses_ping_no_arg() {
    assert_eq!(parse_command(arr(&[b"PING"])).unwrap(), Command::Ping(None));
}

#[test]
fn parses_get() {
    assert_eq!(
        parse_command(arr(&[b"GET", b"foo"])).unwrap(),
        Command::Get(Bytes::from_static(b"foo"))
    );
}

#[test]
fn parses_set_basic() {
    let cmd = parse_command(arr(&[b"SET", b"k", b"v"])).unwrap();
    assert_eq!(
        cmd,
        Command::Set {
            key: Bytes::from_static(b"k"),
            value: Bytes::from_static(b"v"),
            ttl: None,
            cond: SetCond::Always,
        }
    );
}

#[test]
fn parses_set_with_ex_and_nx() {
    let cmd = parse_command(arr(&[b"SET", b"k", b"v", b"EX", b"10", b"NX"])).unwrap();
    assert_eq!(
        cmd,
        Command::Set {
            key: Bytes::from_static(b"k"),
            value: Bytes::from_static(b"v"),
            ttl: Some(Duration::from_secs(10)),
            cond: SetCond::IfAbsent,
        }
    );
}

#[test]
fn unknown_command_errors() {
    assert!(parse_command(arr(&[b"NOPE"])).is_err());
}

#[test]
fn case_insensitive_command_names() {
    assert_eq!(parse_command(arr(&[b"ping"])).unwrap(), Command::Ping(None));
    assert_eq!(parse_command(arr(&[b"PiNg"])).unwrap(), Command::Ping(None));
}
