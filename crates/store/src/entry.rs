use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    #[serde(with = "serde_bytes_compat")]
    pub value: Bytes,
    /// Absolute deadline (seconds since UNIX epoch) when entry expires.
    pub expires_at: Option<u64>,
}

impl Entry {
    pub fn new(value: Bytes, ttl: Option<Duration>) -> Self {
        let expires_at = ttl.map(|d| {
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + d.as_secs()
        });
        Entry { value, expires_at }
    }
    pub fn is_expired(&self, now_secs: u64) -> bool {
        self.expires_at.is_some_and(|e| now_secs >= e)
    }
}

// bytes::Bytes serde compat
mod serde_bytes_compat {
    use bytes::Bytes;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &Bytes, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(v.as_ref())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Bytes, D::Error> {
        let v: Vec<u8> = Vec::deserialize(d)?;
        Ok(Bytes::from(v))
    }
}
