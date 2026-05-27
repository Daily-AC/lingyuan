#!/usr/bin/env bash
# 一键开局：build → server → frontend → N 个 demo bot → 开浏览器。
# Ctrl-C 关全套。
set -euo pipefail
cd "$(dirname "$0")/.."

BOTS=${BOTS:-4}
TICK_MS=${TICK_MS:-600}
WIPE=${WIPE:-1}

echo ">> build server + cli (--quiet)"
cargo build -p server -p cli --quiet

if [ ! -d frontend/node_modules ]; then
  echo ">> install frontend deps"
  (cd frontend && (pnpm install --silent || npm install --silent))
fi

if [ "$WIPE" = "1" ]; then
  echo ">> wipe data/"
  rm -rf data
fi

echo ">> start server :7777 (tick=${TICK_MS}ms)"
LINGYUAN_TICK_MS=$TICK_MS ./target/debug/server > /tmp/lingyuan-server.log 2>&1 &
SERVER_PID=$!

echo ">> start frontend :5173"
(cd frontend && (pnpm dev --port 5173 --host 127.0.0.1 || npx vite --port 5173 --host 127.0.0.1)) > /tmp/lingyuan-fe.log 2>&1 &
FE_PID=$!

BOT_PIDS=()
cleanup() {
  echo
  echo ">> stopping all"
  for p in "${BOT_PIDS[@]}"; do kill "$p" 2>/dev/null || true; done
  kill $SERVER_PID $FE_PID 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

# 等 server 起
for i in $(seq 1 30); do
  if curl -fs http://127.0.0.1:7777/health > /dev/null 2>&1; then
    echo "server up after ${i}*200ms"; break
  fi
  sleep 0.2
done
for i in $(seq 1 30); do
  if curl -fs http://127.0.0.1:5173 > /dev/null 2>&1; then
    echo "frontend up after ${i}*200ms"; break
  fi
  sleep 0.2
done

NAMES=(wukong bajie shaseng tangseng nezha erlang lvbu zhaoyun)
for i in $(seq 1 $BOTS); do
  name=${NAMES[$((i-1))]:-bot$i}
  ./target/debug/survivor demo --name $name --period-ms 500 > /tmp/lingyuan-bot-$name.log 2>&1 &
  BOT_PIDS+=($!)
  echo "  bot $name (pid $!)"
done

if command -v open >/dev/null; then
  echo ">> opening browser"
  open http://127.0.0.1:5173
fi

echo
echo "===================================="
echo "  灵渊运行中。Ctrl-C 退出。"
echo "  server log:  tail -f /tmp/lingyuan-server.log"
echo "  bot logs:    tail -f /tmp/lingyuan-bot-*.log"
echo "===================================="
wait $SERVER_PID
