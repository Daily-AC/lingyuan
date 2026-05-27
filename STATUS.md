# 灵渊 (Lingyuan) — 当前状态

> v2-alpha · 2026-05-27 凌晨/上午两轮推进。spec 见 `docs/superpowers/specs/2026-05-27-lingyuan-design.md`。

## 视觉记录

| 截图 | 说明 |
|------|------|
| `docs/screenshot-v1.png` | M1-M5 完成后初版：色块 tile + 简单 agent dot，密度爆炸 |
| `docs/screenshot-v2-overview.png` | M8 sprite 落地：植物/动物 sprite 满地图 |
| `docs/screenshot-v2-focus.png` | M7 关注模式上线：alice 居中 + 金圈 + 名字胶囊 |
| `docs/screenshot-v3-focus-inv.png` | inventory HUD + 昼夜全局色调 + 4 个 demo bot 实战 |
| `docs/screenshot-v3-focus-effects.png` | 浮字 + minimap + 战斗事件流 |
| `docs/screenshot-v4-full-hud.png` | **最终**：sprite 满 + minimap + 高亮事件 + chip icon |

## ✅ 全部完成

| 里程碑 | 状态 | 关键内容 |
|--------|------|---------|
| **M1 骨架** | ✅ | Cargo workspace 3 crate + tokio + sqlx + axum |
| **M2 基础世界** | ✅ | 80×80 5 biome 地图 + agent join/observe/move + fog of war |
| **M3 求生闭环** | ✅ | hp/hunger/stamina + 9 配方 + 火堆灶台 + 进食 + 死亡重生 |
| **M4 战斗 + 怪物 + PvP** | ✅ | 4 种生物 + 夜晚刷新 + agent vs agent 攻击 + 全场 PvP |
| **M5 社交** | ✅ | 立牌路标 + 飞鸽私信（**唯一靠自然语言的玩法**） |
| **M6-mini Boss** | ✅ | 渡劫者（hp800 atk35）每 1500 tick 自动刷 + 全图通告事件 |
| **M7+ UI 改造** | ✅ | 关注模式 + agent 高亮 + idle bob + 浮字 + minimap + 昼夜色调 + chip icon + canvas 直接点 + Esc 取消 |
| **M8 sprite** | ✅ | **46 张** gpt-image-2 仙侠像素 sprite（plant 10 / creature 8 / building 2 / agent 4 / item 19 / tile 4） |
| **M9 Agent skill** | ✅ | `assets/skill/lingyuan-survivor.md` 完整 |
| **M10 replay CLI** | ✅ | `survivor replay --db ... --from N --to M --summary` 事件回放 |
| **M-bot Demo NPC** | ✅ | `survivor demo --name X` 自动 AI（找食物/反击/逃跑） |

**测试**：world crate 42 单测 + server crate 3 集成测试 全 PASS。

**代码量**：188 文件 / 19,669 行 / 27 commits。

## 🚀 怎么玩

```bash
cd /Users/e0_7/projects/games/lingyuan

# 1. 起服务
cargo run -p server                       # :7777

# 2. 起 UI（另一终端）
cd frontend && pnpm dev                   # :5173

# 3a. 自己当 agent
cargo run -p cli -- join --name human
cargo run -p cli -- observe
cargo run -p cli -- act move --dir=north

# 3b. 让世界活起来（4 个 NPC bot 自动循环）
cargo run -p cli -- demo --name wukong &
cargo run -p cli -- demo --name bajie &
cargo run -p cli -- demo --name shaseng &
cargo run -p cli -- demo --name tangseng &

# 4. 把 LLM 接进来
cp assets/skill/lingyuan-survivor.md ~/.claude/skills/
# Claude Code 内：/skill lingyuan-survivor

# 5. replay 看过去发生了什么
cargo run -p cli -- replay --db data/world.db --from 100 --to 300 --summary
```

## 浏览器观战 UI 操作

