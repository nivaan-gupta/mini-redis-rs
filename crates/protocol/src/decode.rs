use crate::error::{RespError, Result};
use crate::value::RespValue;
use bytes::{Buf, Bytes, BytesMut};

/// Parse one RESP value from the front of `buf`.
/// On success the consumed bytes are removed from `buf`.
/// On `Incomplete` `buf` is left unchanged.
pub fn parse(buf: &mut BytesMut) -> Result<RespValue> {
    let mut cursor = 0;
    let value = parse_at(buf, &mut cursor)?;
    buf.advance(cursor);
    Ok(value)
}

fn parse_at(buf: &[u8], cursor: &mut usize) -> Result<RespValue> {
    if *cursor >= buf.len() {
        return Err(RespError::Incomplete);
    }
    let tag = buf[*cursor];
    *cursor += 1;
    match tag {
        b'+' => Ok(RespValue::SimpleString(read_line_string(buf, cursor)?)),
        b'-' => Ok(RespValue::Error(read_line_string(buf, cursor)?)),
        b':' => {
            let s = read_line_string(buf, cursor)?;
            let n: i64 = s.parse().map_err(|_| RespError::IntegerOverflow)?;
            Ok(RespValue::Integer(n))
        }
        b'$' => parse_bulk(buf, cursor),
        b'*' => parse_array(buf, cursor),
        _ => Err(RespError::Malformed("unknown type tag")),
    }
}

fn parse_bulk(buf: &[u8], cursor: &mut usize) -> Result<RespValue> {
    let len_str = read_line_string(buf, cursor)?;
    let len: i64 = len_str
        .parse()
        .map_err(|_| RespError::Malformed("bulk length"))?;
    if len < 0 {
        return Ok(RespValue::BulkString(None));
    }
    let len = len as usize;
    if buf.len() < *cursor + len + 2 {
        return Err(RespError::Incomplete);
    }
    let data = Bytes::copy_from_slice(&buf[*cursor..*cursor + len]);
    *cursor += len;
    if &buf[*cursor..*cursor + 2] != b"\r\n" {
        return Err(RespError::Malformed("missing CRLF after bulk"));
    }
    *cursor += 2;
    Ok(RespValue::BulkString(Some(data)))
}

fn parse_array(buf: &[u8], cursor: &mut usize) -> Result<RespValue> {
    let len_str = read_line_string(buf, cursor)?;
    let len: i64 = len_str
        .parse()
        .map_err(|_| RespError::Malformed("array length"))?;
    if len < 0 {
        return Ok(RespValue::Array(None));
    }
    let mut items = Vec::with_capacity(len as usize);
    for _ in 0..len {
        items.push(parse_at(buf, cursor)?);
    }
    Ok(RespValue::Array(Some(items)))
}

fn read_line_string(buf: &[u8], cursor: &mut usize) -> Result<String> {
    let start = *cursor;
    while *cursor + 1 < buf.len() {
        if buf[*cursor] == b'\r' && buf[*cursor + 1] == b'\n' {
            let s = std::str::from_utf8(&buf[start..*cursor]).map_err(|_| RespError::Utf8)?;
            *cursor += 2;
            return Ok(s.to_string());
        }
        *cursor += 1;
    }
    Err(RespError::Incomplete)
}
