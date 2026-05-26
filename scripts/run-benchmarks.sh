#!/usr/bin/env bash
# Compare mini-redis-rs vs real Redis on identical workloads.
# Both servers run on different ports. We use redis-benchmark to drive load.
set -euo pipefail

cd "$(dirname "$0")/.."
source "$HOME/.cargo/env"

cargo build --release --bin mini-redis

# Kill any existing redis-server or mini-redis to clean up
pkill redis-server || true
pkill mini-redis || true
sleep 1

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
