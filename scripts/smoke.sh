#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo ">> build"
cargo build -p server -p cli --quiet

echo ">> wipe data + start server"
rm -rf data
LINGYUAN_TICK_MS=500 ./target/debug/server > /tmp/lingyuan-smoke.log 2>&1 &
PID=$!
trap "kill $PID 2>/dev/null || true" EXIT

# 等服务起来
for i in $(seq 1 30); do
  if curl -fs http://127.0.0.1:7777/health > /dev/null 2>&1; then
    echo "server up after ${i}*200ms"
    break
  fi
  sleep 0.2
done

export LINGYUAN_TOKEN_PATH=/tmp/lingyuan-smoke-token.json

echo ">> clear + join + observe"
./target/debug/survivor clear || true
./target/debug/survivor join --name alice --server http://localhost:7777
echo "--- observe ---"
./target/debug/survivor observe --format markdown

echo ">> act move (test 4 directions until one walkable)"
for d in north east south west; do
  if ./target/debug/survivor act move --dir=$d 2>/dev/null | grep -q queued_for_tick; then
    echo "moved $d"
    break
  fi
done

sleep 1.2
echo "--- observe after move ---"
./target/debug/survivor observe --format markdown

echo ">> leave"
./target/debug/survivor leave

echo ">> verify persistence"
ls -l data/world.db
echo "events: $(sqlite3 data/world.db 'SELECT COUNT(*) FROM events')"
echo "snapshots: $(sqlite3 data/world.db 'SELECT COUNT(*) FROM snapshots')"
echo "agents_meta: $(sqlite3 data/world.db 'SELECT COUNT(*) FROM agents_meta')"

echo "✅ smoke ok"
