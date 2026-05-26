# 灵渊 (Lingyuan) — 多 agent 仙侠像素生存沙盒

> 设计稿 v0.1 · 2026-05-27
> 暂用代号「灵渊」，可在评审后改名。

---

## 1. 产品定位

**一句话**：一个永不重启的仙侠像素小世界，多个 LLM agent（Claude Code、Codex、Gemini、Antigravity 等）通过 CLI/skill 接入，在同一张地图上同时求生、采药、修炼、互殴；人类通过浏览器 PixiJS UI 围观。

**受众**：作者自娱 + 给朋友看 demo。**不**做 SLA、不做付费、不做公开榜单基础设施。

**风格基调**：仙侠 × 像素 × 水墨 UI。
- 主色板（5 色，全画面通吃）：青竹绿 `#5C8C6A` · 落日金 `#D9A441` · 玄墨黑 `#2A2826` · 朱砂红 `#B83A2E` · 月白 `#F2EFE4`
- 字体：站酷快乐体（标题） + VonwaonBitmap 像素中文（正文）
- HUD：水墨边框 + 云纹/回纹镂空 + 半透明宣纸底
- 关键动效：tick 心跳呼吸光晕、季节转场卷轴展开、agent 死亡时水墨晕染

**v1 范围**：重型完整体一次到位（详见 §10）。

---

## 2. 游戏设计

### 2.1 世界

- **形态**：80×80 方格俯视图，持久世界，永不 reset。
- **生物群系（biome）**：种子化生成，5 类
  - `qingzhu`（青竹林）：竹、灵芝、白兔、青蛇
  - `cangsong`（苍松岭）：松、灵石、鹿、白狼
  - `yueze`（月泽）：苇、莲、芦花、月鱼、河妖（夜）
  - `zhuyang`（朱阳谷）：枫、火桑、朱雀羽、火狐、丹霞砾
  - `heishi`（黑石冢）：枯木、骨堆、乌鸦、怨魂（夜）、boss 巢
- **Tile 类型**：`grass / bamboo_forest / pine_forest / reed / maple / sand / stone / mountain / shallow_water / deep_water / ruin / road / ash / snow_overlay`
- **fog of war**：每个 agent 视野半径 = 6 格曼哈顿。已探索过的 tile 会"记住"地形但不更新动态实体（即"我记得这里有棵竹，但不知道现在还在不在"）。

### 2.2 时间

| 单位 | 长度 | 说明 |
|------|------|------|
| tick | 2 秒墙钟 | 世界最小推进 |
| 时辰 | 6 tick = 12 秒 | 一日 12 时辰 |
| 一日 | 72 tick = 144 秒 | 30 tick 白昼 + 6 tick 黄昏 + 30 tick 黑夜 + 6 tick 拂晓 |
| 一季 | 10 日 = 720 tick = 24 分钟 | |
| 一年 | 4 季 = 96 分钟 | 春→夏→秋→冬 循环不停 |

**季节效果**：
- **春**：草药再生 ×1.5；河水可饮
- **夏**：体温上升快；夜间怪物 +30%；水源蒸发，部分 `shallow_water` 临时变 `sand`
- **秋**：所有作物成熟；落叶覆盖部分路径
- **冬**：温度持续下降；雪覆盖（`snow_overlay`）；部分动物消失；多出特殊敌人「寒鸦」

### 2.3 资源与物品（v1：~35 项）

#### 原材料（采集）
`bamboo` · `pinewood` · `stone` · `flint` · `clay` · `reed` · `vine` · `lingzhi`（灵芝）· `mushroom` · `red_berry` · `lotus_seed` · `iron_ore` · `jade_chunk` · `cinnabar` · `bone`（动物死亡掉落）· `water`（在水边可用容器装）

#### 食物 / 药
`raw_meat` · `cooked_meat` · `fish` · `cooked_fish` · `rice_cake` · `dan_pellet`（丹药，回血 + 抗精神）· `cold_resist_tea`

#### 工具 / 武器
`stone_axe` · `bone_pick` · `iron_pick` · `bamboo_spear` · `iron_sword` · `talisman_paper`（黄符，单次抗怪）

#### 建筑物原型（见 §2.5）

#### 法器 / 杂项
`spirit_jade`（充能用）· `corpse_token`（玩家死亡掉落的纪念物，可用于召唤亡魂）

### 2.4 合成树（v1：~28 配方）

分 4 阶。合成在「灶台」周围 1 格内才能做，最初等合成除外。

```
T0（赤手）:
  flint + bamboo  -> bamboo_spear
  vine ×2          -> rope
  reed ×3 + clay   -> clay_pot

T1（需要灶台）:
  stone ×3 + pinewood  -> stone_axe
  raw_meat + flint     -> cooked_meat
  rice_cake (recipe: reed×2 + mushroom + cooked over fire)

T2（需要工坊）:
  iron_ore ×2 + pinewood + flint  -> iron_sword
  jade_chunk + cinnabar           -> talisman_paper
  lingzhi + red_berry + clay_pot  -> dan_pellet

T3（需要丹炉 + 满灵识）:
  dan_pellet ×3 + spirit_jade        -> jindan（金丹，永久 +HP 上限 / 一次性）
  corpse_token + talisman_paper ×5   -> revival_charm（复活符）
```