- **点右侧 agent 行** → 放大跟焦该 agent
- **直接点 canvas 上 agent sprite** → 同上
- **Esc** 或 **右上「全图」按钮** → 切回全图
- **底部 inventory bar**：聚焦时显示该 agent 物品（带 sprite icon）
- **右上 minimap**：80×80 缩略图，红/白点标 agent
- **事件流**：聚焦时和该 agent 相关的事件左侧金边高亮
- **顶栏 tick 心跳灯**：每 tick 闪一下
- **昼夜变色**：夜里全图自动调暗偏冷
- **boss 通告**：渡劫者出现时事件流朱砂红高亮

## 🚧 已知 v1 边界

| 项 | 状态 |
|---|---|
| tile sprite | 主动放弃，gpt-image-2 把"32×32 grass tile"画成整幅仙侠小景。色块够用 |
| 季节差异化 spawn / 资源 | spec 写了但未实装（春草药+1.5 等） |
| warmth/sanity 真实衰减 | 占位但不衰减 |
| T2/T3 配方 / 丹炉 / 金丹 | 等 M6 完整版 |
| demo bot 在山袋里偶尔卡 | 视野检查已加，多 bot 互堵仍会偶尔挤死 |
| 行动 ↔ 像素移动插值 | tick 边界瞬移；可加 lerp 平滑 |
| 怪物的 hp 条 | 当前只有文字 `hp30` 浮标 |

## Sprite 工作流（已跑全套）

调 `~/.claude/skills/gpt-image-2`（RunningHub backbone）：

```bash
source .venv/bin/activate
python scripts/batch_gen_sprites.py -j 3 --only-category plant
python scripts/batch_gen_sprites.py -j 3 --only-category creature
python scripts/batch_gen_sprites.py -j 2 --only-category building
python scripts/batch_gen_sprites.py -j 2 --only-category agent
python scripts/batch_gen_sprites.py -j 3 --only-category item

python scripts/post_process_sprite.py   # 去棋盘 + 降到 32x32 + 量化 5 色
rm -rf frontend/public/sprites && cp -r assets/sprites frontend/public/
```

## 提交历史

```
3a3a4ca feat(M8 v2): item 19 张 sprite 全部生成入仓（共 46 张）+ creature canonical 名补齐
7069946 feat(UI): canvas 上直接点 agent 即聚焦
6c9642f feat(UI): Esc 取消聚焦
2810e3b feat: 聚焦 agent 时事件流相关事件加左金边高亮
c4db460 feat: 夜晚色调克制版 + inventory chip 加 item sprite 图标
68d1071 feat: 战斗浮字 + mini-map + SpectatorEntity.id 暴露
ea40b36 fix(bot): hp<40 + 视野有 hostile 时逃跑
de4c9c5 feat: agent inventory bar 聚焦时底部显示 + 昼夜/季节 CSS filter + bot 食物优先
1bc365a feat(M10): survivor replay CLI 读 SQLite events
bd10960 feat(M6-mini): 渡劫者 boss creature
a901c66 feat(cli): survivor demo NPC 自动 AI bot
8897f13 feat(M7): 关注模式 + 高亮 + bob + tick 心跳 + canvas resize 修
1b86662 docs: STATUS 更新 M8
0c1ab53 feat(M8): 27 张 sprite 入仓 + 前端 sprite-cache
935e13c feat: STATUS doc + M8 sprite 脚本
53dcfeb feat: M5 社交 + M9 skill markdown
1c5678b feat: M4 战斗 + 怪物 + PvP
f7afe67 plan: M4
8f3c612 feat(cli): gather/eat/craft 等 verb + survival-smoke
c6b8d26 feat: M3 求生闭环
d9e0c55 plan: M3
defd2b6 feat: M1+M2 complete
a02ecf7 feat: server + CLI 骨架 + smoke
bfe7710 feat: world crate 完整 25 测试
d384c8f plan: M1+M2
8a1b09e spec: v0.1 设计
```

—— **可观、可玩、可接 agent、有 NPC、有 boss、有动画、有声色（视觉而已）的 v2 alpha 完成**。
