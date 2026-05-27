# 灵渊 (Lingyuan) — 当前状态

> 自动生成于 2026-05-27 凌晨。spec 见 `docs/superpowers/specs/2026-05-27-lingyuan-design.md`。

## ✅ 已完成里程碑

| 里程碑 | 状态 | 内容 | 验证 |
|--------|------|------|------|
| **M1 骨架** | ✅ | Cargo workspace 三 crate（world/server/cli）+ tokio + sqlx + axum | `cargo build` 全过 |
| **M2 基础世界** | ✅ | 80×80 多 biome 地图、agent join/observe/move、fog of war、tile/agent 渲染 | server integration 3/3 + 前端 build 成功 |
| **M3 求生闭环** | ✅ | hp/hunger/stamina 衰减、采集（10 种 plant）、9 个 T0/T1 配方、灶台篝火、进食、死亡重生 | 12 个针对性单测 |
| **M4 战斗与怪物** | ✅ | 攻击动作、动物 AI（rabbit/deer 逃跑）、怪物 AI（wolf/night_demon 主动追击）、夜晚刷新、全场 PvP | 2 个攻击单测 |
| **M5 社交** | ✅ | 立牌（路牌）+ 飞鸽（定向私信）+ 容量限制 + 雾区可见性 | 2 个单测 + e2e 跑通 |
| **M9 Agent skill** | ✅ | `assets/skill/lingyuan-survivor.md` 完整 skill 文档 | 可直接拷给 Claude Code |
| **M8 sprite GPT 流水线** | ✅ | 27 张仙侠像素 sprite（plant/creature/building/agent）入仓；前端 sprite-cache lazy 加载 + fallback 色块 | `docs/screenshot-v1.png` |

**测试统计**：world crate 42 单测 + server crate 3 集成测试 全 PASS；前端 build OK + playwright 截图正常。

## 🚧 未完成里程碑

| 里程碑 | 状态 | 内容 |
|--------|------|------|
| M6 季节效果 + boss | 部分 | clock 跑季节、UI 显示；季节差异化 spawn / boss event 未做 |
| M7 前端水墨终态 | 部分 | 已有功能性 UI；卷轴动画 / 水墨 hover / 关注模式未做 |
| M8 物品图标 sprite | 未跑 | manifest 里 19 个 item icon 还没生（item HUD 也还没建，等 M7 一起做）|
| M8 tile sprite | 主动跳过 | 已尝试，gpt-image-2 把 tile prompt 当艺术品做，全部生成精美但不可平铺的孤景图。tile 保留色块 |
| M10 replay 工具 | 待启动 | 事件日志已持久化（SQLite events 表），但没有 replay CLI |

## 📂 项目结构

```
lingyuan/
├── Cargo.toml                  workspace
├── README.md
├── STATUS.md                   ← 你正在看的文件
├── crates/
│   ├── world/                  纯逻辑（42 单测）
│   ├── server/                 axum + sqlx + tick loop（3 集成测试）
│   └── cli/                    survivor CLI binary
├── frontend/                   Vite + PixiJS（pnpm build OK）
├── assets/
│   └── skill/
│       └── lingyuan-survivor.md  ← agent 接入文档
├── scripts/
│   ├── smoke.sh                端到端基础冒烟
│   ├── demo.sh                 多 agent 同台 demo（开浏览器）
│   ├── survival-smoke.sh       M3 求生闭环冒烟
│   └── gen_sprites.py          M8 sprite 生成（待跑）
└── docs/
    └── superpowers/
        ├── specs/2026-05-27-lingyuan-design.md
        └── plans/
            ├── 2026-05-27-m1-m2-skeleton-and-world.md
            ├── 2026-05-27-m3-survival-loop.md
            └── 2026-05-27-m4-combat.md
```

## 🚀 怎么玩起来

```bash
# 1. 启动服务（监听 :7777）
cd /Users/e0_7/projects/games/lingyuan
cargo run -p server

# 2. 启动浏览器观战 UI（:5173）
cd frontend && pnpm dev
# 然后浏览器开 http://127.0.0.1:5173

# 3a. 自己当 agent 玩玩
cargo run -p cli -- join --name human --server http://localhost:7777
cargo run -p cli -- observe
cargo run -p cli -- act move --dir=north

# 3b. 把 LLM 接进来（Claude Code）
cp assets/skill/lingyuan-survivor.md ~/.claude/skills/
# 在 Claude Code 内：/skill lingyuan-survivor
# Claude 会按 skill 指引自动调 survivor CLI

# 4. 一键 demo（开 server + frontend + 2 个 curl 模拟 agent 随机走）
bash scripts/demo.sh
```

## 🎨 Sprite 工作流（已跑 27 张）

