# Building mini-redis-rs: Lessons from Two Weeks of Rust and Distributed Systems

I spent two weeks building a Redis-compatible key-value store in Rust. Not because the world needed another KV store (it doesn't), but because I wanted to really understand what makes Redis tick. The project worked out better than I expected—it compiles, it runs, it replicates, and it survives process crashes. Here are three things that genuinely surprised me along the way.

## Why I Built This

I've read dozens of blog posts about distributed systems, watched conference talks about consensus algorithms, and memorized the CAP theorem. But there's a massive gap between reading about something and building it. Redis is the perfect learning project: it's famous enough that I can compare my version to the real thing, simple enough that I could finish it in two weeks, and deep enough that it touches persistence, replication, and network I/O all at once.

The secondary motivation: I wanted to prove something to myself. Rust has a reputation for being hard. I'd written small scripts and toy programs, but never a multi-component system. Two weeks seemed like an impossible deadline. Spoiler: it wasn't.

## Surprise One: RESP Parsing is Harder Than It Looks

When I first read the Redis Serialization Protocol (RESP) spec, I thought it was trivial. It's just text-based with length prefixes:

```
*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n
```

This is the command `GET foo`—two elements in an array. The parser should be a simple state machine, right?

I was wrong. The real difficulty isn't the format; it's the **partial frames**. TCP is a stream, not a message boundary. You might receive `*2\r\n$3\r\nGET\r\n` and then nothing for 100ms while the client is still typing. The parser needs to:

1. Stop partway through a frame and ask "give me more bytes"
2. Not consume those bytes from the buffer until it knows the frame is complete
3. Handle null values (`$-1\r\n`), null arrays (`*-1\r\n`), and arrays with nested arrays
4. Respect that bulk strings are **binary-safe**—they can contain `\r\n` in the middle

The tricky part was the third requirement. My first attempt would get confused by a bulk string like:

```
$7\r\nfoo\r\nbar\r\n
```

This is a 7-byte string containing literal `\r\n`. I naively searched for `\r\n` to find the end of the string, which broke on this case. The fix: read the length first, then read exactly that many bytes, then verify the next bytes are `\r\n`. But I also needed to track a cursor separately from consuming the buffer, so partial frames didn't cause the parser to lose state.

The final decoder uses a cursor parameter to track how far we've read without consuming the buffer, then uses `buf.advance()` from the `bytes` crate only after successfully parsing a complete frame. This is elegant in retrospect, but it took me a day to figure out.

**The lesson:** Wire protocols that look simple have subtle state-management requirements. Test with proptest and arbitrary input, not just happy-path examples.

## Surprise Two: Async Rust Inside Locks is a Footgun

By day 5, I had the store working. It's a wrapped `HashMap<Bytes, Entry>` protected by `tokio::sync::RwLock`. Simple. Then I tried to add replication, which required:

1. Hold a read lock on the store
2. Serialize all entries
3. Send them over the network to replicas

Standard stuff. But I wrote:

```rust
pub async fn snapshot(&self) -> HashMap<Bytes, Entry> {
    self.inner.read().await.clone()
}
```

And then called it from a background task during a write-heavy benchmark. The benchmark numbers cratered—throughput dropped 40%. I spent two hours debugging this.

The problem: the guard from `read().await` stays alive during the clone. For a large store (or during the snapshot test where I deliberately created 10,000 entries), the clone can take milliseconds. During that time, the read lock is held, which blocks writers. This serialized all concurrent writes to the store during snapshots.

The fix is trivial: release the lock before cloning by using a separate scope:

```rust
pub async fn snapshot(&self) -> HashMap<Bytes, Entry> {
    let guard = self.inner.read().await;
    let data = guard.clone();
    drop(guard);  // or: scope it differently
    data
}
```

But this only works if the clone is cheap (cloning 10K Bytes objects is still fast) or if you don't hold the lock. For a more sophisticated system, you'd use a read-copy-update pattern or triple-buffering.

**The lesson:** Rust's async runtime is fantastic, but holding a lock across an `await` point is a subtle performance trap. The lock is held until the await completes, not released when you think. This bit me harder than any borrow checker error.

## Surprise Three: WAL Sync Strategies Have Wild Performance Implications

The write-ahead log is how the store survives crashes. Every write (SET, INCR, DEL, etc.) gets appended to disk before responding to the client. If the process dies, we replay the log on restart.

I coded two strategies:

- `fsync_every = false` (default): After writing to the buffer, call `flush()`. This writes to the OS page cache. Very fast.
- `fsync_every = true`: After writing, call `sync_data()` to force the OS to write to disk. Much slower but stronger durability.

The benchmark numbers shocked me:

| Mode | SET ops/sec | INCR ops/sec |
|------|---|---|
| flush() only | 19,700 | 19,700 |
| sync_data() | ~1,800 | ~1,800 |

That's **a 10× slowdown** for synchronous fsync. On a MacBook Pro with an SSD, fsync still costs ~500 microseconds. With 50 concurrent clients, each thread does fsync on every write, so you're serializing on disk I/O immediately.

Real Redis handles this by allowing the user to choose: SAVE (blocking snapshot + fsync) for safety, BGSAVE (background snapshot with eventual fsync) for speed, or even AOF rewrite strategies. We defaulted to the fast path. This is a real trade-off that businesses argue about.

**The lesson:** Durability is not binary; it's a spectrum. Small performance choices (flush vs fsync) cascade into 10× throughput differences. This is why the configuration space for databases is so large.

## What I'd Do Differently (If I Built v0.2)

1. **Replica reads should have eventual consistency guarantees**. Right now, replicas are read-only true read-replicas, but they don't expose any API to tell clients "this data might be 100ms stale." I'd add a timestamp to snapshots and let clients check how far behind a replica is.

2. **Use a faster serde for the WAL**. Bincode is fine, but it's not the fastest. For a 10M-entry snapshot, choosing a faster serializer would help. This is micro-optimization, but in the persistence layer it matters.

3. **Implement EXPIRE as a separate data structure**. Right now, I store `expires_at: Option<u64>` in each Entry. This is simple but wastes memory for entries without TTL. A separate sorted map of (expiration_time, key) would be more elegant and allow range-deletion of expired entries in one pass during garbage collection.

4. **Test cluster membership changes**. The current design assumes the replica set is static. Real systems need to handle replicas joining and leaving. This is hard; I'm glad I didn't try.

## The Build

The project is 6 Cargo crates layered like a stack:

- `protocol`: RESP2 encode/decode (470 lines)
- `command`: Typed command parsing (130 lines)
- `store`: In-memory KV store with TTL (240 lines)
- `persistence`: WAL + snapshots (380 lines)
- `replication`: Leader-replica streaming (210 lines)
- `server`: TCP listener + connection handler (200 lines)

Total: ~1,600 lines of production code. The test suite is 46 passing tests covering unit tests, property tests, and end-to-end integration tests (process spawning, crash recovery, replication).

The CI runs on Ubuntu and macOS automatically. The benchmarks show we get 84k ops/sec for read-only workloads (GET) and 20k ops/sec for writes (SET, INCR) on unoptimized release builds. We're not trying to beat Redis (it gets 90k-100k ops/sec), just to be in the same ballpark and prove the architecture works.

## Real Benchmarks

Running `redis-benchmark -n 100000 -c 50` on an Apple M2 Pro:

**mini-redis-rs:**
- SET: 19,693 ops/sec
- GET: 84,531 ops/sec
- INCR: 19,747 ops/sec

**Real Redis:**
- SET: 92,336 ops/sec
- GET: 94,877 ops/sec
- INCR: 96,061 ops/sec

We're 4-5× slower on write-heavy workloads (due to the RwLock contention on writes) and almost at parity on read-only workloads. For a two-week project, this feels like a win.

## Final Thoughts

The hardest part wasn't the Rust syntax or async runtime. It was the mental model—keeping the whole system in my head at once. Between the RESP parser, the store concurrency semantics, the WAL crash recovery, and the replication protocol, there were 5-6 moving pieces. One tiny bug in the replication snapshot transfer could cause data divergence across replicas, and that bug would only show up in a specific timing window during the integration test.

This is why distributed systems are hard, and why Redis (a 15-year-old project with thousands of users) is so much better than what I built. But that doesn't diminish the value of building it myself. I now understand the shape of the problem in a way reading papers never taught me.

If you want to learn distributed systems, pick a small project, set a deadline, and build it. Don't aim for production-grade correctness; aim for working correctness. Understand the tradeoffs. Make benchmarks. Write tests. Then reflect on what surprised you.

That's the real lesson.
