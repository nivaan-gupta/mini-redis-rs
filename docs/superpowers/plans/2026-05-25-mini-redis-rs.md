# mini-redis-rs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **For Nivaan (the human builder):** This plan is your daily guide for the next 14 days. Each day has tasks broken into 2–5 minute steps. Do them in order. Commit after each task. If you get stuck for >30 minutes, re-read the design spec at `docs/superpowers/specs/2026-05-25-mini-redis-rs-design.md`, then ask for help.

**Goal:** Build a Redis-protocol-compatible, persistent, replicated key-value store in Rust over 14 days.

**Architecture:** Cargo workspace with 6 layered crates (`protocol`, `command`, `store`, `persistence`, `replication`, `server`) + a `bin` binary. Async I/O via `tokio`. Single-leader replication. Append-only WAL + periodic snapshots for durability.

**Tech Stack:** Rust (stable) · tokio · bytes · clap · serde + bincode · thiserror · tracing · proptest · criterion · tempfile

---

## Phase 0: Prerequisites (do before Day 1)

These are environment-setup tasks. Knock them out the day before you start so Day 1 is purely coding.

- [ ] **P0.1: Install Rust toolchain**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# accept defaults; restart your shell
rustc --version  # expect 1.80+ or newer
cargo --version
```

- [ ] **P0.2: Install Redis locally (for benchmarks + compatibility tests)**

```bash
brew install redis
redis-cli --version
redis-benchmark --version
redis-server --version
```

- [ ] **P0.3: Install VS Code + rust-analyzer extension**

Open VS Code → Extensions → search "rust-analyzer" → Install. Restart VS Code.

- [ ] **P0.4: Read these (concentrated Rust onboarding, ~6 hours)**

These are the only docs you need. Don't try to read the whole Rust book — focus:

1. The Rust Book, chapters 1–11 + 13 + 15 + 16: https://doc.rust-lang.org/book/
   - Especially **Ch. 4 (ownership)**, **Ch. 10 (generics + lifetimes)**, **Ch. 15 (smart pointers)**, **Ch. 16 (concurrency)**
2. Tokio tutorial — only the first 4 sections: https://tokio.rs/tokio/tutorial
3. Skim `bytes` crate docs (5 minutes): https://docs.rs/bytes

Pause when you hit something confusing and come back to it during Phase 1 when you're applying it.

---

## Phase 1: Foundation (Days 1–2)

**Goal:** A scaffolded Cargo workspace with CI, CLI flag parsing, structured logging, and a working `cargo run -- --help`. No business logic yet — purely setup.

### Task 1.1: Initialize the Cargo workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `.gitignore`
- Modify: none

- [ ] **Step 1: Verify clean repo state**

```bash
cd /Users/nivaan/mini-redis-rs
git status
```
Expected: only the design doc and (existing) `.git/`.

- [ ] **Step 2: Create the workspace `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = [
    "crates/protocol",
    "crates/command",
    "crates/store",
    "crates/persistence",
    "crates/replication",
    "crates/server",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Nivaan Gupta <nivaangupta2005@gmail.com>"]
license = "MIT"
repository = "https://github.com/nivaan-gupta/mini-redis-rs"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
bytes = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
bincode = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
proptest = "1"
criterion = { version = "0.5", features = ["html_reports"] }
tempfile = "3"
anyhow = "1"
```

- [ ] **Step 3: Create `.gitignore`**

```
/target
**/*.rs.bk
Cargo.lock
.DS_Store
/data
```

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml .gitignore
git commit -m "chore: initialize Cargo workspace"
```

### Task 1.2: Scaffold the six member crates

**Files:**
- Create: `crates/protocol/{Cargo.toml,src/lib.rs}`
- Create: `crates/command/{Cargo.toml,src/lib.rs}`
- Create: `crates/store/{Cargo.toml,src/lib.rs}`
- Create: `crates/persistence/{Cargo.toml,src/lib.rs}`
- Create: `crates/replication/{Cargo.toml,src/lib.rs}`
- Create: `crates/server/{Cargo.toml,src/lib.rs}`

- [ ] **Step 1: Create each crate skeleton**

For each crate listed above, create two files. Example for `protocol`:

`crates/protocol/Cargo.toml`:
```toml
[package]
name = "protocol"
version.workspace = true
edition.workspace = true

[dependencies]
bytes = { workspace = true }
thiserror = { workspace = true }
```

`crates/protocol/src/lib.rs`:
```rust
//! RESP2 wire-protocol parser and encoder.
```

Do the same for `command`, `store`, `persistence`, `replication`, `server` — adjust the `Cargo.toml` dependencies to whatever each crate eventually needs (you can start with empty `[dependencies]` and add as you go in later tasks).

- [ ] **Step 2: Verify the workspace builds**

```bash
cargo build
```
Expected: builds with `warning: crate ... has no library targets` for each empty crate — that's fine, they'll fill in.

- [ ] **Step 3: Commit**

```bash
git add crates/
git commit -m "chore: scaffold six member crates"
```

### Task 1.3: Add the binary crate

**Files:**
- Create: `crates/server/src/bin/mini-redis.rs`
- Modify: `crates/server/Cargo.toml`

- [ ] **Step 1: Add dependencies to `crates/server/Cargo.toml`**

```toml
[package]
name = "server"
version.workspace = true
edition.workspace = true

[dependencies]
tokio = { workspace = true }
clap = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }

[[bin]]
name = "mini-redis"
path = "src/bin/mini-redis.rs"
```

- [ ] **Step 2: Create the binary**

`crates/server/src/bin/mini-redis.rs`:
```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mini-redis", version, about = "A Redis-compatible KV store in Rust")]
struct Args {
    /// TCP port to bind
    #[arg(short, long, default_value_t = 6380)]
    port: u16,

    /// Data directory for WAL + snapshots
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,

    /// Run as replica of leader at host:port
    #[arg(long)]
    replicaof: Option<String>,

    /// fsync on every WAL append (slower, stronger durability)
    #[arg(long)]
    fsync_every_write: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    tracing::info!(?args, "starting mini-redis");

    // Phase 1 stub — real server boot lands in Task 3.6
    tracing::info!("server scaffold ready (no listener yet)");
    Ok(())
}
```

- [ ] **Step 3: Verify it builds and runs**

```bash
cargo run --bin mini-redis -- --help
```
Expected: clap prints flag help.

```bash
RUST_LOG=info cargo run --bin mini-redis
```
Expected: two `info` lines, then exits.

- [ ] **Step 4: Commit**

```bash
git add crates/server/
git commit -m "feat: add mini-redis binary with CLI flags + structured logging"
```

### Task 1.4: GitHub Actions CI

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the workflow**

`.github/workflows/ci.yml`:
```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt }
      - run: cargo fmt --all -- --check

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy }
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets -- -D warnings

  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --all-features
```

- [ ] **Step 2: Locally verify clippy + fmt pass**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
Expected: all three pass with no errors.

- [ ] **Step 3: Commit**

```bash
git add .github/
git commit -m "ci: add fmt, clippy, test workflows on ubuntu + macos"
```

### Task 1.5: Push to GitHub

**Files:** none

- [ ] **Step 1: Create the GitHub repo via `gh`**

```bash
gh repo create mini-redis-rs --public \
  --description "Redis-protocol-compatible KV store in Rust — single-leader replication + WAL persistence" \
  --source=. --remote=origin --push
```

- [ ] **Step 2: Verify CI runs**

Visit `https://github.com/nivaan-gupta/mini-redis-rs/actions`. The three jobs should be green within ~3 minutes.

- [ ] **Step 3: Add topics + README placeholder**

```bash
gh repo edit nivaan-gupta/mini-redis-rs \
  --add-topic rust --add-topic redis --add-topic distributed-systems \
  --add-topic key-value-store --add-topic database --add-topic tokio
echo "# mini-redis-rs\n\nWork in progress. See design doc in [docs/](docs/)." > README.md
git add README.md && git commit -m "docs: add placeholder README"
git push
```

---

## Phase 2: RESP2 Protocol (Days 3–4)

**Goal:** A fully-tested RESP2 parser and encoder. After this phase, `cargo test -p protocol` shows green and `proptest` round-trips pass for arbitrary RESP values.

### Task 2.1: Define `RespValue` and `RespError`

**Files:**
- Modify: `crates/protocol/Cargo.toml`
- Create: `crates/protocol/src/lib.rs` (replace)
- Create: `crates/protocol/src/value.rs`
- Create: `crates/protocol/src/error.rs`

- [ ] **Step 1: Update `Cargo.toml`**

```toml
[package]
name = "protocol"
version.workspace = true
edition.workspace = true

[dependencies]
bytes = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

- [ ] **Step 2: Create the value enum**

`crates/protocol/src/value.rs`:
```rust
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
```

- [ ] **Step 3: Create the error type**

`crates/protocol/src/error.rs`:
```rust
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
```

- [ ] **Step 4: Wire it up**

`crates/protocol/src/lib.rs`:
```rust
//! RESP2 wire protocol — parsing and encoding.

mod error;
mod value;

pub use error::{RespError, Result};
pub use value::RespValue;
```

- [ ] **Step 5: Verify it builds**

```bash
cargo build -p protocol
```
Expected: clean build.

- [ ] **Step 6: Commit**

```bash
git add crates/protocol/
git commit -m "feat(protocol): define RespValue, RespError types"
```

### Task 2.2: Implement RESP2 encoder (TDD)

**Files:**
- Create: `crates/protocol/src/encode.rs`
- Create: `crates/protocol/tests/encode.rs`
- Modify: `crates/protocol/src/lib.rs`

- [ ] **Step 1: Write the failing test**

`crates/protocol/tests/encode.rs`:
```rust
use protocol::{encode, RespValue};
use bytes::Bytes;

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
```

- [ ] **Step 2: Run to confirm it fails**

```bash
cargo test -p protocol --test encode
```
Expected: `error[E0432]: unresolved import 'protocol::encode'`.

- [ ] **Step 3: Implement the encoder**

`crates/protocol/src/encode.rs`:
```rust
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
```

- [ ] **Step 4: Export the encoder**

In `crates/protocol/src/lib.rs`, add:
```rust
mod encode;
pub use encode::encode;
```

- [ ] **Step 5: Run to confirm all 11 tests pass**

```bash
cargo test -p protocol --test encode
```
Expected: `test result: ok. 11 passed; 0 failed`.

- [ ] **Step 6: Commit**

```bash
git add crates/protocol/
git commit -m "feat(protocol): RESP2 encoder + 11 unit tests"
```

### Task 2.3: Implement RESP2 decoder (TDD)

**Files:**
- Create: `crates/protocol/src/decode.rs`
- Create: `crates/protocol/tests/decode.rs`
- Modify: `crates/protocol/src/lib.rs`

- [ ] **Step 1: Write the failing test**

`crates/protocol/tests/decode.rs`:
```rust
use protocol::{parse, RespError, RespValue};
use bytes::{Bytes, BytesMut};

fn buf(s: &[u8]) -> BytesMut { BytesMut::from(s) }

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
    let mut b = buf(b"$7\r\nfoo\r\nbar\r\n");
    assert_eq!(
        parse(&mut b).unwrap(),
        RespValue::BulkString(Some(Bytes::from_static(b"foo\r\nbar")))
    );
}
```

- [ ] **Step 2: Confirm it fails**

```bash
cargo test -p protocol --test decode
```
Expected: `unresolved import 'protocol::parse'`.

- [ ] **Step 3: Implement the decoder**

`crates/protocol/src/decode.rs`:
```rust
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
    let len: i64 = len_str.parse().map_err(|_| RespError::Malformed("bulk length"))?;
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
    let len: i64 = len_str.parse().map_err(|_| RespError::Malformed("array length"))?;
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
```

- [ ] **Step 4: Export it**

In `crates/protocol/src/lib.rs`:
```rust
mod decode;
pub use decode::parse;
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p protocol --test decode
```
Expected: `test result: ok. 13 passed; 0 failed`.

- [ ] **Step 6: Commit**

```bash
git add crates/protocol/
git commit -m "feat(protocol): RESP2 decoder + 13 unit tests including binary-safe bulk"
```

### Task 2.4: Property-based round-trip tests

**Files:**
- Create: `crates/protocol/tests/roundtrip.rs`

- [ ] **Step 1: Write the proptest**

`crates/protocol/tests/roundtrip.rs`:
```rust
use protocol::{encode, parse, RespValue};
use bytes::{Bytes, BytesMut};
use proptest::prelude::*;

fn arb_resp_value() -> impl Strategy<Value = RespValue> {
    let leaf = prop_oneof![
        any::<String>().prop_filter("no CR/LF in simple", |s| !s.contains('\r') && !s.contains('\n'))
            .prop_map(RespValue::SimpleString),
        any::<String>().prop_filter("no CR/LF in error", |s| !s.contains('\r') && !s.contains('\n'))
            .prop_map(RespValue::Error),
        any::<i64>().prop_map(RespValue::Integer),
        any::<Vec<u8>>().prop_map(|v| RespValue::BulkString(Some(Bytes::from(v)))),
        Just(RespValue::BulkString(None)),
    ];
    leaf.prop_recursive(3, 16, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(|v| RespValue::Array(Some(v))),
            Just(RespValue::Array(None)),
        ]
    })
}

proptest! {
    #[test]
    fn encode_then_parse_roundtrip(val in arb_resp_value()) {
        let bytes = encode(&val);
        let mut buf = BytesMut::from(&bytes[..]);
        let parsed = parse(&mut buf).expect("parse must succeed");
        prop_assert_eq!(parsed, val);
        prop_assert!(buf.is_empty(), "all bytes consumed");
    }
}
```

- [ ] **Step 2: Run it**

```bash
cargo test -p protocol --test roundtrip
```
Expected: passes with default 256 cases.

- [ ] **Step 3: Commit**

```bash
git add crates/protocol/tests/roundtrip.rs
git commit -m "test(protocol): proptest round-trip for arbitrary RespValues"
```

---

## Phase 3: Store, Commands, Connection (Days 5–6)

**Goal:** At the end of this phase, `redis-cli -p 6380 ping` returns `PONG`, and you can `SET foo bar`, `GET foo`, `DEL foo`, `INCR counter`, `EXPIRE foo 1`, and `KEYS "*"` against your binary.

### Task 3.1: Define `Command` enum + parser

**Files:**
- Modify: `crates/command/Cargo.toml`
- Create: `crates/command/src/lib.rs`
- Create: `crates/command/src/parse.rs`
- Create: `crates/command/tests/parse.rs`

- [ ] **Step 1: Update dependencies**

`crates/command/Cargo.toml`:
```toml
[package]
name = "command"
version.workspace = true
edition.workspace = true

[dependencies]
protocol = { path = "../protocol" }
bytes = { workspace = true }
thiserror = { workspace = true }
```

- [ ] **Step 2: Define the command types**

`crates/command/src/lib.rs`:
```rust
//! Typed commands parsed from RESP arrays.

mod parse;

use bytes::Bytes;
use std::time::Duration;
use thiserror::Error;

pub use parse::parse_command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetCond { Always, IfAbsent, IfExists }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ping(Option<Bytes>),
    Echo(Bytes),
    Get(Bytes),
    Set { key: Bytes, value: Bytes, ttl: Option<Duration>, cond: SetCond },
    Del(Vec<Bytes>),
    Exists(Vec<Bytes>),
    Incr(Bytes),
    Decr(Bytes),
    Expire { key: Bytes, ttl: Duration },
    Ttl(Bytes),
    Keys(Bytes), // pattern as bytes
    Info,
    ReplicaOf(Option<(String, u16)>),  // None means REPLICAOF NO ONE
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
```

- [ ] **Step 3: Implement the parser**

`crates/command/src/parse.rs`:
```rust
use crate::{Command, CommandError, SetCond};
use bytes::Bytes;
use protocol::RespValue;
use std::time::Duration;

pub fn parse_command(value: RespValue) -> Result<Command, CommandError> {
    let items = match value {
        RespValue::Array(Some(items)) => items,
        _ => return Err(CommandError::BadFraming),
    };
    let mut bulks: Vec<Bytes> = items.into_iter().map(|v| match v {
        RespValue::BulkString(Some(b)) => Ok(b),
        _ => Err(CommandError::BadFraming),
    }).collect::<Result<_, _>>()?;

    if bulks.is_empty() {
        return Err(CommandError::BadFraming);
    }
    let cmd_name = std::str::from_utf8(&bulks[0])
        .map_err(|_| CommandError::BadFraming)?
        .to_ascii_uppercase();
    bulks.remove(0); // consume name

    match cmd_name.as_str() {
        "PING" => match bulks.len() {
            0 => Ok(Command::Ping(None)),
            1 => Ok(Command::Ping(Some(bulks.remove(0)))),
            _ => Err(CommandError::WrongArity("PING".into())),
        },
        "ECHO" => {
            if bulks.len() != 1 { return Err(CommandError::WrongArity("ECHO".into())); }
            Ok(Command::Echo(bulks.remove(0)))
        }
        "GET" => {
            if bulks.len() != 1 { return Err(CommandError::WrongArity("GET".into())); }
            Ok(Command::Get(bulks.remove(0)))
        }
        "SET" => parse_set(bulks),
        "DEL" => {
            if bulks.is_empty() { return Err(CommandError::WrongArity("DEL".into())); }
            Ok(Command::Del(bulks))
        }
        "EXISTS" => {
            if bulks.is_empty() { return Err(CommandError::WrongArity("EXISTS".into())); }
            Ok(Command::Exists(bulks))
        }
        "INCR" => {
            if bulks.len() != 1 { return Err(CommandError::WrongArity("INCR".into())); }
            Ok(Command::Incr(bulks.remove(0)))
        }
        "DECR" => {
            if bulks.len() != 1 { return Err(CommandError::WrongArity("DECR".into())); }
            Ok(Command::Decr(bulks.remove(0)))
        }
        "EXPIRE" => {
            if bulks.len() != 2 { return Err(CommandError::WrongArity("EXPIRE".into())); }
            let secs = parse_i64(&bulks[1])?;
            if secs < 0 { return Err(CommandError::InvalidArg("seconds must be >= 0")); }
            Ok(Command::Expire { key: bulks.remove(0), ttl: Duration::from_secs(secs as u64) })
        }
        "TTL" => {
            if bulks.len() != 1 { return Err(CommandError::WrongArity("TTL".into())); }
            Ok(Command::Ttl(bulks.remove(0)))
        }
        "KEYS" => {
            if bulks.len() != 1 { return Err(CommandError::WrongArity("KEYS".into())); }
            Ok(Command::Keys(bulks.remove(0)))
        }
        "INFO" => Ok(Command::Info),
        "REPLICAOF" => {
            if bulks.len() != 2 { return Err(CommandError::WrongArity("REPLICAOF".into())); }
            let host = std::str::from_utf8(&bulks[0]).map_err(|_| CommandError::InvalidArg("host"))?.to_string();
            let port_str = std::str::from_utf8(&bulks[1]).map_err(|_| CommandError::InvalidArg("port"))?;
            if host.eq_ignore_ascii_case("NO") && port_str.eq_ignore_ascii_case("ONE") {
                return Ok(Command::ReplicaOf(None));
            }
            let port: u16 = port_str.parse().map_err(|_| CommandError::InvalidArg("port not u16"))?;
            Ok(Command::ReplicaOf(Some((host, port))))
        }
        other => Err(CommandError::Unknown(other.to_string())),
    }
}

fn parse_set(mut args: Vec<Bytes>) -> Result<Command, CommandError> {
    if args.len() < 2 { return Err(CommandError::WrongArity("SET".into())); }
    let key = args.remove(0);
    let value = args.remove(0);
    let mut ttl = None;
    let mut cond = SetCond::Always;
    let mut i = 0;
    while i < args.len() {
        let token = std::str::from_utf8(&args[i]).map_err(|_| CommandError::InvalidArg("SET option"))?.to_ascii_uppercase();
        match token.as_str() {
            "EX" => {
                if i + 1 >= args.len() { return Err(CommandError::WrongArity("SET".into())); }
                let secs = parse_i64(&args[i + 1])?;
                if secs <= 0 { return Err(CommandError::InvalidArg("EX must be > 0")); }
                ttl = Some(Duration::from_secs(secs as u64));
                i += 2;
            }
            "PX" => {
                if i + 1 >= args.len() { return Err(CommandError::WrongArity("SET".into())); }
                let ms = parse_i64(&args[i + 1])?;
                if ms <= 0 { return Err(CommandError::InvalidArg("PX must be > 0")); }
                ttl = Some(Duration::from_millis(ms as u64));
                i += 2;
            }
            "NX" => { cond = SetCond::IfAbsent; i += 1; }
            "XX" => { cond = SetCond::IfExists; i += 1; }
            _ => return Err(CommandError::InvalidArg("unknown SET option")),
        }
    }
    Ok(Command::Set { key, value, ttl, cond })
}

fn parse_i64(b: &[u8]) -> Result<i64, CommandError> {
    std::str::from_utf8(b).map_err(|_| CommandError::NotInteger)?
        .parse::<i64>().map_err(|_| CommandError::NotInteger)
}
```

- [ ] **Step 4: Test the parser**

`crates/command/tests/parse.rs`:
```rust
use bytes::Bytes;
use command::{parse_command, Command, SetCond};
use protocol::RespValue;
use std::time::Duration;

fn arr(items: &[&[u8]]) -> RespValue {
    RespValue::Array(Some(items.iter().map(|b| RespValue::BulkString(Some(Bytes::copy_from_slice(b)))).collect()))
}

#[test]
fn parses_ping_no_arg() {
    assert_eq!(parse_command(arr(&[b"PING"])).unwrap(), Command::Ping(None));
}

#[test]
fn parses_get() {
    assert_eq!(
        parse_command(arr(&[b"GET", b"foo"])).unwrap(),
        Command::Get(Bytes::from_static(b"foo"))
    );
}

#[test]
fn parses_set_basic() {
    let cmd = parse_command(arr(&[b"SET", b"k", b"v"])).unwrap();
    assert_eq!(cmd, Command::Set {
        key: Bytes::from_static(b"k"),
        value: Bytes::from_static(b"v"),
        ttl: None,
        cond: SetCond::Always,
    });
}

#[test]
fn parses_set_with_ex_and_nx() {
    let cmd = parse_command(arr(&[b"SET", b"k", b"v", b"EX", b"10", b"NX"])).unwrap();
    assert_eq!(cmd, Command::Set {
        key: Bytes::from_static(b"k"),
        value: Bytes::from_static(b"v"),
        ttl: Some(Duration::from_secs(10)),
        cond: SetCond::IfAbsent,
    });
}

#[test]
fn unknown_command_errors() {
    assert!(parse_command(arr(&[b"NOPE"])).is_err());
}

#[test]
fn case_insensitive_command_names() {
    assert_eq!(parse_command(arr(&[b"ping"])).unwrap(), Command::Ping(None));
    assert_eq!(parse_command(arr(&[b"PiNg"])).unwrap(), Command::Ping(None));
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p command
```
Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/command/
git commit -m "feat(command): typed Command enum + RESP parser with 6 unit tests"
```

### Task 3.2: Implement `Store` with TTL

**Files:**
- Modify: `crates/store/Cargo.toml`
- Create: `crates/store/src/lib.rs`
- Create: `crates/store/src/entry.rs`
- Create: `crates/store/tests/store.rs`

- [ ] **Step 1: Update dependencies**

```toml
[package]
name = "store"
version.workspace = true
edition.workspace = true

[dependencies]
command = { path = "../command" }
protocol = { path = "../protocol" }
bytes = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 2: Define `Entry`**

`crates/store/src/entry.rs`:
```rust
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
        let expires_at = ttl.map(|d| SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() + d.as_secs());
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
```

- [ ] **Step 3: Implement `Store`**

`crates/store/src/lib.rs`:
```rust
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
    pub fn new() -> Arc<Self> { Arc::new(Self::default()) }

    pub async fn apply(&self, cmd: &Command) -> RespValue {
        let now = now_secs();
        match cmd {
            Command::Ping(arg) => match arg {
                Some(b) => RespValue::BulkString(Some(b.clone())),
                None => RespValue::SimpleString("PONG".into()),
            },
            Command::Echo(b) => RespValue::BulkString(Some(b.clone())),
            Command::Get(key) => self.do_get(key, now).await,
            Command::Set { key, value, ttl, cond } => self.do_set(key.clone(), value.clone(), *ttl, cond.clone()).await,
            Command::Del(keys) => self.do_del(keys, now).await,
            Command::Exists(keys) => self.do_exists(keys, now).await,
            Command::Incr(key) => self.do_incr(key.clone(), 1, now).await,
            Command::Decr(key) => self.do_incr(key.clone(), -1, now).await,
            Command::Expire { key, ttl } => self.do_expire(key, *ttl, now).await,
            Command::Ttl(key) => self.do_ttl(key, now).await,
            Command::Keys(pattern) => self.do_keys(pattern, now).await,
            Command::Info => RespValue::BulkString(Some(Bytes::from_static(b"# Server\nrole:master\n"))),
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

    async fn do_set(&self, key: Bytes, value: Bytes, ttl: Option<Duration>, cond: SetCond) -> RespValue {
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
                if !e.is_expired(now) { count += 1; }
            }
        }
        RespValue::Integer(count)
    }

    async fn do_exists(&self, keys: &[Bytes], now: u64) -> RespValue {
        let guard = self.inner.read().await;
        let mut count = 0i64;
        for k in keys {
            if let Some(e) = guard.get(k) {
                if !e.is_expired(now) { count += 1; }
            }
        }
        RespValue::Integer(count)
    }

    async fn do_incr(&self, key: Bytes, delta: i64, now: u64) -> RespValue {
        let mut guard = self.inner.write().await;
        let current: i64 = match guard.get(&key) {
            Some(e) if !e.is_expired(now) => {
                match std::str::from_utf8(&e.value).ok().and_then(|s| s.parse().ok()) {
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
            if e.is_expired(now) { continue; }
            let key_str = match std::str::from_utf8(k) { Ok(s) => s, Err(_) => continue };
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
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
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
            pi += 1; si += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star_p = Some(pi); star_s = si; pi += 1;
        } else if pi < p.len() && p[pi] == b'[' {
            // simple [abc] class
            let close = match p[pi..].iter().position(|&b| b == b']') {
                Some(i) => pi + i,
                None => return false,
            };
            let class = &p[pi + 1..close];
            if class.contains(&s[si]) { pi = close + 1; si += 1; }
            else if let Some(sp) = star_p { pi = sp + 1; star_s += 1; si = star_s; }
            else { return false; }
        } else if let Some(sp) = star_p {
            pi = sp + 1; star_s += 1; si = star_s;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' { pi += 1; }
    pi == p.len()
}
```

- [ ] **Step 4: Test the store**

`crates/store/tests/store.rs`:
```rust
use bytes::Bytes;
use command::{Command, SetCond};
use protocol::RespValue;
use std::time::Duration;
use store::Store;

fn b(s: &'static [u8]) -> Bytes { Bytes::from_static(s) }

#[tokio::test]
async fn set_then_get() {
    let s = Store::new();
    s.apply(&Command::Set { key: b(b"k"), value: b(b"v"), ttl: None, cond: SetCond::Always }).await;
    assert_eq!(s.apply(&Command::Get(b(b"k"))).await, RespValue::BulkString(Some(b(b"v"))));
}

#[tokio::test]
async fn get_missing_returns_null_bulk() {
    let s = Store::new();
    assert_eq!(s.apply(&Command::Get(b(b"nope"))).await, RespValue::null_bulk());
}

#[tokio::test]
async fn del_count() {
    let s = Store::new();
    s.apply(&Command::Set { key: b(b"a"), value: b(b"1"), ttl: None, cond: SetCond::Always }).await;
    s.apply(&Command::Set { key: b(b"b"), value: b(b"2"), ttl: None, cond: SetCond::Always }).await;
    assert_eq!(s.apply(&Command::Del(vec![b(b"a"), b(b"missing"), b(b"b")])).await, RespValue::Integer(2));
}

#[tokio::test]
async fn incr_creates_then_increments() {
    let s = Store::new();
    assert_eq!(s.apply(&Command::Incr(b(b"n"))).await, RespValue::Integer(1));
    assert_eq!(s.apply(&Command::Incr(b(b"n"))).await, RespValue::Integer(2));
    assert_eq!(s.apply(&Command::Decr(b(b"n"))).await, RespValue::Integer(1));
}

#[tokio::test]
async fn incr_on_non_integer_errors() {
    let s = Store::new();
    s.apply(&Command::Set { key: b(b"k"), value: b(b"abc"), ttl: None, cond: SetCond::Always }).await;
    match s.apply(&Command::Incr(b(b"k"))).await {
        RespValue::Error(_) => {}
        other => panic!("expected error, got {:?}", other),
    }
}

#[tokio::test]
async fn ttl_expires_value() {
    let s = Store::new();
    s.apply(&Command::Set { key: b(b"k"), value: b(b"v"), ttl: Some(Duration::from_secs(1)), cond: SetCond::Always }).await;
    assert_eq!(s.apply(&Command::Get(b(b"k"))).await, RespValue::BulkString(Some(b(b"v"))));
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert_eq!(s.apply(&Command::Get(b(b"k"))).await, RespValue::null_bulk());
}

#[tokio::test]
async fn nx_only_sets_if_absent() {
    let s = Store::new();
    assert_eq!(
        s.apply(&Command::Set { key: b(b"k"), value: b(b"first"), ttl: None, cond: SetCond::IfAbsent }).await,
        RespValue::ok()
    );
    assert_eq!(
        s.apply(&Command::Set { key: b(b"k"), value: b(b"second"), ttl: None, cond: SetCond::IfAbsent }).await,
        RespValue::null_bulk()
    );
    assert_eq!(s.apply(&Command::Get(b(b"k"))).await, RespValue::BulkString(Some(b(b"first"))));
}

#[tokio::test]
async fn keys_glob_star() {
    let s = Store::new();
    for k in [b"user:1" as &[u8], b"user:2", b"order:9"] {
        s.apply(&Command::Set { key: Bytes::copy_from_slice(k), value: b(b"x"), ttl: None, cond: SetCond::Always }).await;
    }
    let res = s.apply(&Command::Keys(Bytes::from_static(b"user:*"))).await;
    match res {
        RespValue::Array(Some(items)) => assert_eq!(items.len(), 2),
        other => panic!("expected array, got {:?}", other),
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p store
```
Expected: 8 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/store/
git commit -m "feat(store): in-memory KV store with TTL, INCR/DECR, NX/XX, KEYS globbing"
```

### Task 3.3: Per-connection tokio handler

**Files:**
- Modify: `crates/server/Cargo.toml`
- Create: `crates/server/src/lib.rs`
- Create: `crates/server/src/conn.rs`

- [ ] **Step 1: Update server `Cargo.toml`**

```toml
[package]
name = "server"
version.workspace = true
edition.workspace = true

[dependencies]
protocol = { path = "../protocol" }
command = { path = "../command" }
store = { path = "../store" }
tokio = { workspace = true }
bytes = { workspace = true }
clap = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }

[[bin]]
name = "mini-redis"
path = "src/bin/mini-redis.rs"
```

- [ ] **Step 2: Implement `ConnectionHandler`**

`crates/server/src/conn.rs`:
```rust
use bytes::BytesMut;
use command::parse_command;
use protocol::{encode, parse, RespError, RespValue};
use std::sync::Arc;
use store::Store;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn handle_connection(mut stream: TcpStream, store: Arc<Store>) -> std::io::Result<()> {
    let mut buf = BytesMut::with_capacity(4096);
    loop {
        match parse(&mut buf) {
            Ok(value) => {
                let response = match parse_command(value) {
                    Ok(cmd) => store.apply(&cmd).await,
                    Err(e) => RespValue::error(format!("ERR {}", e)),
                };
                stream.write_all(&encode(&response)).await?;
            }
            Err(RespError::Incomplete) => {
                let n = stream.read_buf(&mut buf).await?;
                if n == 0 {
                    return Ok(()); // client disconnected
                }
            }
            Err(other) => {
                let err = RespValue::error(format!("ERR {}", other));
                stream.write_all(&encode(&err)).await?;
                buf.clear();
            }
        }
    }
}
```

- [ ] **Step 3: Add `conn` to lib.rs**

`crates/server/src/lib.rs`:
```rust
//! Server: TCP listener + per-connection task wiring.

pub mod conn;
```

(We add the `server` module + `Server` export in Task 3.4 once the file exists. Don't commit yet — Tasks 3.3 + 3.4 form one buildable unit.)

### Task 3.4: TCP listener with background TTL sweep

**Files:**
- Create: `crates/server/src/server.rs`

- [ ] **Step 1: Add `server` module to lib.rs**

Update `crates/server/src/lib.rs`:
```rust
//! Server: TCP listener + per-connection task wiring.

pub mod conn;
pub mod server;

pub use server::Server;
```

- [ ] **Step 2: Implement `Server`**

`crates/server/src/server.rs`:
```rust
use crate::conn::handle_connection;
use std::sync::Arc;
use std::time::Duration;
use store::Store;
use tokio::net::TcpListener;

pub struct Server {
    addr: String,
    store: Arc<Store>,
}

impl Server {
    pub fn new(port: u16, store: Arc<Store>) -> Self {
        Self { addr: format!("0.0.0.0:{port}"), store }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        tracing::info!(addr = %self.addr, "listening");

        // Background TTL sweep every 5s.
        let sweep_store = self.store.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(5));
            loop {
                ticker.tick().await;
                sweep_store.sweep_expired().await;
            }
        });

        loop {
            let (stream, peer) = listener.accept().await?;
            tracing::debug!(?peer, "accepted connection");
            let store = self.store.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, store).await {
                    tracing::warn!(error = %e, "connection error");
                }
            });
        }
    }
}
```

- [ ] **Step 3: Wire the binary to actually run the server**

Replace `crates/server/src/bin/mini-redis.rs`:
```rust
use clap::Parser;
use server::Server;
use std::path::PathBuf;
use std::sync::Arc;
use store::Store;

#[derive(Parser, Debug)]
#[command(name = "mini-redis", version, about = "A Redis-compatible KV store in Rust")]
struct Args {
    #[arg(short, long, default_value_t = 6380)]
    port: u16,
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,
    #[arg(long)]
    replicaof: Option<String>,
    #[arg(long)]
    fsync_every_write: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args = Args::parse();
    tracing::info!(?args, "starting mini-redis");

    let store = Store::new();
    Server::new(args.port, store).run().await
}
```

- [ ] **Step 4: Build + run + smoke test with `redis-cli`**

```bash
cargo run --bin mini-redis
```

In another terminal:
```bash
redis-cli -p 6380 PING
# expect: PONG
redis-cli -p 6380 SET hello world
# expect: OK
redis-cli -p 6380 GET hello
# expect: "world"
redis-cli -p 6380 INCR counter
# expect: (integer) 1
redis-cli -p 6380 INCR counter
# expect: (integer) 2
redis-cli -p 6380 KEYS "*"
# expect: 1) "hello"  2) "counter"
```

Stop the server with Ctrl+C.

- [ ] **Step 5: Commit**

```bash
git add crates/server/
git commit -m "feat(server): TCP listener + per-conn handler + TTL sweeper (redis-cli works!)"
```

### Task 3.5: End-to-end integration test with `redis-cli`

**Files:**
- Create: `tests/e2e_redis_cli.rs`
- Modify: root `Cargo.toml` to add `[package]` and `[dev-dependencies]`... actually integration tests at workspace root need a wrapper crate. Use a test crate inside the workspace instead.
- Create: `crates/integration_tests/Cargo.toml`
- Create: `crates/integration_tests/tests/e2e_redis_cli.rs`
- Modify: root `Cargo.toml` to add `integration_tests` to members

- [ ] **Step 1: Add the integration_tests crate**

Edit root `Cargo.toml` — add `"crates/integration_tests"` to `[workspace] members`.

Create `crates/integration_tests/Cargo.toml`:
```toml
[package]
name = "integration_tests"
version.workspace = true
edition.workspace = true
publish = false

[dev-dependencies]
tokio = { workspace = true }
anyhow = { workspace = true }
tempfile = { workspace = true }
```

Create `crates/integration_tests/src/lib.rs`:
```rust
// Empty — this crate exists only for its integration tests.
```

- [ ] **Step 2: Write the e2e test**

`crates/integration_tests/tests/e2e_redis_cli.rs`:
```rust
use std::process::{Command, Stdio};
use std::time::Duration;

fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn redis_cli_basic_commands() {
    // Build first so the binary exists.
    let build = Command::new(env!("CARGO")).args(["build", "--bin", "mini-redis"]).status().unwrap();
    assert!(build.success());

    let bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap()
        .join("target/debug/mini-redis");

    let mut server = Command::new(&bin)
        .args(["--port", "16380"])
        .stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().unwrap();

    assert!(wait_for_port(16380, Duration::from_secs(5)), "server didn't start");

    let run = |args: &[&str]| -> String {
        let out = Command::new("redis-cli").args(["-p", "16380"]).args(args).output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    assert_eq!(run(&["PING"]), "PONG");
    assert_eq!(run(&["SET", "k", "v"]), "OK");
    assert_eq!(run(&["GET", "k"]), "v");
    assert_eq!(run(&["INCR", "n"]), "1");
    assert_eq!(run(&["INCR", "n"]), "2");
    assert_eq!(run(&["DEL", "k"]), "1");
    assert_eq!(run(&["GET", "k"]), "");

    server.kill().unwrap();
    let _ = server.wait();
}
```

- [ ] **Step 3: Run it**

```bash
cargo test -p integration_tests -- --test-threads=1
```
Expected: passes. (Requires `redis-cli` on PATH — installed in P0.2.)

- [ ] **Step 4: Commit**

```bash
git add crates/integration_tests/ Cargo.toml
git commit -m "test: end-to-end test running real redis-cli against our server"
git push
```

Verify CI green on GitHub.

---

## Phase 4: Persistence — WAL + Snapshot + Recovery (Days 7–8)

**Goal:** After this phase, `kill -9` on the server doesn't lose data. `cargo test -p integration_tests --test e2e_crash_recovery` passes.

### Task 4.1: Write-ahead log (append-only)

**Files:**
- Modify: `crates/persistence/Cargo.toml`
- Create: `crates/persistence/src/lib.rs`
- Create: `crates/persistence/src/wal.rs`
- Create: `crates/persistence/tests/wal.rs`

- [ ] **Step 1: Update dependencies**

```toml
[package]
name = "persistence"
version.workspace = true
edition.workspace = true

[dependencies]
protocol = { path = "../protocol" }
command = { path = "../command" }
store = { path = "../store" }
tokio = { workspace = true }
bytes = { workspace = true }
serde = { workspace = true }
bincode = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 2: Define the WAL**

`crates/persistence/src/wal.rs`:
```rust
//! Append-only write-ahead log. Each record:
//! [u32 length BE][bincode-encoded WalRecord]

use crate::record::WalRecord;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, SeekFrom};
use tokio::sync::Mutex;

pub struct Wal {
    file: Arc<Mutex<File>>,
    path: PathBuf,
    fsync_every: bool,
}

impl Wal {
    pub async fn open(path: impl AsRef<Path>, fsync_every: bool) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new().create(true).append(true).read(true).open(&path).await?;
        Ok(Self { file: Arc::new(Mutex::new(file)), path, fsync_every })
    }

    pub async fn append(&self, record: &WalRecord) -> std::io::Result<()> {
        let payload = bincode::serialize(record).expect("WAL record serializable");
        let len = payload.len() as u32;
        let mut guard = self.file.lock().await;
        guard.write_all(&len.to_be_bytes()).await?;
        guard.write_all(&payload).await?;
        if self.fsync_every {
            guard.sync_data().await?;
        } else {
            guard.flush().await?;
        }
        Ok(())
    }

    pub async fn replay(&self) -> std::io::Result<Vec<WalRecord>> {
        let f = File::open(&self.path).await?;
        let mut reader = BufReader::new(f);
        let mut out = Vec::new();
        loop {
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut payload = vec![0u8; len];
            match reader.read_exact(&mut payload).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    tracing::warn!("WAL truncated mid-record; stopping replay");
                    break;
                }
                Err(e) => return Err(e),
            }
            match bincode::deserialize(&payload) {
                Ok(r) => out.push(r),
                Err(e) => {
                    tracing::warn!(error = %e, "WAL record corrupt; stopping replay");
                    break;
                }
            }
        }
        Ok(out)
    }

    pub async fn truncate(&self) -> std::io::Result<()> {
        let mut guard = self.file.lock().await;
        guard.set_len(0).await?;
        guard.seek(SeekFrom::Start(0)).await?;
        Ok(())
    }
}
```

- [ ] **Step 3: Define the record type**

`crates/persistence/src/record.rs`:
```rust
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// What goes in the WAL — the durable form of a mutating command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WalRecord {
    Set { key: Vec<u8>, value: Vec<u8>, ttl_secs: Option<u64> },
    Del { keys: Vec<Vec<u8>> },
    Expire { key: Vec<u8>, ttl_secs: u64 },
    Incr { key: Vec<u8>, delta: i64 },
}

impl WalRecord {
    pub fn from_command(cmd: &command::Command) -> Option<Self> {
        use command::Command;
        match cmd {
            Command::Set { key, value, ttl, .. } => Some(WalRecord::Set {
                key: key.to_vec(),
                value: value.to_vec(),
                ttl_secs: ttl.map(|d| d.as_secs()),
            }),
            Command::Del(keys) => Some(WalRecord::Del { keys: keys.iter().map(|k| k.to_vec()).collect() }),
            Command::Expire { key, ttl } => Some(WalRecord::Expire { key: key.to_vec(), ttl_secs: ttl.as_secs() }),
            Command::Incr(key) => Some(WalRecord::Incr { key: key.to_vec(), delta: 1 }),
            Command::Decr(key) => Some(WalRecord::Incr { key: key.to_vec(), delta: -1 }),
            // Read-only commands are not WAL'd.
            _ => None,
        }
    }

    pub fn to_command(&self) -> command::Command {
        use command::{Command, SetCond};
        match self {
            WalRecord::Set { key, value, ttl_secs } => Command::Set {
                key: Bytes::from(key.clone()),
                value: Bytes::from(value.clone()),
                ttl: ttl_secs.map(Duration::from_secs),
                cond: SetCond::Always,
            },
            WalRecord::Del { keys } => Command::Del(keys.iter().map(|k| Bytes::from(k.clone())).collect()),
            WalRecord::Expire { key, ttl_secs } => Command::Expire { key: Bytes::from(key.clone()), ttl: Duration::from_secs(*ttl_secs) },
            WalRecord::Incr { key, delta } => {
                if *delta >= 0 { Command::Incr(Bytes::from(key.clone())) }
                else { Command::Decr(Bytes::from(key.clone())) }
            }
        }
    }
}
```

- [ ] **Step 4: Library entrypoint (snapshot added in 4.2)**

`crates/persistence/src/lib.rs`:
```rust
//! Persistence: WAL + snapshot + recovery.

pub mod wal;
pub mod record;

pub use record::WalRecord;
pub use wal::Wal;
```

`snapshot` will be added to this file in Task 4.2 once `snapshot.rs` exists.

- [ ] **Step 5: Test the WAL**

`crates/persistence/tests/wal.rs`:
```rust
use persistence::{Wal, WalRecord};
use tempfile::tempdir;

#[tokio::test]
async fn append_then_replay_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wal.log");
    let wal = Wal::open(&path, false).await.unwrap();

    let records = vec![
        WalRecord::Set { key: b"k1".to_vec(), value: b"v1".to_vec(), ttl_secs: None },
        WalRecord::Set { key: b"k2".to_vec(), value: b"v2".to_vec(), ttl_secs: Some(60) },
        WalRecord::Del { keys: vec![b"k1".to_vec()] },
        WalRecord::Incr { key: b"counter".to_vec(), delta: 1 },
    ];
    for r in &records {
        wal.append(r).await.unwrap();
    }

    let replayed = wal.replay().await.unwrap();
    assert_eq!(replayed, records);
}

#[tokio::test]
async fn truncated_wal_stops_at_last_complete_record() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wal.log");
    let wal = Wal::open(&path, false).await.unwrap();

    wal.append(&WalRecord::Set { key: b"k".to_vec(), value: b"v".to_vec(), ttl_secs: None }).await.unwrap();

    // Append a length prefix but no payload (simulate crash mid-write).
    use tokio::io::AsyncWriteExt;
    use tokio::fs::OpenOptions;
    let mut f = OpenOptions::new().append(true).open(&path).await.unwrap();
    f.write_all(&[0,0,0,42]).await.unwrap(); // claims 42-byte record
    f.flush().await.unwrap();
    drop(f);

    let replayed = wal.replay().await.unwrap();
    assert_eq!(replayed.len(), 1);
}
```

- [ ] **Step 6: Run + commit**

```bash
cargo test -p persistence --test wal
git add crates/persistence/
git commit -m "feat(persistence): WAL with append, replay, truncation-safe recovery"
```

### Task 4.2: Snapshot

**Files:**
- Create: `crates/persistence/src/snapshot.rs`
- Create: `crates/persistence/tests/snapshot.rs`

- [ ] **Step 1: Update lib.rs to expose snapshot**

Update `crates/persistence/src/lib.rs`:
```rust
//! Persistence: WAL + snapshot + recovery.

pub mod wal;
pub mod record;
pub mod snapshot;

pub use record::WalRecord;
pub use wal::Wal;
pub use snapshot::Snapshot;
```

- [ ] **Step 2: Implement snapshot**

`crates/persistence/src/snapshot.rs`:
```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use bytes::Bytes;
use store::Entry;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct Snapshot { path: PathBuf }

impl Snapshot {
    pub fn new(path: impl AsRef<Path>) -> Self { Self { path: path.as_ref().to_path_buf() } }

    pub async fn write(&self, data: &HashMap<Bytes, Entry>) -> std::io::Result<()> {
        let tmp = self.path.with_extension("rdb.tmp");
        let serializable: HashMap<Vec<u8>, Entry> =
            data.iter().map(|(k, v)| (k.to_vec(), v.clone())).collect();
        let bytes = bincode::serialize(&serializable).expect("snapshot serializes");
        let mut f = fs::File::create(&tmp).await?;
        f.write_all(&bytes).await?;
        f.sync_data().await?;
        fs::rename(&tmp, &self.path).await?;  // atomic replace
        Ok(())
    }

    pub async fn read(&self) -> std::io::Result<Option<HashMap<Bytes, Entry>>> {
        let bytes = match fs::read(&self.path).await {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let raw: HashMap<Vec<u8>, Entry> = bincode::deserialize(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(raw.into_iter().map(|(k, v)| (Bytes::from(k), v)).collect()))
    }
}
```

- [ ] **Step 3: Test snapshot**

`crates/persistence/tests/snapshot.rs`:
```rust
use bytes::Bytes;
use persistence::Snapshot;
use std::collections::HashMap;
use store::Entry;
use tempfile::tempdir;

#[tokio::test]
async fn round_trips_data() {
    let dir = tempdir().unwrap();
    let snap = Snapshot::new(dir.path().join("snap.rdb"));
    let mut data = HashMap::new();
    data.insert(Bytes::from_static(b"k1"), Entry::new(Bytes::from_static(b"v1"), None));
    data.insert(Bytes::from_static(b"k2"), Entry::new(Bytes::from_static(b"v2"), None));

    snap.write(&data).await.unwrap();
    let back = snap.read().await.unwrap().unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back.get(&Bytes::from_static(b"k1")).unwrap().value, Bytes::from_static(b"v1"));
}

#[tokio::test]
async fn missing_file_returns_none() {
    let dir = tempdir().unwrap();
    let snap = Snapshot::new(dir.path().join("nope.rdb"));
    assert!(snap.read().await.unwrap().is_none());
}
```

- [ ] **Step 4: Run + commit**

```bash
cargo test -p persistence
git add crates/persistence/
git commit -m "feat(persistence): atomic snapshot via rename + tempfile-based tests"
```

### Task 4.3: Wire persistence into Server

**Files:**
- Modify: `crates/server/src/server.rs`
- Modify: `crates/server/src/conn.rs`
- Modify: `crates/server/src/bin/mini-redis.rs`
- Modify: `crates/server/Cargo.toml` (add persistence dep)

- [ ] **Step 1: Add persistence to server deps**

In `crates/server/Cargo.toml` `[dependencies]` add:
```toml
persistence = { path = "../persistence" }
```

- [ ] **Step 2: Pass a `Wal` to `handle_connection`**

Update `crates/server/src/conn.rs`:
```rust
use bytes::BytesMut;
use command::parse_command;
use persistence::{Wal, WalRecord};
use protocol::{encode, parse, RespError, RespValue};
use std::sync::Arc;
use store::Store;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn handle_connection(mut stream: TcpStream, store: Arc<Store>, wal: Arc<Wal>) -> std::io::Result<()> {
    let mut buf = BytesMut::with_capacity(4096);
    loop {
        match parse(&mut buf) {
            Ok(value) => {
                let response = match parse_command(value) {
                    Ok(cmd) => {
                        if let Some(rec) = WalRecord::from_command(&cmd) {
                            wal.append(&rec).await?;
                        }
                        store.apply(&cmd).await
                    }
                    Err(e) => RespValue::error(format!("ERR {}", e)),
                };
                stream.write_all(&encode(&response)).await?;
            }
            Err(RespError::Incomplete) => {
                let n = stream.read_buf(&mut buf).await?;
                if n == 0 { return Ok(()); }
            }
            Err(other) => {
                let err = RespValue::error(format!("ERR {}", other));
                stream.write_all(&encode(&err)).await?;
                buf.clear();
            }
        }
    }
}
```

- [ ] **Step 3: Build the server with persistence + recovery**

Update `crates/server/src/server.rs`:
```rust
use crate::conn::handle_connection;
use persistence::{Snapshot, Wal, WalRecord};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use store::Store;
use tokio::net::TcpListener;

pub struct Server {
    port: u16,
    data_dir: PathBuf,
    fsync_every: bool,
}

impl Server {
    pub fn new(port: u16, data_dir: PathBuf, fsync_every: bool) -> Self {
        Self { port, data_dir, fsync_every }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.data_dir).await?;
        let snap_path = self.data_dir.join("snapshot.rdb");
        let wal_path = self.data_dir.join("wal.log");

        // 1. Load snapshot if present.
        let store = Store::new();
        let snap = Snapshot::new(&snap_path);
        if let Some(data) = snap.read().await? {
            tracing::info!(keys = data.len(), "loaded snapshot");
            store.load(data).await;
        }

        // 2. Replay WAL.
        let wal = Arc::new(Wal::open(&wal_path, self.fsync_every).await?);
        let records = wal.replay().await?;
        if !records.is_empty() {
            tracing::info!(records = records.len(), "replaying WAL");
            for r in &records {
                store.apply(&r.to_command()).await;
            }
        }

        // 3. Start TCP listener.
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!(%addr, "listening");

        // 4. Background TTL sweep.
        let sweep_store = store.clone();
        tokio::spawn(async move {
            let mut t = tokio::time::interval(Duration::from_secs(5));
            loop { t.tick().await; sweep_store.sweep_expired().await; }
        });

        // 5. Periodic snapshot every 5 min.
        let snap_store = store.clone();
        let snap_for_task = Snapshot::new(&snap_path);
        let wal_for_task = wal.clone();
        tokio::spawn(async move {
            let mut t = tokio::time::interval(Duration::from_secs(300));
            t.tick().await; // skip first immediate tick
            loop {
                t.tick().await;
                let data = snap_store.snapshot().await;
                if let Err(e) = snap_for_task.write(&data).await { tracing::warn!(error = %e, "snapshot failed"); continue; }
                if let Err(e) = wal_for_task.truncate().await { tracing::warn!(error = %e, "wal truncate failed"); }
                tracing::info!("snapshot + wal truncate complete");
            }
        });

        // 6. Accept loop.
        loop {
            let (stream, peer) = listener.accept().await?;
            tracing::debug!(?peer, "accepted");
            let store = store.clone();
            let wal = wal.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, store, wal).await {
                    tracing::warn!(error = %e, "connection error");
                }
            });
        }
    }
}
```

You will hit a compile error: `store.clone()` doesn't exist on `Arc<Store>`... wait, it does — Arc is Clone. But you'll need to verify the receiver. Adjust if needed.

- [ ] **Step 4: Update binary**

`crates/server/src/bin/mini-redis.rs`:
```rust
use clap::Parser;
use server::Server;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mini-redis", version)]
struct Args {
    #[arg(short, long, default_value_t = 6380)]
    port: u16,
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,
    #[arg(long)]
    replicaof: Option<String>,
    #[arg(long)]
    fsync_every_write: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args = Args::parse();
    Server::new(args.port, args.data_dir, args.fsync_every_write).run().await
}
```

- [ ] **Step 5: Manual smoke test of crash recovery**

```bash
rm -rf ./data
cargo run --bin mini-redis -- --data-dir ./data &
SERVER_PID=$!
sleep 1
redis-cli -p 6380 SET foo bar
redis-cli -p 6380 SET n 42
redis-cli -p 6380 INCR n
kill -9 $SERVER_PID
sleep 1
cargo run --bin mini-redis -- --data-dir ./data &
SERVER_PID=$!
sleep 1
redis-cli -p 6380 GET foo
# expect: "bar"
redis-cli -p 6380 GET n
# expect: "43"
kill $SERVER_PID
rm -rf ./data
```

- [ ] **Step 6: Commit**

```bash
git add crates/server/ crates/persistence/
git commit -m "feat(server): wire WAL + snapshot + recovery into server boot"
```

### Task 4.4: Automated crash-recovery integration test

**Files:**
- Create: `crates/integration_tests/tests/e2e_crash_recovery.rs`

- [ ] **Step 1: Write the test**

`crates/integration_tests/tests/e2e_crash_recovery.rs`:
```rust
use std::process::{Command, Stdio};
use std::time::Duration;

fn wait_for_port(port: u16, t: Duration) -> bool {
    let dl = std::time::Instant::now() + t;
    while std::time::Instant::now() < dl {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() { return true; }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn cli(port: u16, args: &[&str]) -> String {
    let out = Command::new("redis-cli").args(["-p", &port.to_string()]).args(args).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn data_survives_kill_minus_9() {
    let dir = tempfile::tempdir().unwrap();

    Command::new(env!("CARGO")).args(["build", "--bin", "mini-redis"]).status().unwrap();
    let bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap()
        .join("target/debug/mini-redis");

    let mut child = Command::new(&bin)
        .args(["--port", "16381", "--data-dir"]).arg(dir.path())
        .stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().unwrap();
    assert!(wait_for_port(16381, Duration::from_secs(5)));

    assert_eq!(cli(16381, &["SET", "foo", "bar"]), "OK");
    assert_eq!(cli(16381, &["INCR", "n"]), "1");
    assert_eq!(cli(16381, &["INCR", "n"]), "2");

    // Hard kill.
    unsafe { libc::kill(child.id() as i32, libc::SIGKILL); }
    let _ = child.wait();

    // Restart.
    let mut child2 = Command::new(&bin)
        .args(["--port", "16381", "--data-dir"]).arg(dir.path())
        .stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().unwrap();
    assert!(wait_for_port(16381, Duration::from_secs(5)));

    assert_eq!(cli(16381, &["GET", "foo"]), "bar");
    assert_eq!(cli(16381, &["GET", "n"]), "2");

    child2.kill().unwrap();
    let _ = child2.wait();
}
```

Add `libc = "0.2"` to `crates/integration_tests/Cargo.toml` `[dev-dependencies]`.

- [ ] **Step 2: Run + commit**

```bash
cargo test -p integration_tests --test e2e_crash_recovery -- --test-threads=1
git add crates/integration_tests/
git commit -m "test: end-to-end crash recovery via kill -9 + restart"
git push
```

---

## Phase 5: Replication (Days 9–10)

**Goal:** Spin up 1 leader + 2 replicas; writes to the leader appear on replicas within 500 ms.

### Task 5.1: Replication: leader broadcast channel

**Files:**
- Modify: `crates/replication/Cargo.toml`
- Create: `crates/replication/src/lib.rs`
- Create: `crates/replication/src/leader.rs`

- [ ] **Step 1: Update deps**

```toml
[package]
name = "replication"
version.workspace = true
edition.workspace = true

[dependencies]
protocol = { path = "../protocol" }
command = { path = "../command" }
store = { path = "../store" }
persistence = { path = "../persistence" }
tokio = { workspace = true }
bytes = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
bincode = { workspace = true }
```

- [ ] **Step 2: Implement the leader-side broadcaster**

`crates/replication/src/leader.rs`:
```rust
use command::Command;
use persistence::WalRecord;
use std::sync::Arc;
use store::Store;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use bincode;

/// The leader broadcasts WalRecord events to replicas via this channel.
pub type ReplStream = broadcast::Sender<WalRecord>;

pub fn channel() -> ReplStream { broadcast::channel(1024).0 }

/// Notify replicas of a committed mutating command. No-op for read-only.
pub fn broadcast_command(tx: &ReplStream, cmd: &Command) {
    if let Some(rec) = WalRecord::from_command(cmd) {
        let _ = tx.send(rec);
    }
}

/// Run the replication listener on a separate port for replicas to connect.
pub async fn run_replication_listener(addr: &str, tx: ReplStream, store: Arc<Store>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "replication listener up");
    loop {
        let (stream, peer) = listener.accept().await?;
        tracing::info!(?peer, "replica connected");
        let rx = tx.subscribe();
        let store = store.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_replica(stream, rx, store).await {
                tracing::warn!(error = %e, "replica conn failed");
            }
        });
    }
}

async fn handle_replica(
    mut stream: TcpStream,
    mut rx: broadcast::Receiver<WalRecord>,
    store: Arc<Store>,
) -> anyhow::Result<()> {
    // 1. Send snapshot.
    let snapshot = store.snapshot().await;
    let serializable: std::collections::HashMap<Vec<u8>, store::Entry> =
        snapshot.iter().map(|(k, v)| (k.to_vec(), v.clone())).collect();
    let snap_bytes = bincode::serialize(&serializable)?;
    stream.write_all(&(snap_bytes.len() as u32).to_be_bytes()).await?;
    stream.write_all(&snap_bytes).await?;
    stream.flush().await?;
    tracing::info!(bytes = snap_bytes.len(), "snapshot sent to replica");

    // 2. Stream subsequent commands.
    loop {
        let rec = rx.recv().await?;
        let bytes = bincode::serialize(&rec)?;
        stream.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
        stream.write_all(&bytes).await?;
        stream.flush().await?;
    }
}
```

- [ ] **Step 3: Library entrypoint (leader only — follower added in 5.2)**

`crates/replication/src/lib.rs`:
```rust
pub mod leader;

pub use leader::{channel, broadcast_command, run_replication_listener, ReplStream};
```

Don't try to build yet — the server crate doesn't import replication until Task 5.3.

### Task 5.2: Replication: follower (replica) side

**Files:**
- Create: `crates/replication/src/follower.rs`

- [ ] **Step 1: Update lib.rs to expose follower**

Add to `crates/replication/src/lib.rs`:
```rust
pub mod leader;
pub mod follower;

pub use leader::{channel, broadcast_command, run_replication_listener, ReplStream};
pub use follower::run_replica;
```

- [ ] **Step 2: Implement the follower**

`crates/replication/src/follower.rs`:
```rust
use bytes::Bytes;
use persistence::WalRecord;
use std::sync::Arc;
use store::{Entry, Store};
use tokio::io::{AsyncReadExt};
use tokio::net::TcpStream;

pub async fn run_replica(leader_addr: &str, store: Arc<Store>) -> anyhow::Result<()> {
    tracing::info!(%leader_addr, "connecting to leader for replication");
    let mut stream = TcpStream::connect(leader_addr).await?;

    // 1. Read snapshot.
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let snap_len = u32::from_be_bytes(len_buf) as usize;
    let mut snap_bytes = vec![0u8; snap_len];
    stream.read_exact(&mut snap_bytes).await?;
    let snap: std::collections::HashMap<Vec<u8>, Entry> = bincode::deserialize(&snap_bytes)?;
    let bytes_map: std::collections::HashMap<Bytes, Entry> =
        snap.into_iter().map(|(k, v)| (Bytes::from(k), v)).collect();
    let keys = bytes_map.len();
    store.load(bytes_map).await;
    tracing::info!(keys, "replica loaded snapshot");

    // 2. Loop on records.
    loop {
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).await.is_err() {
            tracing::warn!("leader connection closed");
            return Ok(());
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload).await?;
        let rec: WalRecord = bincode::deserialize(&payload)?;
        let cmd = rec.to_command();
        store.apply(&cmd).await;
    }
}
```

### Task 5.3: Wire replication into the server

**Files:**
- Modify: `crates/server/Cargo.toml` (add replication)
- Modify: `crates/server/src/server.rs`
- Modify: `crates/server/src/conn.rs`
- Modify: `crates/server/src/bin/mini-redis.rs`

- [ ] **Step 1: Add replication dep**

In `crates/server/Cargo.toml`:
```toml
replication = { path = "../replication" }
```

- [ ] **Step 2: Update conn handler to broadcast**

Update `crates/server/src/conn.rs` to take a `repl_tx: Option<replication::ReplStream>`:

```rust
use bytes::BytesMut;
use command::parse_command;
use persistence::{Wal, WalRecord};
use protocol::{encode, parse, RespError, RespValue};
use replication::ReplStream;
use std::sync::Arc;
use store::Store;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn handle_connection(
    mut stream: TcpStream,
    store: Arc<Store>,
    wal: Arc<Wal>,
    repl_tx: Option<ReplStream>,
    read_only: bool,
) -> std::io::Result<()> {
    let mut buf = BytesMut::with_capacity(4096);
    loop {
        match parse(&mut buf) {
            Ok(value) => {
                let response = match parse_command(value) {
                    Ok(cmd) => {
                        if read_only && WalRecord::from_command(&cmd).is_some() {
                            RespValue::error("READONLY You can't write against a read only replica.")
                        } else {
                            if let Some(rec) = WalRecord::from_command(&cmd) {
                                wal.append(&rec).await?;
                                if let Some(tx) = &repl_tx { let _ = tx.send(rec); }
                            }
                            store.apply(&cmd).await
                        }
                    }
                    Err(e) => RespValue::error(format!("ERR {}", e)),
                };
                stream.write_all(&encode(&response)).await?;
            }
            Err(RespError::Incomplete) => {
                let n = stream.read_buf(&mut buf).await?;
                if n == 0 { return Ok(()); }
            }
            Err(other) => {
                let err = RespValue::error(format!("ERR {}", other));
                stream.write_all(&encode(&err)).await?;
                buf.clear();
            }
        }
    }
}
```

- [ ] **Step 3: Update `Server` to manage role**

In `crates/server/src/server.rs`, add config + behavior:

```rust
use crate::conn::handle_connection;
use persistence::{Snapshot, Wal};
use replication::{channel, run_replica, run_replication_listener, ReplStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use store::Store;
use tokio::net::TcpListener;

pub struct Server {
    pub port: u16,
    pub repl_port: u16,
    pub data_dir: PathBuf,
    pub fsync_every: bool,
    pub replicaof: Option<String>,
}

impl Server {
    pub async fn run(self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.data_dir).await?;
        let snap = Snapshot::new(self.data_dir.join("snapshot.rdb"));
        let wal = Arc::new(Wal::open(self.data_dir.join("wal.log"), self.fsync_every).await?);

        let store = Store::new();
        if let Some(d) = snap.read().await? { store.load(d).await; }
        for r in wal.replay().await? { store.apply(&r.to_command()).await; }

        let read_only = self.replicaof.is_some();

        // If we're a replica, kick off the follower task.
        if let Some(leader) = &self.replicaof {
            let s = store.clone();
            let leader_addr = leader.clone();
            tokio::spawn(async move {
                if let Err(e) = run_replica(&leader_addr, s).await {
                    tracing::error!(error = %e, "replica task exited");
                }
            });
        }

        // If we're a leader, kick off the replication listener.
        let repl_tx: Option<ReplStream> = if !read_only {
            let tx = channel();
            let s = store.clone();
            let addr = format!("0.0.0.0:{}", self.repl_port);
            let tx_for_task = tx.clone();
            tokio::spawn(async move {
                if let Err(e) = run_replication_listener(&addr, tx_for_task, s).await {
                    tracing::error!(error = %e, "replication listener exited");
                }
            });
            Some(tx)
        } else {
            None
        };

        // TTL sweep.
        let s = store.clone();
        tokio::spawn(async move {
            let mut t = tokio::time::interval(Duration::from_secs(5));
            loop { t.tick().await; s.sweep_expired().await; }
        });

        // Periodic snapshot (leader only).
        if !read_only {
            let snap_store = store.clone();
            let snap_path = self.data_dir.join("snapshot.rdb");
            let wal2 = wal.clone();
            tokio::spawn(async move {
                let mut t = tokio::time::interval(Duration::from_secs(300));
                t.tick().await;
                let snap = Snapshot::new(&snap_path);
                loop {
                    t.tick().await;
                    let data = snap_store.snapshot().await;
                    if snap.write(&data).await.is_ok() { let _ = wal2.truncate().await; }
                }
            });
        }

        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!(%addr, role = if read_only { "replica" } else { "leader" }, "listening");

        loop {
            let (s, _) = listener.accept().await?;
            let store = store.clone();
            let wal = wal.clone();
            let tx = repl_tx.clone();
            tokio::spawn(async move {
                let _ = handle_connection(s, store, wal, tx, read_only).await;
            });
        }
    }
}
```

- [ ] **Step 4: Update binary CLI**

```rust
use clap::Parser;
use server::Server;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mini-redis", version)]
struct Args {
    #[arg(short, long, default_value_t = 6380)]
    port: u16,
    #[arg(long, default_value_t = 6390)]
    repl_port: u16,
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,
    #[arg(long)]
    replicaof: Option<String>,
    #[arg(long)]
    fsync_every_write: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args = Args::parse();
    Server { port: args.port, repl_port: args.repl_port, data_dir: args.data_dir, fsync_every: args.fsync_every_write, replicaof: args.replicaof }.run().await
}
```

- [ ] **Step 5: Manual smoke test**

In terminal 1 (leader):
```bash
rm -rf /tmp/leader; cargo run --bin mini-redis -- --port 6380 --repl-port 6390 --data-dir /tmp/leader
```

Terminal 2 (replica):
```bash
rm -rf /tmp/r1; cargo run --bin mini-redis -- --port 6381 --data-dir /tmp/r1 --replicaof 127.0.0.1:6390
```

Terminal 3 (client):
```bash
redis-cli -p 6380 SET foo bar     # to leader
sleep 0.5
redis-cli -p 6381 GET foo         # from replica, expect "bar"
redis-cli -p 6381 SET nope x      # expect READONLY error
```

- [ ] **Step 6: Commit**

```bash
git add crates/
git commit -m "feat(replication): leader-replica streaming via broadcast + snapshot sync"
```

### Task 5.4: Replication integration test

**Files:**
- Create: `crates/integration_tests/tests/e2e_replication.rs`

- [ ] **Step 1: Write the test**

`crates/integration_tests/tests/e2e_replication.rs`:
```rust
use std::process::{Child, Command, Stdio};
use std::time::Duration;

fn wait_for_port(port: u16, t: Duration) -> bool {
    let dl = std::time::Instant::now() + t;
    while std::time::Instant::now() < dl {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() { return true; }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn cli(port: u16, args: &[&str]) -> String {
    let out = Command::new("redis-cli").args(["-p", &port.to_string()]).args(args).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn spawn(bin: &std::path::Path, args: &[&str]) -> Child {
    Command::new(bin).args(args).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap()
}

#[test]
fn writes_to_leader_appear_on_two_replicas() {
    let leader_dir = tempfile::tempdir().unwrap();
    let r1_dir = tempfile::tempdir().unwrap();
    let r2_dir = tempfile::tempdir().unwrap();

    Command::new(env!("CARGO")).args(["build", "--bin", "mini-redis"]).status().unwrap();
    let bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap()
        .join("target/debug/mini-redis");

    let mut leader = spawn(&bin, &[
        "--port", "26380", "--repl-port", "26390",
        "--data-dir", leader_dir.path().to_str().unwrap(),
    ]);
    assert!(wait_for_port(26380, Duration::from_secs(5)));
    assert!(wait_for_port(26390, Duration::from_secs(5)));

    let mut r1 = spawn(&bin, &[
        "--port", "26381", "--data-dir", r1_dir.path().to_str().unwrap(),
        "--replicaof", "127.0.0.1:26390",
    ]);
    let mut r2 = spawn(&bin, &[
        "--port", "26382", "--data-dir", r2_dir.path().to_str().unwrap(),
        "--replicaof", "127.0.0.1:26390",
    ]);
    assert!(wait_for_port(26381, Duration::from_secs(5)));
    assert!(wait_for_port(26382, Duration::from_secs(5)));

    // Wait a moment for snapshot to arrive on replicas.
    std::thread::sleep(Duration::from_millis(500));

    assert_eq!(cli(26380, &["SET", "k1", "v1"]), "OK");
    assert_eq!(cli(26380, &["SET", "k2", "v2"]), "OK");
    assert_eq!(cli(26380, &["INCR", "n"]), "1");

    // Allow replication to catch up.
    std::thread::sleep(Duration::from_millis(500));

    assert_eq!(cli(26381, &["GET", "k1"]), "v1");
    assert_eq!(cli(26381, &["GET", "n"]), "1");
    assert_eq!(cli(26382, &["GET", "k2"]), "v2");

    // Writes to replicas rejected.
    let out = cli(26382, &["SET", "rejected", "x"]);
    assert!(out.contains("READONLY"), "expected READONLY error, got: {out}");

    let _ = leader.kill(); let _ = leader.wait();
    let _ = r1.kill(); let _ = r1.wait();
    let _ = r2.kill(); let _ = r2.wait();
}
```

- [ ] **Step 2: Run + commit**

```bash
cargo test -p integration_tests --test e2e_replication -- --test-threads=1
git add crates/integration_tests/
git commit -m "test: end-to-end 1 leader + 2 replicas convergence test"
git push
```

---

## Phase 6: Benchmarks, README, Blog, Release (Days 11–14)

**Goal:** Ship a polished v0.1.0 release that recruiters will actually find impressive.

### Task 6.1: Criterion micro-benchmarks

**Files:**
- Create: `crates/protocol/benches/encode_decode.rs`
- Modify: `crates/protocol/Cargo.toml`

- [ ] **Step 1: Add criterion dev-dep**

In `crates/protocol/Cargo.toml`:
```toml
[dev-dependencies]
proptest = { workspace = true }
criterion = { workspace = true }

[[bench]]
name = "encode_decode"
harness = false
```

- [ ] **Step 2: Write the bench**

`crates/protocol/benches/encode_decode.rs`:
```rust
use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use protocol::{encode, parse, RespValue};

fn bench_encode(c: &mut Criterion) {
    let v = RespValue::Array(Some(vec![
        RespValue::BulkString(Some(Bytes::from_static(b"SET"))),
        RespValue::BulkString(Some(Bytes::from_static(b"user:42:name"))),
        RespValue::BulkString(Some(Bytes::from_static(b"Nivaan Gupta"))),
    ]));
    c.bench_function("encode_set", |b| b.iter(|| {
        let _ = black_box(encode(black_box(&v)));
    }));
}

fn bench_decode(c: &mut Criterion) {
    let bytes = encode(&RespValue::Array(Some(vec![
        RespValue::BulkString(Some(Bytes::from_static(b"SET"))),
        RespValue::BulkString(Some(Bytes::from_static(b"k"))),
        RespValue::BulkString(Some(Bytes::from_static(b"v"))),
    ])));
    c.bench_function("decode_set", |b| b.iter(|| {
        let mut buf = BytesMut::from(&bytes[..]);
        let _ = black_box(parse(&mut buf));
    }));
}

criterion_group!(benches, bench_encode, bench_decode);
criterion_main!(benches);
```

- [ ] **Step 3: Run + commit**

```bash
cargo bench -p protocol
# look at the perf numbers — note them down for the blog post
git add crates/protocol/
git commit -m "bench(protocol): criterion micro-benchmarks for encode/decode"
```

### Task 6.2: Real `redis-benchmark` comparison

**Files:**
- Create: `docs/BENCHMARKS.md`
- Create: `scripts/run-benchmarks.sh`

- [ ] **Step 1: Write the script**

`scripts/run-benchmarks.sh`:
```bash
#!/usr/bin/env bash
# Compare mini-redis-rs vs real Redis on identical workloads.
# Both servers run on different ports. We use redis-benchmark to drive load.
set -euo pipefail

cd "$(dirname "$0")/.."
cargo build --release --bin mini-redis

# Start real Redis on 6379.
redis-server --daemonize yes --port 6379 --logfile /tmp/real-redis.log

# Start mini-redis on 6380.
rm -rf /tmp/mr-bench
./target/release/mini-redis --port 6380 --data-dir /tmp/mr-bench > /tmp/mr.log 2>&1 &
MR_PID=$!
sleep 1

OPS=100000
CLIENTS=50

echo "=== mini-redis-rs ==="
redis-benchmark -p 6380 -n $OPS -c $CLIENTS -t set,get,incr -q | tee /tmp/mr-bench.txt

echo ""
echo "=== real Redis ==="
redis-benchmark -p 6379 -n $OPS -c $CLIENTS -t set,get,incr -q | tee /tmp/real-bench.txt

kill $MR_PID || true
redis-cli -p 6379 SHUTDOWN NOSAVE || true

echo ""
echo "Done. Results in /tmp/mr-bench.txt and /tmp/real-bench.txt."
```

- [ ] **Step 2: Make executable and run**

```bash
chmod +x scripts/run-benchmarks.sh
./scripts/run-benchmarks.sh
```

Note the numbers.

- [ ] **Step 3: Write BENCHMARKS.md**

`docs/BENCHMARKS.md`:
```markdown
# Benchmarks

All benchmarks run on [your laptop spec — fill in], macOS [version].

Workload: `redis-benchmark -n 100000 -c 50 -t set,get,incr -q`

| Operation | mini-redis-rs (ops/s) | real Redis (ops/s) | ratio |
|---|---|---|---|
| SET       | [fill in]              | [fill in]          | [fill in] |
| GET       | [fill in]              | [fill in]          | [fill in] |
| INCR      | [fill in]              | [fill in]          | [fill in] |

## Reproducing

```
./scripts/run-benchmarks.sh
```

## Notes

- mini-redis-rs runs without `fsync_every_write` by default. Enable with `--fsync-every-write` for stronger durability at lower throughput.
- Real Redis is single-threaded for the data path; we use `tokio::sync::RwLock<HashMap>` which is also effectively single-writer.
- We're not trying to beat Redis. Real Redis is 15 years of optimization; we built ours in 2 weeks. The point is to be in the same order of magnitude on basic ops.
```

Fill in the real numbers from your run.

- [ ] **Step 4: Commit**

```bash
git add docs/BENCHMARKS.md scripts/
git commit -m "bench: redis-benchmark comparison vs real Redis"
```

### Task 6.3: Showcase README

**Files:**
- Modify: `README.md` (replace entirely)

- [ ] **Step 1: Write the README**

```markdown
# mini-redis-rs

> A Redis-protocol-compatible key-value store in Rust — with WAL persistence, snapshots, and single-leader replication. Built in 2 weeks as a learning exercise in distributed systems.

[![CI](https://github.com/nivaan-gupta/mini-redis-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/nivaan-gupta/mini-redis-rs/actions)

```bash
# Start a server
$ cargo run --release --bin mini-redis -- --port 6380

# Talk to it with the real Redis CLI
$ redis-cli -p 6380 SET hello world
OK
$ redis-cli -p 6380 GET hello
"world"

# Run real Redis benchmark against it
$ redis-benchmark -p 6380 -n 100000 -t set,get -q
SET: [XX,XXX].XX requests per second
GET: [XX,XXX].XX requests per second
```

## What it does

- ✅ **Redis-protocol compatible (RESP2)** — `redis-cli`, `redis-benchmark`, and any RESP client work out of the box
- ✅ **Persistence** — append-only WAL + periodic snapshots; survives `kill -9`
- ✅ **Single-leader replication** — leader broadcasts to N read-only replicas
- ✅ **TTL** — lazy + active expiration
- ❌ Cluster mode, pub/sub, transactions, complex types — out of scope for v0.1

## Architecture

```
                ┌──────────────────────────────────────────┐
                │           Mini-Redis Process             │
                │                                          │
   ┌─────────┐  │  ┌──────────────┐    ┌────────────────┐  │
   │ redis-  │──┼─▶│  Connection  │───▶│    Command     │  │
   │  cli    │  │  │  Manager     │    │    Executor    │  │
   │ (or any │◀─┼──│  (tokio task │◀───│                │  │
   │  RESP)  │  │  │   per conn)  │    └────────┬───────┘  │
   └─────────┘  │  └──────────────┘             │          │
                │                               ▼          │
                │                        ┌──────────────┐  │
                │                        │    Store     │  │
                │                        │  (RwLock<    │  │
                │                        │  HashMap>)   │  │
                │                        └──────┬───────┘  │
                │                               │          │
                │              ┌────────────────┼──────────┤
                │              ▼                ▼          │
                │      ┌──────────────┐  ┌──────────────┐  │
                │      │  Persister   │  │ Replicator   │──┼─▶ replica conns
                │      │  WAL +       │  │  (broadcast  │  │
                │      │  Snapshot    │  │   channel)   │  │
                │      └──────┬───────┘  └──────────────┘  │
                │             ▼                            │
                │       wal.log / snapshot.rdb             │
                └──────────────────────────────────────────┘
```

## Commands supported

| Group | Commands |
|---|---|
| Connection | PING, ECHO |
| KV         | GET, SET (EX, PX, NX, XX), DEL, EXISTS |
| Counters   | INCR, DECR |
| TTL        | EXPIRE, TTL |
| Discovery  | KEYS, INFO |
| Replication | REPLICAOF |

## Running a replicated cluster

```bash
# Leader on 6380 (replication listener on 6390)
mini-redis --port 6380 --repl-port 6390 --data-dir /tmp/leader

# Replica
mini-redis --port 6381 --data-dir /tmp/r1 --replicaof 127.0.0.1:6390

# Writes go to leader, reads work on either
redis-cli -p 6380 SET foo bar
redis-cli -p 6381 GET foo   # "bar"
```

## Benchmarks

See [docs/BENCHMARKS.md](docs/BENCHMARKS.md).

## Project layout

```
crates/
├── protocol/       # RESP2 encode/decode
├── command/        # typed commands
├── store/          # in-memory + TTL
├── persistence/    # WAL + snapshot
├── replication/    # leader-replica streaming
├── server/         # connection + executor + glue + binary
└── integration_tests/
```

## Build & test

```bash
cargo test --workspace      # 50+ unit + integration tests
cargo clippy --workspace -- -D warnings
cargo bench -p protocol
```

## What I learned

See the postmortem blog post: [`docs/blog/lessons.md`](docs/blog/lessons.md).

## License

MIT.
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: showcase README with quickstart, architecture, commands"
git push
```

### Task 6.4: Blog post

**Files:**
- Create: `docs/blog/lessons.md`

- [ ] **Step 1: Outline the blog post**

`docs/blog/lessons.md` — write a 1500–2500 word post structured as:

1. **Why I built this** (1 paragraph) — wanted to understand the bottom of the stack; chose a project I could finish in 2 weeks; targeted Redis because of the obvious wow-factor demo
2. **Three things that surprised me** — pick three actual surprises from your 14 days. Examples:
   - "RESP looks trivial. It isn't — binary-safe bulk strings + null arrays + pipelining make the parser harder than I expected."
   - "Async Rust is harder than sync Rust. Lifetimes inside async blocks broke my brain on day 3. Here's the specific gotcha and how I fixed it."
   - "WAL durability is a knob, not a binary. `flush()` vs `fsync()` is a 50× performance difference and I had to think hard about which one is the default."
3. **What I'd do differently** (1-2 paragraphs)
4. **Architecture quick-tour** — link to README, paste the diagram
5. **The benchmark numbers** — be honest about being slower than real Redis

Be specific, technical, and honest. No filler.

- [ ] **Step 2: Commit**

```bash
git add docs/blog/lessons.md
git commit -m "docs: postmortem blog post"
```

### Task 6.5: Cut the v0.1.0 release

**Files:** none

- [ ] **Step 1: Tag and release**

```bash
git tag v0.1.0
git push --tags

gh release create v0.1.0 \
  --title "mini-redis-rs v0.1.0" \
  --notes "First release. Redis-protocol-compatible KV store with WAL persistence and single-leader replication.

Built in 2 weeks. Tested with real \`redis-cli\` and \`redis-benchmark\`. See [README](README.md) and [BENCHMARKS](docs/BENCHMARKS.md).

Supported commands: PING, ECHO, GET, SET (EX/PX/NX/XX), DEL, EXISTS, INCR, DECR, EXPIRE, TTL, KEYS, INFO, REPLICAOF."
```

- [ ] **Step 2: Pin the repo on your profile**

Visit https://github.com/nivaan-gupta → "Customize your pins" → add `mini-redis-rs`. Move it to slot 1 if you can.

- [ ] **Step 3: Update your profile README**

Edit `nivaan-gupta/nivaan-gupta` README:
- Add `mini-redis-rs` to the "Solo / Personal Projects" section at the top
- Update bio if needed

```bash
cd ../nivaan-gupta # or wherever your profile repo is
# edit README.md
git commit -am "Feature mini-redis-rs"
git push
```

- [ ] **Step 4: Publish the blog post**

- Upload to dev.to (free): copy `docs/blog/lessons.md`, paste into dev.to editor, publish.
- Link from your profile README.

- [ ] **Step 5: Final celebrate**

You shipped a distributed system in 2 weeks. 🎉

---

## Buffer (Days 13–14, soft)

If anything slipped:

- Day 13: Catch-up day for any incomplete tasks
- Day 14: Polish, fix flaky tests, add to README, push final

If everything's on time, use Days 13–14 for:
- More commands (`MGET`, `MSET`, `SETNX`)
- TLS support (rustls)
- Pub/sub (`SUBSCRIBE`, `PUBLISH`)
- A simple Web UI to inspect the store

But these are post-v0.1.0 stretches. Ship v0.1.0 first.

---

## Done criteria checklist

Check these off before declaring the project done:

- [ ] `cargo test --workspace` is green (target: 50+ tests)
- [ ] `cargo clippy --workspace -- -D warnings` is clean
- [ ] `cargo fmt --all -- --check` passes
- [ ] CI is green on GitHub
- [ ] `redis-cli -p 6380 SET k v && redis-cli -p 6380 GET k` works against your binary
- [ ] `redis-benchmark -p 6380 -t set,get -n 100000 -q` produces numbers
- [ ] `kill -9` + restart preserves all data
- [ ] 1 leader + 2 replicas converge on the same data within 500 ms
- [ ] README has architecture diagram, commands table, quickstart
- [ ] Blog post draft exists in `docs/blog/`
- [ ] `v0.1.0` tag exists on GitHub
- [ ] `mini-redis-rs` is pinned on the profile (replaces a current pin)
