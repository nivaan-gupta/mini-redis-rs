//! Typed commands parsed from RESP arrays.

mod parse;

use bytes::Bytes;
use std::time::Duration;
use thiserror::Error;

pub use parse::parse_command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetCond {
    Always,
    IfAbsent,
    IfExists,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ping(Option<Bytes>),
    Echo(Bytes),
    Get(Bytes),
    Set {
        key: Bytes,
        value: Bytes,
        ttl: Option<Duration>,
        cond: SetCond,
    },
    Del(Vec<Bytes>),
    Exists(Vec<Bytes>),
    Incr(Bytes),
    Decr(Bytes),
    Expire {
        key: Bytes,
        ttl: Duration,
    },
    Ttl(Bytes),
    Keys(Bytes),
    // pattern as bytes
    Info,
    ReplicaOf(Option<(String, u16)>), // None means REPLICAOF NO ONE
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("wrong number of arguments for '{0}'")]
    WrongArity(String),
    #[error("unknown command '{0}'")]
    Unknown(String),
    #[error("invalid argument: {0}")]
    InvalidArg(&'static str),
    #[error("value is not an integer or out of range")]
    NotInteger,
    #[error("expected command as RESP array of bulk strings")]
    BadFraming,
}
