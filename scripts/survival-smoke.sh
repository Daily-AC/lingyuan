#!/usr/bin/env bash
# M3 求生闭环 e2e 冒烟：
#   起 server → 注册 alice → observe 找最近 plant → walk 到旁边 → gather → eat
set -euo pipefail
cd "$(dirname "$0")/.."

echo ">> build"
cargo build -p server -p cli --quiet

echo ">> wipe + start server"
rm -rf data
LINGYUAN_TICK_MS=250 ./target/debug/server > /tmp/lingyuan-survival.log 2>&1 &
PID=$!
trap "kill $PID 2>/dev/null || true" EXIT

for i in $(seq 1 30); do
  curl -fs http://127.0.0.1:7777/health > /dev/null 2>&1 && break
  sleep 0.2
done

export LINGYUAN_TOKEN_PATH=/tmp/lingyuan-survival-token.json
./target/debug/survivor clear || true
./target/debug/survivor join --name alice --server http://localhost:7777 > /dev/null

# 走 10 步随机找资源
TARGET_X="" ; TARGET_Y="" ; TARGET_KIND=""
for try in $(seq 1 20); do
  OBS=$(./target/debug/survivor observe --format json)
  TARGET_X=$(echo "$OBS" | jq -r '.visible_entities[] | select(.kind=="plant") | .pos.x' | head -1)
  TARGET_Y=$(echo "$OBS" | jq -r '.visible_entities[] | select(.kind=="plant") | .pos.y' | head -1)
  TARGET_KIND=$(echo "$OBS" | jq -r '.visible_entities[] | select(.kind=="plant") | .species' | head -1)
  if [ -n "$TARGET_X" ]; then
    SELF_X=$(echo "$OBS" | jq -r '.self.pos.x')
    SELF_Y=$(echo "$OBS" | jq -r '.self.pos.y')
    DIST=$(( (TARGET_X - SELF_X)*(TARGET_X - SELF_X > 0 ? 1 : -1) + (TARGET_Y - SELF_Y)*(TARGET_Y - SELF_Y > 0 ? 1 : -1) ))
    echo "see plant $TARGET_KIND at ($TARGET_X,$TARGET_Y), self @($SELF_X,$SELF_Y)"
    if [ "$DIST" -le 1 ]; then
      break
    fi
    # 朝目标方向走一步
    if [ "$TARGET_X" -gt "$SELF_X" ]; then DIR=east
    elif [ "$TARGET_X" -lt "$SELF_X" ]; then DIR=west
    elif [ "$TARGET_Y" -gt "$SELF_Y" ]; then DIR=south
    else DIR=north
    fi
    ./target/debug/survivor act move --dir=$DIR > /dev/null
  else
    DIRS=(north south east west)
    DIR=${DIRS[$((RANDOM % 4))]}
    ./target/debug/survivor act move --dir=$DIR > /dev/null
  fi
  sleep 0.4
done

if [ -z "$TARGET_X" ]; then
  echo "❌ no plant found nearby in 20 tries"
  exit 1
fi

echo ">> gather"
./target/debug/survivor act gather --pos="$TARGET_X,$TARGET_Y"
sleep 0.5
echo "--- observe after gather ---"
./target/debug/survivor observe --format markdown

# 如果采到的是 mushroom/red_berry/lingzhi，吃
ITEM_TO_EAT=$(./target/debug/survivor observe --format json | jq -r '.self.inventory[] | select(.item == "mushroom" or .item == "red_berry" or .item == "lingzhi") | .item' | head -1)
if [ -n "$ITEM_TO_EAT" ]; then
  echo ">> eat $ITEM_TO_EAT"
  ./target/debug/survivor act eat --item="$ITEM_TO_EAT"
  sleep 0.5
  echo "--- observe after eat ---"
  ./target/debug/survivor observe --format markdown
fi

./target/debug/survivor leave
echo "✅ survival smoke ok"
