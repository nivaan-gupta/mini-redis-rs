use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// What goes in the WAL — the durable form of a mutating command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WalRecord {
    Set {
        key: Vec<u8>,
        value: Vec<u8>,
        ttl_secs: Option<u64>,
    },
    Del {
        keys: Vec<Vec<u8>>,
    },
    Expire {
        key: Vec<u8>,
        ttl_secs: u64,
    },
    Incr {
        key: Vec<u8>,
        delta: i64,
    },
}

impl WalRecord {
    pub fn from_command(cmd: &command::Command) -> Option<Self> {
        use command::Command;
        match cmd {
            Command::Set {
                key, value, ttl, ..
            } => Some(WalRecord::Set {
                key: key.to_vec(),
                value: value.to_vec(),
                ttl_secs: ttl.map(|d| d.as_secs()),
            }),
            Command::Del(keys) => Some(WalRecord::Del {
                keys: keys.iter().map(|k| k.to_vec()).collect(),
            }),
            Command::Expire { key, ttl } => Some(WalRecord::Expire {
                key: key.to_vec(),
                ttl_secs: ttl.as_secs(),
            }),
            Command::Incr(key) => Some(WalRecord::Incr {
                key: key.to_vec(),
                delta: 1,
            }),
            Command::Decr(key) => Some(WalRecord::Incr {
                key: key.to_vec(),
                delta: -1,
            }),
            // Read-only commands are not WAL'd.
            _ => None,
        }
    }

    pub fn to_command(&self) -> command::Command {
        use command::{Command, SetCond};
        match self {
            WalRecord::Set {
                key,
                value,
                ttl_secs,
            } => Command::Set {
                key: Bytes::from(key.clone()),
                value: Bytes::from(value.clone()),
                ttl: ttl_secs.map(Duration::from_secs),
                cond: SetCond::Always,
            },
            WalRecord::Del { keys } => {
                Command::Del(keys.iter().map(|k| Bytes::from(k.clone())).collect())
            }
            WalRecord::Expire { key, ttl_secs } => Command::Expire {
                key: Bytes::from(key.clone()),
                ttl: Duration::from_secs(*ttl_secs),
            },
            WalRecord::Incr { key, delta } => {
                if *delta >= 0 {
                    Command::Incr(Bytes::from(key.clone()))
                } else {
                    Command::Decr(Bytes::from(key.clone()))
                }
            }
        }
    }
}
