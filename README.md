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
SET: 19,692.79 requests per second
GET: 84,530.86 requests per second
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
cargo test --workspace      # 46+ unit + integration tests
cargo clippy --workspace -- -D warnings
cargo bench -p protocol
```

## What I learned

See the postmortem blog post: [`docs/blog/lessons.md`](docs/blog/lessons.md).

## License

MIT.
