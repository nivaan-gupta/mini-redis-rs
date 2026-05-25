# mini-redis-rs — Design Document

**Status:** Approved, ready for implementation planning
**Author:** Nivaan Gupta
**Date:** 2026-05-25
**Target completion:** 2 weeks intensive (≈14 days)

---

## 1. Goal

Build a Redis-protocol-compatible single-node key-value store in Rust with durable persistence and single-leader replication. The deliverable is a polished open-source repository plus a written-up postmortem suitable for showing to recruiters at infrastructure-heavy companies (Google, Meta, AWS, Cloudflare, etc.).

**Why this project, specifically:**
- Demonstrates the four pillars infra recruiters care about: networking, concurrency, storage, replication
- Has a wow-factor demo: drop-in compatibility with real Redis CLI tools (`redis-cli`, `redis-benchmark`)
- Has measurable outcomes: throughput / latency vs real Redis
- Scope is realistic for 2 weeks intensive while remaining substantial

**Cost:** $0. Local development on macOS. Public GitHub repo. GitHub Actions free tier.

## 2. Success criteria

The project is shipped when **all** of these are true:

1. `redis-cli -p 6380 SET foo bar && redis-cli -p 6380 GET foo` returns `"bar"` against our binary
2. `redis-benchmark -p 6380 -t set,get -n 100000` completes successfully, producing comparable (within an order of magnitude) numbers to real Redis on the same hardware
3. Killing the server process mid-write and restarting recovers all committed data
4. Starting one leader + two replicas, writing to the leader, and reading from a replica returns consistent data within 500 ms
5. CI passes on every push: unit tests, integration tests, `cargo clippy --deny warnings`, `cargo fmt --check`
6. The README contains: architecture diagram, demo GIF, run instructions, benchmark numbers
7. A blog post draft exists in `docs/blog/` documenting at least three non-trivial design decisions

## 3. Scope

### In scope

**Protocol layer**
- RESP2 wire protocol over TCP (`*N\r\n$len\r\nvalue\r\n…` arrays of bulk strings)
- Encoder and decoder with proptest round-trip coverage

**Commands** (String type only)
- Connection: `PING`, `ECHO`
- KV: `GET`, `SET` (with `EX`, `PX`, `NX`, `XX` modifiers), `DEL`, `EXISTS`
- Counters: `INCR`, `DECR`
- TTL: `EXPIRE`, `TTL`
- Discovery: `KEYS <pattern>` (glob-style: `*`, `?`, `[abc]`), `INFO`
- Replication: `REPLICAOF <host> <port>`, `REPLICAOF NO ONE`

**Storage**
- In-memory hash map (`tokio::sync::RwLock<HashMap<String, Entry>>`)
- TTL bookkeeping with lazy expiration on access + active sweep every N seconds

**Persistence**
- Append-only write-ahead log (`wal.log`), durable before client ack
- Periodic snapshot (`snapshot.rdb` — binary dump of current state via `serde` + `bincode`)
- Snapshot triggered by: (a) WAL exceeds 64 MB, or (b) every 5 minutes, whichever first; configurable via CLI
- WAL compaction after snapshot (old WAL truncated to entries after snapshot offset)
- Recovery on startup: load snapshot, then replay WAL

**Replication**
- Single leader, N read replicas
- Replica boots with `--replicaof leader:port`
- Leader sends full snapshot first, then streams committed commands
- Asynchronous (replicas may lag); strong consistency only on the leader

**Tooling**
- `cargo bench` (criterion) for micro-benchmarks
- Integration test that runs `redis-cli` against our binary
- Integration test that runs `redis-benchmark` against our binary
- Integration test that spawns leader + 2 replicas and verifies convergence

### Out of scope (explicitly deferred)

- Cluster mode, sharding, Raft / Paxos consensus
- Pub/sub, Lua scripting (`EVAL`), transactions (`MULTI`/`EXEC`), multi-DB (`SELECT`)
- Complex data types: Lists, Hashes, Sets, Sorted Sets, Streams, Bitmaps, HyperLogLog
- AUTH, ACLs, TLS
- RESP3 protocol
- Disk-backed storage (everything fits in memory in MVP)

## 4. Architecture

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

## 5. Module breakdown

Cargo workspace with one crate per responsibility. Each crate has a clear public API and is unit-testable without spinning up TCP.