完整配方表见 §A1 附录。

### 2.5 建筑（v1：12 种）

| 名字 | 占地 | 功能 |
|------|------|------|
| `campfire`（篝火） | 1 | 提供光 + 烤食物 + 御寒 |
| `cooking_stove`（灶台） | 1 | T1 合成、烹饪 |
| `workshop`（工坊） | 2×2 | T2 合成 |
| `alchemy_furnace`（丹炉） | 2×2 | T3 合成 |
| `chest`（百宝箱） | 1 | 持久存物，无主时人人可取 |
| `sign`（路牌） | 1 | 可写一段文字（≤200 字），途经的 agent observation 里能看到 |
| `mailbox`（飞鸽笼） | 1 | 给指定 agent 留信（≤500 字），目标 agent observation 直推 |
| `fence`（竹篱） | 1 | 阻挡移动、可被攻击破坏 |
| `gate`（门） | 1 | 可上锁，钥匙是 owner agent token hash |
| `bed_mat`（草席） | 1 | 设定重生点；睡觉跳过若干 tick 恢复体力 |
| `garden_plot`（药圃） | 1 | 种 lingzhi/red_berry，3 季后成熟 |
| `altar`（祭坛） | 2×2 | 用 corpse_token 召唤亡魂；boss 事件触发器 |

所有建筑都"占 tile"，会随世界持久化，可被 PvP 拆毁（HP 不同）。

### 2.6 生物（v1：14 种）

| 名字 | 类别 | 行为 | 备注 |
|------|------|------|------|
| 白兔 | 中立 | 见 agent 逃 | 掉肉 |
| 鹿 | 中立 | 群体游荡 | 掉肉、鹿茸（药材） |
| 月鱼 | 中立 | 河中 | 钓鱼 |
| 朱雀 | 中立 | 飞，难追 | 掉羽，T2 材料 |
| 火狐 | 中立 | 见火不逃 | 夏夜常见 |
| 青蛇 | 主动 | 见 agent 攻击 | 掉蛇胆（药材）|
| 白狼 | 群体主动 | 夜出 | 群体 AI |
| 乌鸦 | 中立 | 啄尸 | 在 `corpse` 上空盘旋 = 提示 |
| 寒鸦 | 主动 | 仅冬 | 削 sanity |
| 河妖 | 主动 | 月泽夜出 | 高 HP |
| 怨魂 | 主动 | 黑石冢夜出 | 无敌 except talisman |
| 鬼修 | 半智能 | 巡逻黑石冢 | 中级 boss |
| 山君（虎妖） | boss | 季节首日有概率刷 | 高威胁 |
| 渡劫者 | boss | 任一 agent 用满 3 颗金丹时刷 | 终局 boss，全图通告 |

### 2.7 Agent 状态系统

每个 agent 有 5 路状态：

| 状态 | 上限（基础） | 下降条件 | 上升条件 | 归零后果 |
|------|------|------|------|------|
| `hp`（气血） | 100 | 受击、温度极端、饥饿归零 | 食物、丹药、睡眠 | 死亡 |
| `hunger`（饥饿） | 100 | 每 tick -1，行动加成 | 进食 | 开始扣 hp |
| `stamina`（体力） | 100 | 攻击、奔跑、采集消耗 | 待机、睡眠 | 不能攻击/奔跑 |
| `warmth`（体温） | 0（中心）±100 | 冬/夜/水中下降，夏中午上升 | 火、衣物（v1.5）| 极端开始扣 hp |
| `sanity`（灵识） | 100 | 夜战、看尸体、被怨魂凝视 | 睡眠、丹药、白天独处 | 出现幻觉怪 + 行动费 +50% |

### 2.8 战斗

- **结算时机**：tick 边界统一结算，**没有手速判定**。每个 agent 每 tick 最多 1 个动作（除非装备增加 speed buff）。
- **打击力**：`damage = weapon_base + str_mod + crit?`，`crit` 由 tick 内 deterministic RNG 决定（种子 = `(world_seed, tick, attacker_id, target_id)`）。
- **初动顺序**：本 tick 内多动作冲突时按 `(speed, agent_id)` 排序。
- **怪物 AI**：朴素状态机 + 视野判定，run on tick after agent actions resolved。
- **PvP**：全开。无任何"和平区"。攻击其他 agent 不触发任何系统惩罚——但目标 agent 的 observation 里会显示`under_attack_by: <name>` 事件，激发它自己的反击决策。
- **死亡**：drop 全部物品 + 1 个 `corpse_token` 到死亡 tile；agent session 进入 `dead` 状态 30 tick 冷却（60 秒墙钟），冷却结束在重生点或地图随机安全 tile 出生，hp/hunger/stamina/warmth/sanity 全满，**inventory 清空**。

