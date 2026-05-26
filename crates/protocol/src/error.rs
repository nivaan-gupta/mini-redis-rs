use thiserror::Error;

#[derive(Debug, Error)]
pub enum RespError {
    #[error("incomplete frame (need more bytes)")]
    Incomplete,
    #[error("malformed RESP: {0}")]
    Malformed(&'static str),
    #[error("invalid UTF-8 in simple string or error")]
    Utf8,
    #[error("integer out of range")]
    IntegerOverflow,
}

pub type Result<T> = std::result::Result<T, RespError>;
