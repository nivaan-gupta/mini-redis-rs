//! In-memory key-value store with TTL.

mod entry;

pub use entry::Entry;

use bytes::Bytes;
use command::{Command, SetCond};
use protocol::RespValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct Store {
    inner: RwLock<HashMap<Bytes, Entry>>,
}

impl Store {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub async fn apply(&self, cmd: &Command) -> RespValue {
        let now = now_secs();
        match cmd {
            Command::Ping(arg) => match arg {
                Some(b) => RespValue::BulkString(Some(b.clone())),
                None => RespValue::SimpleString("PONG".into()),
            },
            Command::Echo(b) => RespValue::BulkString(Some(b.clone())),
            Command::Get(key) => self.do_get(key, now).await,
            Command::Set {
                key,
                value,
                ttl,
                cond,
            } => {
                self.do_set(key.clone(), value.clone(), *ttl, cond.clone())
                    .await
            }
            Command::Del(keys) => self.do_del(keys, now).await,
            Command::Exists(keys) => self.do_exists(keys, now).await,
            Command::Incr(key) => self.do_incr(key.clone(), 1, now).await,
            Command::Decr(key) => self.do_incr(key.clone(), -1, now).await,
            Command::Expire { key, ttl } => self.do_expire(key, *ttl, now).await,
            Command::Ttl(key) => self.do_ttl(key, now).await,
            Command::Keys(pattern) => self.do_keys(pattern, now).await,
            Command::Info => {
                RespValue::BulkString(Some(Bytes::from_static(b"# Server\nrole:master\n")))
            }
            Command::ReplicaOf(_) => RespValue::ok(), // wired up in replication crate; store-layer no-op
        }
    }

    async fn do_get(&self, key: &Bytes, now: u64) -> RespValue {
        let guard = self.inner.read().await;
        match guard.get(key) {
            Some(e) if !e.is_expired(now) => RespValue::BulkString(Some(e.value.clone())),
            _ => RespValue::null_bulk(),
        }
    }

    async fn do_set(
        &self,
        key: Bytes,
        value: Bytes,
        ttl: Option<Duration>,
        cond: SetCond,
    ) -> RespValue {
        let mut guard = self.inner.write().await;
        let exists = guard.contains_key(&key);
        match cond {
            SetCond::IfAbsent if exists => return RespValue::null_bulk(),
            SetCond::IfExists if !exists => return RespValue::null_bulk(),
            _ => {}
        }
        guard.insert(key, Entry::new(value, ttl));
        RespValue::ok()
    }

    async fn do_del(&self, keys: &[Bytes], now: u64) -> RespValue {
        let mut guard = self.inner.write().await;
        let mut count = 0i64;
        for k in keys {
            if let Some(e) = guard.remove(k) {
                if !e.is_expired(now) {
                    count += 1;
                }
            }
        }
        RespValue::Integer(count)
    }

    async fn do_exists(&self, keys: &[Bytes], now: u64) -> RespValue {
        let guard = self.inner.read().await;
        let mut count = 0i64;
        for k in keys {
            if let Some(e) = guard.get(k) {
                if !e.is_expired(now) {
                    count += 1;
                }
            }
        }
        RespValue::Integer(count)
    }

    async fn do_incr(&self, key: Bytes, delta: i64, now: u64) -> RespValue {
        let mut guard = self.inner.write().await;
        let current: i64 = match guard.get(&key) {
            Some(e) if !e.is_expired(now) => {
                match std::str::from_utf8(&e.value)
                    .ok()
                    .and_then(|s| s.parse().ok())
                {
                    Some(n) => n,
                    None => return RespValue::error("ERR value is not an integer or out of range"),
                }
            }
            _ => 0,
        };
        let next = match current.checked_add(delta) {
            Some(n) => n,
            None => return RespValue::error("ERR increment would overflow"),
        };
        guard.insert(key, Entry::new(Bytes::from(next.to_string()), None));
        RespValue::Integer(next)
    }

    async fn do_expire(&self, key: &Bytes, ttl: Duration, now: u64) -> RespValue {
        let mut guard = self.inner.write().await;
        match guard.get_mut(key) {
            Some(e) if !e.is_expired(now) => {
                e.expires_at = Some(now + ttl.as_secs());
                RespValue::Integer(1)
            }
            _ => RespValue::Integer(0),
        }
    }

    async fn do_ttl(&self, key: &Bytes, now: u64) -> RespValue {
        let guard = self.inner.read().await;
        match guard.get(key) {
            None => RespValue::Integer(-2),
            Some(e) if e.is_expired(now) => RespValue::Integer(-2),
            Some(e) => match e.expires_at {
                None => RespValue::Integer(-1),
                Some(at) => RespValue::Integer((at - now) as i64),
            },
        }
    }

    async fn do_keys(&self, pattern: &Bytes, now: u64) -> RespValue {
        let guard = self.inner.read().await;
        let pat = std::str::from_utf8(pattern).unwrap_or("");
        let mut out = Vec::new();
        for (k, e) in guard.iter() {
            if e.is_expired(now) {
                continue;
            }
            let key_str = match std::str::from_utf8(k) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if glob_match(pat, key_str) {
                out.push(RespValue::BulkString(Some(k.clone())));
            }
        }
        RespValue::Array(Some(out))
    }

    /// Active expiration: scan a sample of keys and drop expired ones.
    /// Called from a background task.
    pub async fn sweep_expired(&self) {
        let now = now_secs();
        let mut guard = self.inner.write().await;
        guard.retain(|_, e| !e.is_expired(now));
    }

    /// For snapshot/replication: snapshot the current data.
    pub async fn snapshot(&self) -> HashMap<Bytes, Entry> {
        self.inner.read().await.clone()
    }

    pub async fn load(&self, data: HashMap<Bytes, Entry>) {
        *self.inner.write().await = data;
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Redis-style glob: `*` `?` `[abc]`.
fn glob_match(pattern: &str, s: &str) -> bool {
    glob_helper(pattern.as_bytes(), s.as_bytes())
}

fn glob_helper(p: &[u8], s: &[u8]) -> bool {
    let (mut pi, mut si) = (0usize, 0usize);
    let (mut star_p, mut star_s): (Option<usize>, usize) = (None, 0);
    while si < s.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star_p = Some(pi);
            star_s = si;
            pi += 1;
        } else if pi < p.len() && p[pi] == b'[' {
            // simple [abc] class
            let close = match p[pi..].iter().position(|&b| b == b']') {
                Some(i) => pi + i,
                None => return false,
            };
            let class = &p[pi + 1..close];
            if class.contains(&s[si]) {
                pi = close + 1;
                si += 1;
            } else if let Some(sp) = star_p {
                pi = sp + 1;
                star_s += 1;
                si = star_s;
            } else {
                return false;
            }
        } else if let Some(sp) = star_p {
            pi = sp + 1;
            star_s += 1;
            si = star_s;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}