| Crate | Public types / responsibility | Dependencies |
|---|---|---|
| `protocol` | `RespValue` enum + `encode(&RespValue) -> Bytes` + `parse(&mut Bytes) -> Result<RespValue>` | `bytes`, `thiserror` |
| `command` | `Command` enum + `Command::from_resp(RespValue) -> Result<Command>` | `protocol` |
| `store` | `Store` struct + `apply(&self, &Command) -> Response`; TTL sweep task | `command`, `tokio` |
| `persistence` | `Wal::append`, `Snapshot::write`, `recover(dir) -> Store` | `command`, `store`, `tokio` |
| `replication` | `LeaderReplicator` (broadcast cmds), `ReplicaClient` (connect + apply) | `command`, `persistence`, `tokio` |
| `connection` | `ConnectionHandler` (per-conn tokio task) | `protocol`, `command` |
| `server` | `Server::bind_and_run(config)`; ties everything together | all of above |
| `bin/mini-redis` | `main()` + CLI flag parsing via `clap` | `server` |

**Why this structure:** each layer can be reviewed and tested independently. `protocol` doesn't know about commands; `store` doesn't know about networking; `persistence` doesn't know about replication. This makes the code interview-presentable and easy for reviewers to read.

## 6. Data flow

### 6.1 Write path (client → leader)

```
1. Client sends RESP bytes over TCP.
2. ConnectionHandler reads bytes (tokio).
3. protocol::parse → RespValue.
4. command::from_resp → typed Command.
5. Executor:
     a. store.apply(&cmd)                    → Response value
     b. wal.append(&cmd).await               ← durable BEFORE client ack
     c. replicator.broadcast(&cmd)           ← non-blocking via channel
6. protocol::encode(response) → bytes → write to client.
```

Durability ordering matters: the WAL append must complete before the client gets an ack. "Complete" means: at minimum, `write()` + `flush()` (data in OS page cache, survives process kill but not power loss). Configurable to `fsync()` per write (survives power loss, ~50× slower). See §7.2. If the process dies between (5a) and (5b), the client will time out and retry — we do NOT lose acknowledged data.

### 6.2 Replication path (leader → replica)

```
1. Replica process starts with `--replicaof leader_host:leader_port`.
2. Replica opens TCP conn to leader, sends RESP: REPLICAOF self_host self_port.
3. Leader treats the conn as a replica subscriber:
     a. Pauses writes briefly, takes a snapshot (or uses latest cached snapshot).
     b. Streams the snapshot binary frame.
     c. Resumes writes; from the snapshot offset forward, streams new commands as RESP frames.
4. Replica:
     a. Receives snapshot → populates local Store.
     b. Continues reading the stream → applies each Command via the same Executor (minus client ack).
```

Replicas serve reads (`GET`, `EXISTS`, `KEYS`, `TTL`). Writes to a replica are rejected with `READONLY` error.

### 6.3 Recovery path (startup)

```
1. Open data directory specified by --data-dir.
2. If snapshot.rdb exists → load it into Store.
3. Replay wal.log starting from the offset stamped in the snapshot header.
4. Open server socket and start accepting.
```

If neither file exists, start empty. If WAL is corrupted partway (truncated mid-record), stop at the last valid record and log a warning.

## 7. Key design decisions

### 7.1 `tokio::sync::RwLock<HashMap>` vs lock-free / sharded

Use a single `RwLock` for MVP. Real Redis is single-threaded for the data path; a single lock is simpler to reason about and benchmark. Document this as a deliberate choice with a "future work: shard the lock by key hash" note in `ARCHITECTURE.md`.

### 7.2 WAL durability model

Default: write WAL record + `flush` (sends to OS buffer; no fsync). Configurable to `fsync_every_write` for stronger durability with predictable performance cost. Document both modes' tradeoffs in README.

### 7.3 RESP2 (not RESP3)

RESP2 is universally supported by every Redis client and tool. RESP3 adds richer types but doubles the protocol scope. Out for MVP.

### 7.4 Asynchronous replication (no quorum)

Leader doesn't wait for replica acks. Trades read-after-write consistency on replicas for write latency. Document this and the implications. (Synchronous replication would require quorum logic that pulls us toward Raft — saved for v2.)

### 7.5 String values only (no Lists/Hashes/Sets)

Each additional type doubles the surface area of the command set. String covers ≥80% of real Redis usage. The architecture is structured so that adding new value types is a `store::Value` enum extension, not a rewrite.

## 8. Testing strategy

| Layer | What's tested | How |
|---|---|---|
| `protocol` | RESP encode → decode round-trips, edge cases (empty arrays, nested, huge bulk) | proptest |
| `command` | Every command parses; malformed RESP rejected with proper error | unit tests |
| `store` | All command semantics; TTL expires correctly (lazy + active); KEYS pattern matching | unit tests |
| `persistence` | Write commands → kill → recover → state identical | integration: spawn subprocess, kill it, restart |
| `replication` | Leader writes → replicas converge within 500 ms | integration: spawn 1 leader + 2 replicas |
| **System** | `redis-cli`, `redis-benchmark`, `redis-rs` client all work | CI smoke tests using real binaries |