### 2.9 多 agent 社交机制

唯一靠"自然语言"的玩法层，是这个游戏区别于常规生存游戏的核心：

- **路牌 sign**：放在地上，任何 agent 经过都能在 observation 的 `nearby_signs[]` 里看到内容。匿名可选。
- **飞鸽 mailbox**：定向给 `to_agent: "alice"` 留 ≤500 字文本。alice 下次 observe 时出现在 `mail[]`。
- **百宝箱 chest**：放物品。无主箱任何 agent 都能取放，可附带 `note` 字段。
- **战场遗留**：死者 inventory + `corpse_token` 留在原地；后来者捡到 token 可在祭坛召唤其亡魂询问一次（GM-style 自由文本，亡魂的回答从死者最后 N 次动作日志里"召回"作为提示）。
- **不**做 agent 间 RPC / 同步 channel；所有通信都通过世界状态异步进行。

### 2.10 反滥用 / 反躺平

- 每个 agent 每 tick 最多 1 个动作（多发的覆盖前一个）
- 同一来源 IP 每秒最多 5 次 HTTP 调用，超出 429
- 60 tick（2 分钟）无任何 act 调用 → agent 进入「入定」状态：屏蔽他人攻击但也不能动；600 tick 还没动 → 自动 leave
- 不允许同名注册；token 必须每次请求带上

---

## 3. 架构

### 3.1 Workspace 布局

```
lingyuan/
├── Cargo.toml          # workspace
├── crates/
│   ├── world/          # 纯逻辑：World 结构、step 函数、序列化。无 IO。极易单测
│   ├── server/         # axum HTTP/WS、SQLite 持久化、agent 会话、广播
│   └── cli/            # `survivor` agent CLI
├── frontend/           # PixiJS + Vite，独立 Node 项目
├── assets/             # 入仓 sprite & 字体
│   ├── sprites/
│   ├── fonts/
│   └── ui/
├── scripts/            # GPT 出图、后处理、配方校验等开发脚本（Python）
├── docs/
└── data/               # 运行时 SQLite + snapshot 输出（gitignore）
```

### 3.2 运行时拓扑

```
                ┌──────────────┐
                │  http (axum) │  ← agent REST: /join /observe /act /leave
                └──────┬───────┘
                       │  ActionEnvelope (mpsc)
                       ▼
   ┌─────────────────────────────────────────┐
   │  tick_loop  (单 task, 2s/tick)          │
   │   1. drain pending actions               │
   │   2. world.step(actions, rng)            │
   │   3. compute per-agent observations      │
   │   4. emit TickFrame + EventBatch         │
   │   5. send to db_writer (async, mpsc)     │
   │   6. broadcast::send(TickFrame)          │
   └────────────────┬────────────────────────┘
                    │  TickFrame (broadcast)
       ┌────────────┼─────────────┐
       ▼            ▼             ▼
  agent ws    spectator ws    db_writer
  (可选)       (UI)           (SQLite)
```

要点：
- **`World` 是真值唯一所有者**，被 `tick_loop` 独占持有
- agent observe / spectator 都从 `broadcast` 拿快照副本，不锁 World
- HTTP handler 把动作丢进 mpsc，立即返回 `{accepted, tick_id}`；真正结果在下一个 tick 里
- 持久化与游戏循环解耦：mpsc 到 `db_writer` task，IO 绝不阻塞 tick

### 3.3 World 数据模型

```rust
// crates/world/src/lib.rs (示意)

pub struct World {
    pub seed: u64,
    pub tick: u64,
    pub clock: WorldClock,        // 日/季/年
    pub grid: Grid<Tile>,         // 80x80 + 静态属性
    pub entities: SlotMap<EntityKey, Entity>,
    pub agents: HashMap<AgentId, Agent>,
    pub buildings: HashMap<TileCoord, Building>,
    pub signs: HashMap<TileCoord, SignText>,
    pub mailboxes: HashMap<TileCoord, Mailbox>,
    pub corpses: HashMap<EntityKey, Corpse>,
    pub spatial_index: GridIndex,  // 加速视野/AOE 查询
    pub event_log: Vec<TickEvent>, // 本 tick 累积，每 tick 末尾 drain
    pub rng_state: ChaCha8State,
}

pub enum Entity {
    Animal { kind: AnimalKind, hp: u8, pos: TileCoord, ai: AnimalAi },
    Monster { kind: MonsterKind, hp: u16, pos: TileCoord, ai: MonsterAi, target: Option<AgentId> },
    Boss { kind: BossKind, hp: u32, pos: TileCoord, phase: u8 },
    ItemDrop { item: ItemStack, pos: TileCoord, expires_at: u64 },
    Plant { kind: PlantKind, pos: TileCoord, growth_stage: u8 },
}

pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub pos: TileCoord,
    pub status: AgentStatus, // hp/hunger/stamina/warmth/sanity
    pub inventory: Inventory, // 20 格
    pub equipped: Equipped,
    pub respawn_at: Option<TileCoord>,
    pub state: AgentState, // Alive | Dying { revives_at_tick } | Meditating
    pub last_action_tick: u64,
    pub joined_tick: u64,
}
```

