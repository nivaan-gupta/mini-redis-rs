use bytes::Bytes;

/// A RESP2 value — the universe of things that can travel on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespValue {
    /// `+OK\r\n`
    SimpleString(String),
    /// `-ERR message\r\n`
    Error(String),
    /// `:42\r\n`
    Integer(i64),
    /// `$5\r\nhello\r\n` or `$-1\r\n` (null)
    BulkString(Option<Bytes>),
    /// `*N\r\n<value>...` or `*-1\r\n` (null array)
    Array(Option<Vec<RespValue>>),
}

impl RespValue {
    pub fn ok() -> Self { RespValue::SimpleString("OK".into()) }
    pub fn null_bulk() -> Self { RespValue::BulkString(None) }
    pub fn error(msg: impl Into<String>) -> Self { RespValue::Error(msg.into()) }
}
