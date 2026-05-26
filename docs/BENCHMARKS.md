# Benchmarks

All benchmarks run on Apple M2 Pro, macOS 26.3.1.

Workload: `redis-benchmark -n 100000 -c 50 -t set,get,incr -q`

| Operation | mini-redis-rs (ops/s) | real Redis (ops/s) | ratio |
|---|---|---|---|
| SET       | 19,692.79             | 92,336.11          | 0.21x |
| GET       | 84,530.86             | 94,876.66          | 0.89x |
| INCR      | 19,747.23             | 96,061.48          | 0.21x |

## Reproducing

```bash
./scripts/run-benchmarks.sh
```

## Notes

- mini-redis-rs runs without `fsync_every_write` by default. Enable with `--fsync-every-write` for stronger durability at lower throughput.
- Real Redis is single-threaded for the data path; we use `tokio::sync::RwLock<HashMap>` which is also effectively single-writer.
- We're not trying to beat Redis. Real Redis is 15+ years of optimization; we built ours in 2 weeks. The point is to be in the same order of magnitude on basic ops.
- SET and INCR are write operations that acquire exclusive locks; they are naturally slower in our RwLock implementation. GET is a read-only operation and performs much better.