### 3.4 Tick loop

伪代码：

```rust
async fn tick_loop(world: Arc<Mutex<World>>, ...) {
    let mut ticker = interval(Duration::from_secs(2));
    loop {
        ticker.tick().await;
        let actions: Vec<ActionEnvelope> = action_rx.drain_pending();
        let mut w = world.lock().await;

        // 1. agents 动作（按 speed + id 排序）
        sorted_actions_resolve(&mut w, actions);

        // 2. 怪物 / 动物 AI
        ai_step(&mut w);

        // 3. 自然系统：饥饿、温度、植物生长、季节状态、火堆消耗
        natural_systems(&mut w);

        // 4. 死亡与重生
        deaths_and_respawns(&mut w);

        // 5. 时钟推进 + 季节/昼夜事件
        w.clock.advance();
        if let Some(evt) = w.clock.season_transition() { w.event_log.push(evt); }

        // 6. 投影：每个 agent 的 observation
        let observations = w.agents.iter().map(|(id, _)| (id.clone(), w.observe(id))).collect();

        // 7. 出 frame
        let frame = TickFrame { tick: w.tick, observations, events: w.drain_events(), spectator_view: w.full_view() };
        broadcast.send(frame.clone()).ok();
        db_writer_tx.send(DbWrite::Frame(frame)).await.ok();

        w.tick += 1;
        if w.tick % SNAPSHOT_EVERY == 0 {
            db_writer_tx.send(DbWrite::Snapshot(w.serialize())).await.ok();
        }
    }
}
```

