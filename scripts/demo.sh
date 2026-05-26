#!/usr/bin/env bash
# 多 agent + 前端 demo。开启服务、前端 dev、用 curl 模拟两个 agent 随机走动，
# 在浏览器打开 http://127.0.0.1:5173 观战。Ctrl-C 退出。
set -euo pipefail
cd "$(dirname "$0")/.."

echo ">> build server + cli"
cargo build -p server -p cli --quiet

if [ ! -d frontend/node_modules ]; then
  echo ">> install frontend deps"
  (cd frontend && (pnpm install --silent || npm install --silent))
fi

echo ">> wipe data"
rm -rf data

echo ">> start server :7777"
LINGYUAN_TICK_MS=1000 ./target/debug/server > /tmp/lingyuan-server.log 2>&1 &
SERVER_PID=$!

echo ">> start frontend :5173"
(cd frontend && (pnpm dev --port 5173 --host 127.0.0.1 || npx vite --port 5173 --host 127.0.0.1)) > /tmp/lingyuan-fe.log 2>&1 &
FE_PID=$!

cleanup() {
  echo
  echo ">> stopping agents loop, server ($SERVER_PID), frontend ($FE_PID)"
  kill ${AGENTS_PID:-} 2>/dev/null || true
  kill $SERVER_PID 2>/dev/null || true
  kill $FE_PID 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

# 等服务
for i in $(seq 1 30); do
  if curl -fs http://127.0.0.1:7777/health > /dev/null 2>&1; then
    echo "server up"
    break
  fi
  sleep 0.3
done

# 用 curl 直接注册两个 agent，避免依赖单个 token store
join() {
  curl -s -X POST http://127.0.0.1:7777/api/v1/join \
    -H 'Content-Type: application/json' \
    -d "{\"name\":\"$1\"}"
}

ALICE=$(join alice)
BOB=$(join bob)
A_ID=$(echo "$ALICE" | sed -n 's/.*"agent_id":"\([^"]*\)".*/\1/p')
A_TOK=$(echo "$ALICE" | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')
B_ID=$(echo "$BOB" | sed -n 's/.*"agent_id":"\([^"]*\)".*/\1/p')
B_TOK=$(echo "$BOB" | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

echo "alice = $A_ID, bob = $B_ID"

echo ">> opening browser: http://127.0.0.1:5173"
( command -v open >/dev/null && open http://127.0.0.1:5173 ) || true

random_walk() {
  local id=$1 tok=$2
  while true; do
    local dirs=(north south east west)
    local d=${dirs[$((RANDOM % 4))]}
    curl -s -X POST http://127.0.0.1:7777/api/v1/act \
      -H 'Content-Type: application/json' \
      -H "Authorization: Bearer $tok" \
      -H "X-Agent-Id: $id" \
      -d "{\"kind\":\"move\",\"data\":{\"dir\":\"$d\"}}" > /dev/null
    sleep 1.5
  done
}

( random_walk "$A_ID" "$A_TOK" ) &
A_PID=$!
( random_walk "$B_ID" "$B_TOK" ) &
B_PID=$!
AGENTS_PID="$A_PID $B_PID"

echo ">> demo running. Ctrl-C to stop."
wait $SERVER_PID