调用 `~/.claude/skills/gpt-image-2` skill（RunningHub backbone）：

```bash
# 已跑完的批次
python scripts/batch_gen_sprites.py --only-category plant      # 10/10
python scripts/batch_gen_sprites.py --only-category creature   # 8/8
python scripts/batch_gen_sprites.py --only-category building   # 2/2
python scripts/batch_gen_sprites.py --only-category agent      # 4/4
python scripts/batch_gen_sprites.py --only-category tile       # 4/13 后停（产出全是孤景）

# 待跑
python scripts/batch_gen_sprites.py --only-category item       # 19 张

# 后处理：去 RH 棋盘背景 + 降采样到 32x32 + 量化 5 色
python scripts/post_process_sprite.py

# 同步到前端
rm -rf frontend/public/sprites && cp -r assets/sprites frontend/public/
```

效果见 `docs/screenshot-v1.png`。

### Sprite 老 OpenAI 备份方案



`scripts/gen_sprites.py` 已写好，需要 OpenAI API key 才能跑：

```bash
export OPENAI_API_KEY=sk-...
python scripts/gen_sprites.py        # 一次性生成 ~60 张 sprite 入仓 assets/sprites/
python scripts/audit_sprites.py      # 校验完整性 + 调色板合规
```

资产清单：
- 13 种 tile × 1 = 13
- 2 种建筑（campfire / cooking_stove）× 1 = 2
- 4 种 creature × 2 朝向 = 8
- 10 种 plant × 1 = 10
- agent 4 朝向（hash tint 区分玩家）= 4
- 物品图标 × ~20

预算估算：~60 张 × $0.04 = **< $3 一次性**。

跑完后，前端 `tile-layer.ts` / `entity-layer.ts` 需要改用 `Sprite.from('...')` 替代当前的 `Graphics.rect/circle`。这是 ~1 天工作量。

## 🔧 当前主要技术债

1. **server warning**：`TickFrame.clock` 和 `.observations` 字段 dead_code（未实际使用）。无害但应清理。
2. **survival-smoke.sh** 的 bash 寻路逻辑有 bug（没正确算 abs）——单测覆盖了真功能，这只是冒烟脚本不够好看，不阻塞。
3. **agent 重生位置**完全随机；可改为"在死亡点附近 N 格"更合理。
4. **mailbox 没有持久化进 SQLite**（snapshot 含 World 全字段所以有；但事件日志没单独 mail 事件除了 sent_mail 摘要）——观察上 OK，重启恢复 OK。
5. **iron_sword/talisman_paper 等 T2 物品已在 ItemKind 枚举里但没有配方**（占位等 M6）。

## 📊 提交历史

```
53dcfeb feat: M5 社交（立牌 + 飞鸽）+ M9 skill markdown — 42 world tests pass, e2e green
1c5678b feat: M4 战斗+怪物+PvP（attack action, creature_ai, night spawn） — 40 world tests pass
f7afe67 plan: M4 战斗+怪物+PvP
8f3c612 feat(cli): support gather/eat/craft/place/pickup/drop verbs; survival-smoke.sh
c6b8d26 feat: M3 求生闭环（hunger/gather/eat/craft/place/death/respawn） — 38 unit + 3 integration tests
d9e0c55 plan: M3 求生闭环（hunger/gather/craft/build/eat/death/respawn）
defd2b6 feat: M1+M2 complete — server WS 推送、frontend PixiJS、scripts/demo.sh 验证全栈
a02ecf7 feat(server+cli): axum+sqlx server with REST/WS + survivor CLI + smoke.sh — integration 2/2 + e2e green
bfe7710 feat(world): complete crate (coord/tile/grid/clock/rng/action/event/agent/gen/observation/world) — 25 tests pass
d384c8f plan: M1+M2 骨架与基础世界实施计划
8a1b09e spec: 灵渊 v0.1 设计文档
```

## 🎯 推荐继续顺序（如果你想接着推）

1. **跑 sprite 生成**：`python scripts/gen_sprites.py`（需要 API key），改前端 layers 用 sprite，UI 立刻从 dev demo 变 product
2. **加 boss / 渡劫**：丹炉合成 + 金丹 + 满 3 颗触发全图通告 boss，给一局加一个高光时刻
3. **季节效果**：春草药 +1.5 / 夏夜怪 +30% / 冬覆雪 + 寒鸦，让循环不枯燥
4. **关注模式 UI**：点击地图上的 agent → 切换到它视角 + 显示它最近动作日志，观战体验↑↑
5. **replay 工具**：CLI 读 SQLite events，重播某 tick 区间到内存 World，输出最终状态截图

—— 至此 v1 alpha 可玩、可观、可接 agent。