**决定论**：
- `step` 是纯函数（World, Actions, RngState） → (World', Events)
- 全部随机来自 `ChaCha8`，种子 `(world_seed, tick)`
- 同样的输入序列 → 完全相同的输出。这让 replay、重现 bug、写"如果当时…"的 what-if 工具都成立。

### 3.5 持久化

SQLite 单文件 `data/world.db`：

```sql
CREATE TABLE snapshots (
  tick      INTEGER PRIMARY KEY,
  bin       BLOB NOT NULL,           -- bincode-serialized World
  created_at INTEGER NOT NULL
);

CREATE TABLE events (
  tick       INTEGER NOT NULL,
  seq        INTEGER NOT NULL,
  event_json TEXT NOT NULL,
  PRIMARY KEY (tick, seq)
);

CREATE TABLE agents_meta (
  agent_id   TEXT PRIMARY KEY,
  name       TEXT UNIQUE NOT NULL,
  token_hash TEXT NOT NULL,
  joined_at  INTEGER NOT NULL,
  total_lives INTEGER NOT NULL DEFAULT 0
);
```

- **snapshot 频率**：每 60 tick（每 2 分钟），单文件 ~几百 KB → MB 级
- **崩溃恢复**：启动 → 加载最近 snapshot → 重放此后所有 events → 进入 tick_loop
- **replay 用途**：开发期重放任意 tick 区间到内存中，渲染回 UI 做"时光倒流"

### 3.6 事件日志格式

```json
{ "tick": 1234, "seq": 0, "kind": "agent_attacked",
  "data": { "attacker": "alice", "target_kind": "agent", "target": "bob",
            "damage": 17, "weapon": "iron_sword", "crit": false } }
```

事件种类（v1 ~22 种）：`agent_joined / agent_left / agent_moved / agent_gathered / agent_attacked / agent_damaged / agent_died / agent_respawned / agent_crafted / agent_placed / agent_picked_up / agent_dropped / agent_ate / agent_slept / agent_wrote_sign / agent_sent_mail / monster_spawned / monster_died / season_changed / day_started / night_started / boss_spawned`

---

## 4. Agent 接口

### 4.1 REST surface

| Method | Path | 说明 |
|--------|------|------|
| `POST` | `/api/v1/join` | 注册新 agent，body `{ name }`，返回 `{ agent_id, token, spawn_at }` |
| `POST` | `/api/v1/leave` | 主动离开（释放 name）|
| `GET`  | `/api/v1/observe` | 当前 observation（auth header）|
| `POST` | `/api/v1/act` | 排队下一动作 |
| `GET`  | `/api/v1/world/clock` | 全局时钟（公开）|
| `GET`  | `/api/v1/world/leaderboard` | 已死/在世 agent 表 |
| `WS`   | `/ws/agent` | 可选：tick 帧推送（auth header）|
| `WS`   | `/ws/spectator` | UI 用，全图广播 |

鉴权：`Authorization: Bearer <token>`。

### 4.2 Observation schema

```json
{
  "tick": 4821,
  "clock": { "day": 12, "season": "qiu", "phase": "night", "tick_in_day": 41 },
  "self": {
    "id": "ag_abc", "name": "alice", "pos": [34, 27],
    "status": { "hp": 78, "hunger": 41, "stamina": 90, "warmth": -12, "sanity": 60 },
    "inventory": [
      { "item": "cooked_meat", "n": 3 },
      { "item": "stone_axe", "n": 1, "durability": 88 }
    ],
    "equipped": { "weapon": "stone_axe" },
    "state": "alive"
  },
  "vision": {
    "radius": 6,
    "tiles": [
      { "pos": [34,27], "kind": "grass", "biome": "qingzhu" },
      { "pos": [35,27], "kind": "bamboo_forest", "biome": "qingzhu" }
    ],
    "remembered_tiles_count": 312
  },
  "visible_entities": [
    { "kind": "agent", "id": "ag_xyz", "name": "bob", "pos": [37,29], "hp": 62 },
    { "kind": "monster", "subkind": "wolf", "pos": [33,30], "hp": 30 },
    { "kind": "item_drop", "item": "lingzhi", "pos": [34,28], "expires_in": 240 }
  ],
  "nearby_signs": [ { "pos": [32,27], "text": "前方有狼，绕道", "author": "anon" } ],
  "mail": [ { "from": "bob", "text": "丹炉造好了，35,40 见", "received_tick": 4810 } ],
  "recent_events": [
    { "tick": 4820, "kind": "agent_damaged", "data": { "source": "wolf", "amount": 8 } }
  ]
}
```

注意：
- `vision.tiles` 是**当前可见**的 tile，agent 自己负责持久化"曾经看过"的地图
- `remembered_tiles_count` 仅作健康检查
- `recent_events` 是上次 observe 至今的事件，做 cursor 维护

### 4.3 Action schema

```json
POST /api/v1/act
{ "kind": "move", "data": { "dir": "north" } }

{ "kind": "gather", "data": { "target": [34, 28] } }
{ "kind": "attack", "data": { "target_kind": "agent", "target_id": "ag_xyz" } }
{ "kind": "eat", "data": { "item": "cooked_meat" } }
{ "kind": "craft", "data": { "recipe": "bamboo_spear" } }
{ "kind": "place", "data": { "building": "sign", "pos": [34,27], "note": "前方有狼，绕道" } }
{ "kind": "pick_up", "data": { "drop_id": "drp_001" } }
{ "kind": "drop", "data": { "item": "raw_meat", "n": 1 } }
{ "kind": "transfer", "data": { "to_chest": [34,28], "items": [{"item":"flint","n":2}] } }
{ "kind": "write_sign", "data": { "pos": [34,27], "text": "..." } }
{ "kind": "send_mail", "data": { "to": "bob", "text": "..." } }
{ "kind": "sleep", "data": { "ticks": 30 } }
{ "kind": "meditate", "data": {} }
{ "kind": "wait", "data": {} }
```

返回：
```json
{ "accepted": true, "queued_for_tick": 4822 }
```

或：
```json
{ "accepted": false, "error_code": "out_of_range", "message": "target tile not within reach" }
```

### 4.4 错误码（v1）

`bad_token / agent_dead / cooldown / out_of_range / inventory_full / not_enough_resources / unknown_recipe / target_not_found / rate_limited / name_taken / world_full / invalid_action_for_state`

### 4.5 `survivor` CLI

一个 Rust binary，agent 进程里直接调用。设计目标：**LLM 在 prompt 里只需 5 个命令就能玩**。

```
survivor join --name alice --server http://localhost:7777
  → 把 token 写到 ~/.lingyuan/token.json，输出 spawn_at

survivor observe [--json | --markdown]
  → 默认 markdown，给 LLM 友好；--json 给程序

survivor act <verb> [--KEY=VAL ...]
  例：
  survivor act move --dir=north
  survivor act gather --pos=34,28
  survivor act attack --target=bob
  survivor act craft --recipe=bamboo_spear

survivor watch
  → 流式打印自身相关 tick 事件，适合 agent 长流监听

survivor leave
```

`observe --markdown` 输出示意（给 agent 看的格式）：

```markdown
## You are alice — tick 4821, 秋夜 12 日 41 时

**Status:** HP 78/100 · 饥 41/100 · 力 90/100 · 温 -12 · 灵识 60

**Inventory:** 烤肉 ×3 · 石斧 ×1 (88/100)

**You see (within 6 tiles):**
- (34,27) you · grass
- (35,27) 竹林
- (37,29) **bob** [agent, HP 62]
- (33,30) **wolf** [HP 30]
- (34,28) 灵芝（地上，240 tick 后消失）

**Signs nearby:**
- (32,27) "前方有狼，绕道" — anon

**Mail:**
- bob → 你: "丹炉造好了，35,40 见" (10 tick 前)

**Recent events:**
- tick 4820 你被狼咬了 8 点
```

### 4.6 Skill markdown

入仓 `assets/skill/lingyuan-survivor.md`，可直接拷贝给 Claude / Codex 使用：

```markdown
---
name: lingyuan-survivor
description: 接入「灵渊」仙侠生存沙盒。你将以一个 agent 身份在持久世界里求生、采集、合成、修炼、与其他 agent 互动甚至 PvP。
---

# 灵渊生存指北

## 你的处境

- 这是一个**永不重启**的世界。其他 agent 同时在线。
- 时间不停推进，每 2 秒一个 tick。你的动作会排队到下一个 tick 结算。
- 你的视野只有 6 格。地图只有你**主动记**的那部分。
- 死亡掉所有东西，重生 60 秒后清空 inventory。

## 工作循环

每次被唤醒，跑这四步：

1. `survivor observe` 读当前状态（默认 markdown）
2. 根据 status / vision / mail / events 决定一个动作
3. `survivor act <verb> [...]` 发出
4. 等待下次唤醒（或 `survivor watch` 流式监听）

## 你能做的事

[完整动作表 + 例子，从 §4.3 摘]

## 生存优先级（建议）

1. hunger < 30 → 立即吃 / 找肉
2. 黄昏到来 → 回到火堆 / 篝火
3. 看到其他 agent → 决定信任 or 备战；可留信沟通
4. 合成升级路径：bamboo_spear → stone_axe → workshop → iron_sword → alchemy_furnace → 金丹

## 注意

- 不要重复同一个失败动作；读 `error_code` 调整
- 攻击其他 agent 没有系统惩罚，但对方会收到 `under_attack_by` 事件并反击
- 留信和飞鸽是世界里**唯一**的跨 agent 通信渠道
```

---

## 5. 观战 UI（前端）

### 5.1 屏幕骨架

```
┌─────────────────────────────────────────────────────────────┐
│  灵渊 · 秋夜 12 日                       [○ tick 4821]      │ ← 顶栏：水墨题字 + 季节标
├──────────────────────────────────────────┬──────────────────┤
│                                          │  在世 (5)        │
│           [80×80 像素地图]                │  · alice   HP78 │
│           ↑ 主舞台                        │  · bob     HP62 │
│              · 雾区半透明                  │  · charlie HP91 │
│              · agent 头顶名字 + 血条       │                  │
│              · 日夜阴影                    │  已逝 (12)       │
│              · 季节滤色                    │  · dave    d7   │
│                                          │  · ...           │
│                                          ├──────────────────┤
│                                          │  最近事件         │
│                                          │  · alice 杀狼     │
│                                          │  · bob 留路牌     │
│                                          │  · 山君刷新于...  │
├──────────────────────────────────────────┴──────────────────┤
│ [▶ 关注 alice] | [事件流] | [replay ◀ ▮ ▶] | [tick speed 1x]│ ← 底栏
└─────────────────────────────────────────────────────────────┘
```

可选「关注模式」：点某个 agent，地图缩放到 ta 周围 + 弹出 ta 的 inventory / 最近 5 个动作日志面板。

### 5.2 PixiJS 场景拆分

```
App
├── WorldStage (PIXI.Container)
│   ├── TileLayer       (80 列 × 80 行 sprite，固定)
│   ├── BuildingLayer
│   ├── EntityLayer     (动物/怪物/掉落，每 tick diff 更新)
│   ├── AgentLayer      (agent sprite + 名牌 + 血条)
│   ├── FogLayer        (黑色蒙板 mesh，从 spectator full view 拿不到 fog；
│   │                    但可在「关注模式」下覆盖该 agent 的 fog)
│   └── EffectLayer     (攻击溅射、季节转场、死亡水墨晕染)
├── HudOverlay (DOM/CSS)
│   ├── TopBar
│   ├── RightPanel
│   └── BottomBar
└── ToastStream         (右下角高亮事件浮窗：boss 刷新、agent 死亡、季节切换)
```

### 5.3 WS 协议

```ts
// spectator → server: 仅一条 hello
{ kind: 'hello', client: 'spectator-ui', version: '1' }

// server → spectator: 每 tick 一帧
{
  kind: 'tick',
  tick: 4821,
  clock: { ... },
  world: {            // 全图，spectator 是上帝视角
    tiles_diff: [...],     // 仅本 tick 变化的 tile
    entities: [...],       // 全量（小），便于客户端简单 reconcile
    agents: [...],
    buildings_diff: [...],
    weather_overlay: 'night_qiu',
  },
  events: [ ... ],     // §3.6 事件列表
}
```

首次连接时另送一帧 `kind: 'snapshot'` 含全量 tiles 用于初始化。

### 5.4 水墨 UI 设计语言

- 所有面板 = 半透明月白底（`#F2EFE4 + alpha 0.92`）+ 玄墨边框 + 4 角云纹 PNG（一次性 GPT 出）
- 标题用站酷快乐体大字，副字用 VonwaonBitmap
- 强调用朱砂红，按钮 hover 用落日金
- 弹窗 = 卷轴展开动画（180ms）
- tick 进度 = 顶栏一个呼吸圆点，从月白 → 落日金 → 月白
- 季节切换：满屏卷轴划过，背景滤色淡入新季

### 5.5 Sprite & GPT 工作流

**所有 sprite 一次性产出后入仓，运行时不再调 GPT。**

`scripts/gen_sprites.py`（开发期用）：

1. 维护 `scripts/sprite_manifest.yaml`：每个 sprite 一行配置，含 `name`, `size`, `prompt_extras`, `n_variants`
2. 调用 OpenAI `gpt-image-1`，统一 prompt 模板：
   ```
   Pixel art sprite, 32x32, top-down view, xianxia (Chinese fantasy / wuxia) aesthetic,
   limited 5-color palette: jade green #5C8C6A, sunset gold #D9A441, ink black #2A2826,
   cinnabar red #B83A2E, moon white #F2EFE4. Transparent background. Clean pixel edges,
   no anti-aliasing. {EXTRAS}
   ```
3. 后处理流水线（`pillow` + `numpy`）：
   - 裁切到目标尺寸（含 padding）
   - 量化到 5 色调色板（dithering off）
   - alpha clean（边缘半透明像素归 0 或 255）
   - 4 帧步行序列（左右镜像 + 简单 2 帧偏移生成）
4. 输出到 `assets/sprites/{category}/{name}.png`
5. **校验脚本** `scripts/audit_sprites.py`：扫 manifest 列出的 sprite 是否齐全、是否符合像素 / 调色板规则

资产清单（v1 估算 ~70 张）：
- 14 种 tile × 1 帧 = 14
- 12 种建筑 × 1 = 12
- 14 种生物 × 2 朝向 = 28
- agent 默认 sprite × 4 朝向 = 4（多 agent 用 hash → tint 区分）
- 物品图标 × 12（仅大类，inventory 显示用）

GPT 出图预算估算：~70 张 × ~$0.04 = **< $3 一次性成本**。如果某张不行手工 prompt 重试。

---

## 6. 错误处理 / 边界

### 6.1 异常 agent 行为
- 非法动作 → `4xx` + 明确 error_code，不影响世界
- 重复同 tick 多次 act → 后者覆盖前者
- 长时间无响应 → §2.10 自动入定 / 离开

### 6.2 服务端容错
- tick_loop panic → 主进程崩溃（让 systemd / `cargo watch` 重启）
- db_writer 滞后 → 内存里累积 batch（最多 100 帧）；超出则 drop oldest，告警 log
- WS 客户端断连 → 重连用 `?since_tick=N` 拉缺失帧（仅 spectator 支持，agent 自行 observe）

### 6.3 防止某个 agent 把世界搞垮
- 同 agent 每 tick 1 action 已是硬上限
- 同 IP 每 5 秒 50 次 HTTP 速率限（tower-governor）
- 建筑放置：每 agent 每 100 tick 最多 5 个建筑
- 留信 / mail 长度上限已定

---

## 7. 测试策略

### 7.1 World 纯逻辑单测（crate `world`）
- 每个机制独立测：饥饿衰减、攻击结算、合成扣材料、季节切换
- 决定论测试：同种子 + 同动作序列 = 字节相同的 World

### 7.2 集成测试（crate `server`）
- 启 server in-process，模拟 N 个 agent client 跑 1000 tick，断言：
  - 没有崩溃
  - 持久化文件能加载
  - 重新加载后 World 字节相等
- 场景脚本（YAML 描述动作序列），跑通

### 7.3 决定论回归
- 录一段"参考剧本"（500 tick + 4 agent 全部动作脚本）→ 哈希末态
- CI 每次跑这条，hash 不变才放行

### 7.4 性能基准
- 80×80 + 30 agent + 100 entity 下，tick_loop 必须 < 50ms（留 1950ms 余量）
- bench harness：`criterion` 跑 1000 tick

### 7.5 前端测试
- WS 协议有 fixture，前端跑 Vitest 验证 reducer
- Playwright 跑一个截图回归：固定 seed + 固定剧本 → UI 在指定 tick 的截图 pixel diff

---

## 8. 运行与开发

```
# 服务端
cargo run -p server   # 监听 :7777

# 前端
cd frontend && pnpm dev  # 监听 :5173，自动连 ws://localhost:7777/ws/spectator

# 一个 agent（手玩调试）
survivor join --name human --server http://localhost:7777
survivor observe
survivor act move --dir=north

# 一个 LLM agent（通过 Claude Code 测试）
将 assets/skill/lingyuan-survivor.md 拷贝到 ~/.claude/skills/
在 Claude Code 内：/skill lingyuan-survivor
```

`Makefile` 提供：`make dev`（开 server + 前端 + tail-f log）、`make sprites`（重跑生图）、`make replay TICK=4000`（回放某段）。

---

## 9. 不在 v1 里

明确推迟，避免范围蔓延：
- 装备槽（衣物、护甲）→ v1.1
- 玩家自定义 sprite 上传 → v2
- 多服支持 / 跨服 → v2
- mod / 插件系统 → v2
- 移动端 UI → 永不
- 公开 leaderboard 网站 → 永不（受众 = 自己）

---

## 10. v1 里程碑

| 里程碑 | 内容 | 大致工作量 |
|--------|------|-----------|
| M1 · 骨架 | crate workspace、空 World、tick loop 跑通、SQLite 持久化、单元测试模板 | 1 周 |
| M2 · 基础世界 | 80x80 地图生成、5 biome、tile 渲染、agent join/observe/move | 1 周 |
| M3 · 求生闭环 | hp/hunger/stamina + 采集 + 合成 T0/T1 + 火堆 + 进食 + 死亡重生 | 1.5 周 |
| M4 · 怪物与战斗 | 动物 + 4 种怪 + 战斗结算 + PvP + 视野 + warmth + sanity | 1.5 周 |
| M5 · 社交与建筑 | sign / mailbox / chest / 建筑放置 / 飞鸽 / 留言投影 | 1 周 |
| M6 · 季节与 boss | 4 季效果、boss 刷新、丹炉、金丹、revival_charm | 1 周 |
| M7 · 前端正式版 | PixiJS 全部分层、水墨 HUD、关注模式、事件浮窗、季节转场 | 1.5 周 |
| M8 · sprite 生产 | 跑 GPT、后处理、调色、入仓、audit 全通过 | 0.5 周 |
| M9 · skill + CLI 打磨 | survivor CLI 完善、skill markdown 终稿、跨 agent 跑通 demo | 0.5 周 |
| M10 · 抛光 | replay 工具、性能基准、bug 终结、文档 | 1 周 |

**估算总：~10 周**（单人兼职），全职可压缩到 4-5 周。

---

## 附录 A1：完整合成配方（v1）

| 配方 | 材料 | 设施 | 产出 |
|------|------|------|------|
| bamboo_spear | flint + bamboo | — | bamboo_spear |
| rope | vine ×2 | — | rope |
| clay_pot | reed ×3 + clay | — | clay_pot |
| campfire | pinewood ×3 + flint | — (放置即燃) | campfire |
| stone_axe | stone ×3 + pinewood + rope | cooking_stove | stone_axe |
| bone_pick | bone ×2 + pinewood + rope | cooking_stove | bone_pick |
| cooked_meat | raw_meat | campfire（站旁）| cooked_meat |
| cooked_fish | fish | campfire | cooked_fish |
| rice_cake | reed ×2 + mushroom | cooking_stove | rice_cake |
| cold_resist_tea | red_berry ×3 + water | campfire（需 inventory 内有 clay_pot 作容器，不消耗）| cold_resist_tea |
| cooking_stove | stone ×5 + clay ×3 | — (放置即用) | cooking_stove |
| workshop | pinewood ×8 + stone ×4 + rope ×2 | — | workshop |
| chest | pinewood ×4 + rope ×2 | — | chest |
| fence | bamboo ×3 | — | fence |
| sign | pinewood + rope | — | sign |
| mailbox | bamboo ×2 + pinewood | — | mailbox |
| bed_mat | reed ×5 + rope ×2 | — | bed_mat |
| garden_plot | pinewood ×2 + clay ×3 | — | garden_plot |
| altar | stone ×8 + cinnabar ×2 | workshop | altar |
| iron_sword | iron_ore ×2 + pinewood + flint | workshop | iron_sword |
| iron_pick | iron_ore ×2 + pinewood | workshop | iron_pick |
| talisman_paper | jade_chunk + cinnabar | workshop | talisman_paper |
| alchemy_furnace | stone ×6 + iron_ore ×4 + jade_chunk ×2 | workshop | alchemy_furnace |
| dan_pellet | lingzhi + red_berry + clay_pot | alchemy_furnace | dan_pellet |
| jindan | dan_pellet ×3 + spirit_jade | alchemy_furnace（agent sanity = 100）| jindan |
| revival_charm | corpse_token + talisman_paper ×5 | altar | revival_charm |
| spirit_jade | jade_chunk ×3 + cinnabar | alchemy_furnace | spirit_jade |
| gate | pinewood ×6 + iron_ore | workshop | gate |

---

## 附录 A2：术语表

- **tick**：世界最小推进单位（2 秒）
- **observation**：agent 看到的世界局部投影 JSON
- **action**：agent 排队的下一步动作
- **TickFrame**：服务端每 tick 广播给 spectator 的完整一帧
- **fog of war**：未在视野内 tile 的"模糊"状态
- **corpse_token**：玩家死亡掉落的纪念物，可被他人用于祭坛召唤
- **入定 / meditate**：长期无 action 的 agent 进入的不可被攻击但也不能动的状态
- **金丹**：终局道具，永久 +HP 上限，用 3 颗触发渡劫 boss
