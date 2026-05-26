use bytes::Bytes;
use protocol::{encode, RespValue};

#[test]
fn encodes_simple_string() {
    let out = encode(&RespValue::SimpleString("OK".into()));
    assert_eq!(&out[..], b"+OK\r\n");
}

#[test]
fn encodes_error() {
    let out = encode(&RespValue::Error("ERR something".into()));
    assert_eq!(&out[..], b"-ERR something\r\n");
}

#[test]
fn encodes_integer() {
    let out = encode(&RespValue::Integer(42));
    assert_eq!(&out[..], b":42\r\n");
}

#[test]
fn encodes_negative_integer() {
    let out = encode(&RespValue::Integer(-7));
    assert_eq!(&out[..], b":-7\r\n");
}

#[test]
fn encodes_bulk_string() {
    let out = encode(&RespValue::BulkString(Some(Bytes::from_static(b"hello"))));
    assert_eq!(&out[..], b"$5\r\nhello\r\n");
}

#[test]
fn encodes_empty_bulk_string() {
    let out = encode(&RespValue::BulkString(Some(Bytes::new())));
    assert_eq!(&out[..], b"$0\r\n\r\n");
}

#[test]
fn encodes_null_bulk_string() {
    let out = encode(&RespValue::BulkString(None));
    assert_eq!(&out[..], b"$-1\r\n");
}

#[test]
fn encodes_array() {
    let out = encode(&RespValue::Array(Some(vec![
        RespValue::BulkString(Some(Bytes::from_static(b"GET"))),
        RespValue::BulkString(Some(Bytes::from_static(b"foo"))),
    ])));
    assert_eq!(&out[..], b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n");
}

#[test]
fn encodes_empty_array() {
    let out = encode(&RespValue::Array(Some(vec![])));
    assert_eq!(&out[..], b"*0\r\n");
}

#[test]
fn encodes_null_array() {
    let out = encode(&RespValue::Array(None));
    assert_eq!(&out[..], b"*-1\r\n");
}

#[test]
fn encodes_nested_array() {
    let out = encode(&RespValue::Array(Some(vec![
        RespValue::Integer(1),
        RespValue::Array(Some(vec![RespValue::Integer(2)])),
    ])));
    assert_eq!(&out[..], b"*2\r\n:1\r\n*1\r\n:2\r\n");
}
