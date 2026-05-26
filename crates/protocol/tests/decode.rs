use bytes::{Bytes, BytesMut};
use protocol::{parse, RespError, RespValue};

fn buf(s: &[u8]) -> BytesMut {
    BytesMut::from(s)
}

#[test]
fn parses_simple_string() {
    let mut b = buf(b"+OK\r\n");
    assert_eq!(parse(&mut b).unwrap(), RespValue::SimpleString("OK".into()));
    assert!(b.is_empty());
}

#[test]
fn parses_error() {
    let mut b = buf(b"-ERR boom\r\n");
    assert_eq!(parse(&mut b).unwrap(), RespValue::Error("ERR boom".into()));
}

#[test]
fn parses_integer() {
    let mut b = buf(b":42\r\n");
    assert_eq!(parse(&mut b).unwrap(), RespValue::Integer(42));
}

#[test]
fn parses_negative_integer() {
    let mut b = buf(b":-7\r\n");
    assert_eq!(parse(&mut b).unwrap(), RespValue::Integer(-7));
}

#[test]
fn parses_bulk_string() {
    let mut b = buf(b"$5\r\nhello\r\n");
    assert_eq!(
        parse(&mut b).unwrap(),
        RespValue::BulkString(Some(Bytes::from_static(b"hello")))
    );
}

#[test]
fn parses_empty_bulk_string() {
    let mut b = buf(b"$0\r\n\r\n");
    assert_eq!(
        parse(&mut b).unwrap(),
        RespValue::BulkString(Some(Bytes::new()))
    );
}

#[test]
fn parses_null_bulk_string() {
    let mut b = buf(b"$-1\r\n");
    assert_eq!(parse(&mut b).unwrap(), RespValue::BulkString(None));
}

#[test]
fn parses_array() {
    let mut b = buf(b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n");
    assert_eq!(
        parse(&mut b).unwrap(),
        RespValue::Array(Some(vec![
            RespValue::BulkString(Some(Bytes::from_static(b"GET"))),
            RespValue::BulkString(Some(Bytes::from_static(b"foo"))),
        ]))
    );
}

#[test]
fn parses_empty_array() {
    let mut b = buf(b"*0\r\n");
    assert_eq!(parse(&mut b).unwrap(), RespValue::Array(Some(vec![])));
}

#[test]
fn parses_null_array() {
    let mut b = buf(b"*-1\r\n");
    assert_eq!(parse(&mut b).unwrap(), RespValue::Array(None));
}

#[test]
fn incomplete_returns_incomplete_error() {
    let mut b = buf(b"$5\r\nhel");
    assert!(matches!(parse(&mut b), Err(RespError::Incomplete)));
}

#[test]
fn malformed_returns_malformed_error() {
    let mut b = buf(b"@not-resp\r\n");
    assert!(matches!(parse(&mut b), Err(RespError::Malformed(_))));
}

#[test]
fn parses_binary_safe_bulk_with_crlf_inside() {
    let mut b = buf(b"$8\r\nfoo\r\nbar\r\n");
    assert_eq!(
        parse(&mut b).unwrap(),
        RespValue::BulkString(Some(Bytes::from_static(b"foo\r\nbar")))
    );
}