CI runs on every push via GitHub Actions (free for public repos). Matrix: stable Rust, Linux + macOS.

## 9. Milestones — 14-day plan

| Days | Goal | Demo at end of segment |
|---|---|---|
| 1–2 | Rust onboarding (book ch. 1-10 + tokio tutorial); workspace scaffold; CI green on empty workspace; CLI parsing | `cargo run -- --help` shows flags |
| 3–4 | RESP2 parser + encoder; proptest round-trips passing | `cargo test -p protocol` ✅ |
| 5–6 | `Store` + commands (`PING`, `GET`, `SET`, `DEL`, `EXISTS`, `INCR`, `DECR`, `EXPIRE`, `TTL`, `KEYS`); `ConnectionHandler`; per-conn tokio task | **`redis-cli` against our server works end-to-end** |
| 7–8 | WAL append + replay; snapshot write/load; compaction; crash-recovery integration test | `kill -9` → restart → all data present |
| 9–10 | Leader-replica replication; 2-replica convergence integration test | Write to leader → read from replica returns correct value |
| 11 | Run `redis-benchmark` against both binaries on identical workloads; record numbers; build comparison chart | `docs/BENCHMARKS.md` with real numbers |
| 12 | README polish: architecture diagrams, demo GIF, badges, quickstart, contributing | Profile-grade README |
| 13 | Blog post draft (`docs/blog/`) covering: Rust ownership pain points encountered, RESP edge cases, WAL durability decisions | Post draft ready for dev.to |
| 14 | Buffer / final polish / GitHub release `v0.1.0` with binaries built by CI | Tagged release |

If any milestone slips by >1 day, drop replication scope (replication is day 9–10) and use that time for polish. Better to ship a polished single-node store than a half-working replicated one.

## 10. Repo structure

```
mini-redis-rs/
├── Cargo.toml                # workspace root
├── README.md                 # the showcase
├── LICENSE                   # MIT
├── .github/workflows/ci.yml  # build + test + clippy + fmt + benchmarks-on-release
├── crates/
│   ├── protocol/             # RESP2 encode/decode
│   ├── command/              # typed Command enum
│   ├── store/                # in-memory + TTL
│   ├── persistence/          # WAL + snapshot + recovery
│   ├── replication/          # leader-replica streaming
│   └── server/               # connection + executor + glue
├── benches/                  # criterion benchmarks
├── tests/                    # integration tests
│   ├── e2e_redis_cli.rs
│   ├── e2e_redis_benchmark.rs
│   ├── e2e_crash_recovery.rs
│   └── e2e_replication.rs
├── examples/                 # example usage
└── docs/
    ├── ARCHITECTURE.md       # diagrams + design rationale
    ├── BENCHMARKS.md         # real perf vs Redis
    └── blog/
        └── lessons.md        # the postmortem blog post
```

## 11. Tooling & dependencies (planned crates)

- `tokio` — async runtime
- `bytes` — efficient byte buffers for RESP parsing
- `clap` — CLI flags
- `serde` + `bincode` — snapshot serialization
- `thiserror` — typed errors
- `tracing` + `tracing-subscriber` — structured logging
- `proptest` — property-based tests
- `criterion` — benchmarks
- `tempfile` — integration test isolation

No paid services. Everything runs locally.

## 12. Open questions / risks

| Risk | Mitigation |
|---|---|
| Rust learning curve eats more than 2 days | Have a fallback Go branch ready conceptually; but commit to Rust on day 1 and grind |
| RESP parsing has subtle edge cases (binary-safe bulk strings, pipelining) | Lean heavily on proptest from day 3 |
| Replication is finicky — partial connection drops, slow replicas | If day 10 is rough, ship without replication; single-node + WAL is still impressive |
| Benchmarks look embarrassing vs real Redis | Frame them honestly: "10× slower but I built it from scratch in 2 weeks" beats hiding numbers |
| File I/O on macOS has different fsync semantics than Linux | Test on both via CI matrix |

## 13. Definition of done

The project is shipped (and ready to show recruiters) when:

- All 7 success criteria from §2 are met
- The repo has ≥80% test coverage (`cargo tarpaulin`)
- The README is the single best 90-second read on the project
- The blog post is publishable (clean prose, real technical content)
- A `v0.1.0` tag exists on GitHub with release notes
- The repo is pinned on `nivaan-gupta`'s GitHub profile (replaces a current pin)

---

*This design is the contract for the implementation plan. The implementation plan in `docs/superpowers/plans/` will translate every IN-scope item into discrete, ordered tasks with concrete file paths and code.*
