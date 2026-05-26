use crate::value::RespValue;
use bytes::{BufMut, Bytes, BytesMut};

pub fn encode(value: &RespValue) -> Bytes {
    let mut buf = BytesMut::with_capacity(64);
    encode_into(value, &mut buf);
    buf.freeze()
}

fn encode_into(value: &RespValue, buf: &mut BytesMut) {
    match value {
        RespValue::SimpleString(s) => {
            buf.put_u8(b'+');
            buf.put_slice(s.as_bytes());
            buf.put_slice(b"\r\n");
        }
        RespValue::Error(s) => {
            buf.put_u8(b'-');
            buf.put_slice(s.as_bytes());
            buf.put_slice(b"\r\n");
        }
        RespValue::Integer(n) => {
            buf.put_u8(b':');
            buf.put_slice(n.to_string().as_bytes());
            buf.put_slice(b"\r\n");
        }
        RespValue::BulkString(None) => {
            buf.put_slice(b"$-1\r\n");
        }
        RespValue::BulkString(Some(b)) => {
            buf.put_u8(b'$');
            buf.put_slice(b.len().to_string().as_bytes());
            buf.put_slice(b"\r\n");
            buf.put_slice(b);
            buf.put_slice(b"\r\n");
        }
        RespValue::Array(None) => {
            buf.put_slice(b"*-1\r\n");
        }
        RespValue::Array(Some(items)) => {
            buf.put_u8(b'*');
            buf.put_slice(items.len().to_string().as_bytes());
            buf.put_slice(b"\r\n");
            for item in items {
                encode_into(item, buf);
            }
        }
    }
}
