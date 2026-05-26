# 灵渊 M1+M2 实施计划：骨架 + 基础世界

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 Rust workspace、World 纯逻辑核心、axum 服务端、SQLite 持久化、`survivor` CLI、PixiJS 前端骨架全部跑通，能在浏览器看到 80×80 仙侠像素地图，多个 agent 通过 CLI join 进来、移动、observe，且服务端崩溃后能从持久化恢复世界。

**Architecture:** 单 Rust workspace 三 crate（`world` 纯逻辑、`server` 网络+持久化、`cli` agent 客户端）+ 独立 Vite/PixiJS 前端。`World` 由 tick_loop 单 task 独占，动作通过 mpsc 排队，每 2s 推一帧，事件 + 周期 snapshot 写 SQLite，WS 广播给前端。

**Tech Stack:** Rust 1.75+ / axum 0.7 / tokio 1.36 / sqlx 0.7 (sqlite) / serde + bincode / rand_chacha / noise 0.8（地图生成）/ tracing；前端 Vite 5 + TypeScript + PixiJS 8。

---

## File Structure

```
lingyuan/
├── Cargo.toml                         # workspace 根
├── rust-toolchain.toml                # 锁 rustc 版本
├── crates/
│   ├── world/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                 # 重导出
│   │       ├── coord.rs               # TileCoord, Direction
│   │       ├── tile.rs                # Tile, Biome 枚举
│   │       ├── grid.rs                # Grid<T> 容器
│   │       ├── clock.rs               # WorldClock（日/季/年）
│   │       ├── agent.rs               # Agent, AgentId, AgentStatus
│   │       ├── action.rs              # Action 枚举
│   │       ├── event.rs               # TickEvent
│   │       ├── observation.rs         # Observation 投影
│   │       ├── world.rs               # World 主结构 + step
│   │       ├── gen.rs                 # 地图种子化生成
│   │       └── rng.rs                 # ChaCha8 决定论辅助
│   ├── server/
│   │   ├── Cargo.toml
│   │   ├── migrations/
│   │   │   └── 0001_initial.sql
│   │   └── src/
│   │       ├── main.rs
│   │       ├── config.rs              # 端口、tick 速率、DB 路径
│   │       ├── state.rs               # AppState
│   │       ├── tick_loop.rs           # 主游戏循环
│   │       ├── db.rs                  # SQLite 接入 + writer task
│   │       ├── persistence.rs         # snapshot / event 序列化
│   │       ├── auth.rs                # token 校验
│   │       └── routes/
│   │           ├── mod.rs
│   │           ├── health.rs
│   │           ├── join.rs
│   │           ├── observe.rs
│   │           ├── act.rs
│   │           ├── leave.rs
│   │           ├── clock.rs
│   │           └── ws.rs              # /ws/spectator + /ws/agent
│   └── cli/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands.rs            # clap 子命令枚举
│           ├── client.rs              # HTTP client wrapper
│           ├── token_store.rs         # ~/.lingyuan/token.json
│           └── render.rs              # observe --markdown 渲染
├── frontend/
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   └── src/
│       ├── main.ts                    # 应用入口
│       ├── ws.ts                      # WS 客户端 + reconnect
│       ├── types.ts                   # 与 server 共享的 JSON 类型
│       ├── stage/
│       │   ├── world-stage.ts
│       │   ├── tile-layer.ts
│       │   ├── agent-layer.ts
│       │   └── camera.ts
│       └── hud/
│           ├── top-bar.ts
│           ├── right-panel.ts
│           └── style.css
├── docs/                              # spec + plans（已存在）
├── data/                              # 运行时 SQLite（gitignore）
└── .gitignore
```

**Responsibilities:**
- `world::*` — 纯逻辑，无 IO，无 async；World 的 step 函数是确定性的纯函数
- `server::tick_loop` — 唯一持有 World 的 task，2s/tick；其他任务通过 mpsc/broadcast 通信
- `server::routes` — 薄薄一层，handler 把请求转译成 ActionEnvelope/查询，不写业务
- `server::db` — 独立 task，从 mpsc 取写请求，IO 不阻塞 tick_loop
- `cli` — 仅 HTTP client + 渲染，无游戏逻辑
- `frontend` — PixiJS 渲染 + WS 客户端，无业务规则

---

## Task 0: Repo & toolchain 初始化

**Files:**
- Create: `/Users/e0_7/projects/games/lingyuan/Cargo.toml`
- Create: `/Users/e0_7/projects/games/lingyuan/rust-toolchain.toml`
- Create: `/Users/e0_7/projects/games/lingyuan/.gitignore`（已部分存在，覆盖）
- Create: `/Users/e0_7/projects/games/lingyuan/README.md`

- [ ] **Step 1: 写 workspace Cargo.toml**

```toml
[workspace]
members = ["crates/world", "crates/server", "crates/cli"]
resolver = "2"

[workspace.package]
edition = "2021"
rust-version = "1.75"
license = "MIT"

[workspace.dependencies]
anyhow = "1"
thiserror = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
bincode = "1.3"
tokio = { version = "1.36", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
rand = "0.8"
rand_chacha = "0.3"
noise = "0.8"
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 2: 写 rust-toolchain.toml**

```toml
[toolchain]
channel = "1.75.0"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 3: 覆盖 .gitignore**

```
target/
node_modules/
data/
dist/
.DS_Store
*.log
.env
.lingyuan/
```

- [ ] **Step 4: 写 README.md**

```markdown
# 灵渊 (Lingyuan)

多 agent 仙侠像素生存沙盒。详见 `docs/superpowers/specs/2026-05-27-lingyuan-design.md`。

## Quick start

```bash
cargo run -p server    # 开服 :7777
cd frontend && pnpm dev # 开观战 :5173
cargo run -p cli -- join --name alice --server http://localhost:7777
```
```

- [ ] **Step 5: 校验 + commit**

Run: `cd /Users/e0_7/projects/games/lingyuan && cargo --version`
Expected: 输出 1.75+ 版本。

```bash
cd /Users/e0_7/projects/games/lingyuan
git add Cargo.toml rust-toolchain.toml .gitignore README.md
git commit -m "chore: workspace 初始化"
```

---

## Task 1: crate `world` 骨架 + 坐标与方向

**Files:**
- Create: `crates/world/Cargo.toml`
- Create: `crates/world/src/lib.rs`
- Create: `crates/world/src/coord.rs`
- Test: `crates/world/src/coord.rs`（inline `#[cfg(test)]`）

- [ ] **Step 1: 写 crate Cargo.toml**

```toml
[package]
name = "world"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
bincode.workspace = true
rand.workspace = true
rand_chacha.workspace = true
noise.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

- [ ] **Step 2: 写 lib.rs 重导出**

```rust
pub mod coord;
pub mod tile;
pub mod grid;
pub mod clock;
pub mod agent;
pub mod action;
pub mod event;
pub mod observation;
pub mod rng;
pub mod gen;
pub mod world;

pub use coord::{Direction, TileCoord};
pub use tile::{Biome, Tile, TileKind};
pub use clock::{Season, WorldClock, DayPhase};
pub use agent::{Agent, AgentId, AgentState, AgentStatus};
pub use action::Action;
pub use event::TickEvent;
pub use observation::Observation;
pub use world::World;
```

- [ ] **Step 3: 写 coord.rs（先放失败测试）**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileCoord {
    pub x: i16,
    pub y: i16,
}

impl TileCoord {
    pub const fn new(x: i16, y: i16) -> Self { Self { x, y } }
    pub fn manhattan(self, other: TileCoord) -> u16 {
        ((self.x - other.x).abs() + (self.y - other.y).abs()) as u16
    }
    pub fn step(self, dir: Direction) -> TileCoord {
        let (dx, dy) = dir.delta();
        TileCoord::new(self.x + dx, self.y + dy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North, South, East, West,
}

impl Direction {
    pub const ALL: [Direction; 4] = [Direction::North, Direction::South, Direction::East, Direction::West];
    pub fn delta(self) -> (i16, i16) {
        match self {
            Direction::North => (0, -1),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
            Direction::West => (-1, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manhattan_basic() {
        assert_eq!(TileCoord::new(0,0).manhattan(TileCoord::new(3,4)), 7);
    }
    #[test]
    fn step_north_decreases_y() {
        assert_eq!(TileCoord::new(5,5).step(Direction::North), TileCoord::new(5,4));
    }
}
```

- [ ] **Step 4: 占位其余模块（空文件）防止 lib.rs 编译失败**

每个模块写：`// stub` 一行。文件：`tile.rs grid.rs clock.rs agent.rs action.rs event.rs observation.rs rng.rs gen.rs world.rs`。后续 task 会填。

- [ ] **Step 5: cargo build + test**

Run: `cd /Users/e0_7/projects/games/lingyuan && cargo build -p world && cargo test -p world coord`
Expected: 2 tests pass.

- [ ] **Step 6: commit**

```bash
git add crates/world Cargo.lock
git commit -m "feat(world): coord + direction"
```

---

## Task 2: Tile + Biome + Grid

**Files:**
- Modify: `crates/world/src/tile.rs`
- Modify: `crates/world/src/grid.rs`

- [ ] **Step 1: 实现 tile.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Biome {
    Qingzhu,   // 青竹林
    Cangsong,  // 苍松岭
    Yueze,     // 月泽
    Zhuyang,   // 朱阳谷
    Heishi,    // 黑石冢
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileKind {
    Grass,
    BambooForest,
    PineForest,
    Reed,
    Maple,
    Sand,
    Stone,
    Mountain,
    ShallowWater,
    DeepWater,
    Ruin,
    Road,
    Ash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub kind: TileKind,
    pub biome: Biome,
}

impl Tile {
    pub fn is_walkable(&self) -> bool {
        !matches!(self.kind, TileKind::Mountain | TileKind::DeepWater)
    }
    pub fn blocks_vision(&self) -> bool {
        matches!(self.kind, TileKind::Mountain | TileKind::BambooForest | TileKind::PineForest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mountain_blocks_walk_and_vision() {
        let t = Tile { kind: TileKind::Mountain, biome: Biome::Cangsong };
        assert!(!t.is_walkable());
        assert!(t.blocks_vision());
    }
}
```

- [ ] **Step 2: 实现 grid.rs**

```rust
use serde::{Deserialize, Serialize};
use crate::coord::TileCoord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid<T> {
    pub width: u16,
    pub height: u16,
    cells: Vec<T>,
}

impl<T: Clone> Grid<T> {
    pub fn filled(width: u16, height: u16, value: T) -> Self {
        Self { width, height, cells: vec![value; (width as usize) * (height as usize)] }
    }
    fn index(&self, c: TileCoord) -> Option<usize> {
        if c.x < 0 || c.y < 0 || c.x >= self.width as i16 || c.y >= self.height as i16 { return None; }
        Some(c.y as usize * self.width as usize + c.x as usize)
    }
    pub fn in_bounds(&self, c: TileCoord) -> bool { self.index(c).is_some() }
    pub fn get(&self, c: TileCoord) -> Option<&T> { self.index(c).map(|i| &self.cells[i]) }
    pub fn set(&mut self, c: TileCoord, v: T) -> bool {
        if let Some(i) = self.index(c) { self.cells[i] = v; true } else { false }
    }
    pub fn iter(&self) -> impl Iterator<Item = (TileCoord, &T)> + '_ {
        let w = self.width as i16;
        self.cells.iter().enumerate().map(move |(i, v)| {
            let x = (i as i16) % w;
            let y = (i as i16) / w;
            (TileCoord::new(x, y), v)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::TileCoord;
    #[test]
    fn out_of_bounds_returns_none() {
        let g = Grid::filled(10, 10, 0u8);
        assert!(g.get(TileCoord::new(-1, 5)).is_none());
        assert!(g.get(TileCoord::new(5, 10)).is_none());
        assert!(g.get(TileCoord::new(0, 0)).is_some());
    }
    #[test]
    fn set_then_get_roundtrip() {
        let mut g = Grid::filled(5, 5, 0u8);
        g.set(TileCoord::new(2, 3), 7);
        assert_eq!(g.get(TileCoord::new(2, 3)), Some(&7));
    }
}
```

- [ ] **Step 3: cargo test**

Run: `cargo test -p world tile grid`
Expected: 3 tests pass (mountain test + 2 grid tests).

- [ ] **Step 4: commit**

```bash
git add crates/world/src/tile.rs crates/world/src/grid.rs
git commit -m "feat(world): tile + grid"
```

---

## Task 3: WorldClock + Season

**Files:**
- Modify: `crates/world/src/clock.rs`

- [ ] **Step 1: 实现 clock.rs（含测试）**

```rust
use serde::{Deserialize, Serialize};

pub const TICKS_PER_DAY: u32 = 72;        // 30 白 + 6 黄昏 + 30 夜 + 6 拂晓
pub const DAYS_PER_SEASON: u32 = 10;
pub const SEASONS_PER_YEAR: u32 = 4;
pub const TICKS_PER_SEASON: u32 = TICKS_PER_DAY * DAYS_PER_SEASON; // 720
pub const TICKS_PER_YEAR: u32 = TICKS_PER_SEASON * SEASONS_PER_YEAR; // 2880

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Season { Chun, Xia, Qiu, Dong }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DayPhase { Day, Dusk, Night, Dawn }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldClock {
    pub tick: u64,
}

impl WorldClock {
    pub fn new() -> Self { Self { tick: 0 } }
    pub fn advance(&mut self) { self.tick += 1; }
    pub fn tick_in_day(&self) -> u32 { (self.tick as u32) % TICKS_PER_DAY }
    pub fn day_in_season(&self) -> u32 {
        (self.tick as u32 / TICKS_PER_DAY) % DAYS_PER_SEASON
    }
    pub fn season(&self) -> Season {
        let s = (self.tick as u32 / TICKS_PER_SEASON) % SEASONS_PER_YEAR;
        match s { 0 => Season::Chun, 1 => Season::Xia, 2 => Season::Qiu, _ => Season::Dong }
    }
    pub fn year(&self) -> u32 { self.tick as u32 / TICKS_PER_YEAR }
    pub fn phase(&self) -> DayPhase {
        match self.tick_in_day() {
            0..=29 => DayPhase::Day,
            30..=35 => DayPhase::Dusk,
            36..=65 => DayPhase::Night,
            _ => DayPhase::Dawn,
        }
    }
    pub fn is_night(&self) -> bool { matches!(self.phase(), DayPhase::Night) }
    pub fn just_changed_season(&self) -> bool {
        self.tick > 0 && (self.tick as u32) % TICKS_PER_SEASON == 0
    }
}

impl Default for WorldClock { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phase_day_then_night() {
        let mut c = WorldClock::new();
        assert_eq!(c.phase(), DayPhase::Day);
        for _ in 0..36 { c.advance(); }
        assert_eq!(c.phase(), DayPhase::Night);
    }
    #[test]
    fn season_cycle() {
        let mut c = WorldClock::new();
        assert_eq!(c.season(), Season::Chun);
        for _ in 0..TICKS_PER_SEASON { c.advance(); }
        assert_eq!(c.season(), Season::Xia);
        assert!(c.just_changed_season());
    }
    #[test]
    fn year_advances() {
        let mut c = WorldClock::new();
        for _ in 0..TICKS_PER_YEAR { c.advance(); }
        assert_eq!(c.year(), 1);
    }
}
```

- [ ] **Step 2: cargo test**

Run: `cargo test -p world clock`
Expected: 3 tests pass.

- [ ] **Step 3: commit**

```bash
git add crates/world/src/clock.rs
git commit -m "feat(world): clock + season"
```

---

## Task 4: Deterministic RNG helper

**Files:**
- Modify: `crates/world/src/rng.rs`

- [ ] **Step 1: 实现 rng.rs**

```rust
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;

pub fn rng_for(world_seed: u64, tick: u64, salt: u64) -> ChaCha8Rng {
    let s = world_seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(tick.wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(salt);
    ChaCha8Rng::seed_from_u64(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    #[test]
    fn same_inputs_same_sequence() {
        let mut a = rng_for(42, 100, 7);
        let mut b = rng_for(42, 100, 7);
        for _ in 0..16 {
            let x: u64 = a.gen();
            let y: u64 = b.gen();
            assert_eq!(x, y);
        }
    }
    #[test]
    fn different_salt_differs() {
        let mut a = rng_for(42, 100, 7);
        let mut b = rng_for(42, 100, 8);
        let x: u64 = a.gen();
        let y: u64 = b.gen();
        assert_ne!(x, y);
    }
}
```

- [ ] **Step 2: cargo test + commit**

Run: `cargo test -p world rng`
Expected: 2 tests pass.

```bash
git add crates/world/src/rng.rs
git commit -m "feat(world): deterministic RNG helper"
```

---

## Task 5: Action + TickEvent 枚举

**Files:**
- Modify: `crates/world/src/action.rs`
- Modify: `crates/world/src/event.rs`

- [ ] **Step 1: action.rs**

```rust
use serde::{Deserialize, Serialize};
use crate::coord::{Direction, TileCoord};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Action {
    Move { dir: Direction },
    Wait,
    Observe,  // 仅占位
    // 后续 milestone 加 Gather/Attack/Eat/Craft/...
}
```

- [ ] **Step 2: event.rs**

```rust
use serde::{Deserialize, Serialize};
use crate::{AgentId, TileCoord};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum TickEvent {
    AgentJoined { agent: AgentId, name: String, at: TileCoord },
    AgentLeft { agent: AgentId, name: String },
    AgentMoved { agent: AgentId, from: TileCoord, to: TileCoord },
    AgentMoveFailed { agent: AgentId, reason: String },
    SeasonChanged { to: crate::clock::Season },
    DayStarted { day: u32 },
    NightStarted { day: u32 },
}
```

- [ ] **Step 3: cargo build**

Run: `cargo build -p world`
Expected: 编译通过（agent.rs 还是 stub，下个 task 填）。
若失败：意味着 event.rs 引的 AgentId 未定义；先把 agent.rs stub 改成：

```rust
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);
```

再 `cargo build -p world`。

- [ ] **Step 4: commit**

```bash
git add crates/world/src/action.rs crates/world/src/event.rs crates/world/src/agent.rs
git commit -m "feat(world): action + event + agent id stub"
```

---

## Task 6: Agent 完整定义

**Files:**
- Modify: `crates/world/src/agent.rs`

- [ ] **Step 1: 完整 agent.rs**

```rust
use serde::{Deserialize, Serialize};
use crate::coord::TileCoord;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentStatus {
    pub hp: i16,
    pub hunger: i16,
    pub stamina: i16,
    pub warmth: i16,
    pub sanity: i16,
}

impl AgentStatus {
    pub fn fresh() -> Self {
        Self { hp: 100, hunger: 100, stamina: 100, warmth: 0, sanity: 100 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum AgentState {
    Alive,
    Dying { revives_at_tick: u64 },
    Meditating { since_tick: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub pos: TileCoord,
    pub status: AgentStatus,
    pub state: AgentState,
    pub last_action_tick: u64,
    pub joined_tick: u64,
}

impl Agent {
    pub fn new_at(id: AgentId, name: String, pos: TileCoord, tick: u64) -> Self {
        Self {
            id, name, pos,
            status: AgentStatus::fresh(),
            state: AgentState::Alive,
            last_action_tick: tick,
            joined_tick: tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_status_full() {
        let s = AgentStatus::fresh();
        assert_eq!(s.hp, 100);
        assert_eq!(s.hunger, 100);
    }
}
```

- [ ] **Step 2: cargo test + commit**

Run: `cargo test -p world agent`
Expected: 1 test passes.

```bash
git add crates/world/src/agent.rs
git commit -m "feat(world): agent struct"
```

---

## Task 7: 地图生成（gen.rs）

**Files:**
- Modify: `crates/world/src/gen.rs`

- [ ] **Step 1: 实现 gen.rs**

```rust
use noise::{NoiseFn, Perlin};
use crate::{Biome, Grid, Tile, TileCoord, TileKind};

pub const WORLD_WIDTH: u16 = 80;
pub const WORLD_HEIGHT: u16 = 80;

pub fn generate(seed: u64) -> Grid<Tile> {
    let biome_noise = Perlin::new((seed & 0xFFFF_FFFF) as u32);
    let detail_noise = Perlin::new(((seed >> 32) & 0xFFFF_FFFF) as u32);
    let mut g = Grid::filled(
        WORLD_WIDTH, WORLD_HEIGHT,
        Tile { kind: TileKind::Grass, biome: Biome::Qingzhu },
    );
    for y in 0..WORLD_HEIGHT as i16 {
        for x in 0..WORLD_WIDTH as i16 {
            let nx = x as f64 / 18.0;
            let ny = y as f64 / 18.0;
            let b = biome_noise.get([nx, ny]);
            let d = detail_noise.get([nx * 3.0, ny * 3.0]);
            let biome = biome_from_noise(b);
            let kind = tile_kind_for(biome, d);
            g.set(TileCoord::new(x, y), Tile { kind, biome });
        }
    }
    g
}

fn biome_from_noise(v: f64) -> Biome {
    // v ∈ [-1, 1] 大致分 5 段
    match v {
        x if x < -0.5 => Biome::Yueze,
        x if x < -0.1 => Biome::Qingzhu,
        x if x <  0.2 => Biome::Cangsong,
        x if x <  0.6 => Biome::Zhuyang,
        _ => Biome::Heishi,
    }
}

fn tile_kind_for(biome: Biome, d: f64) -> TileKind {
    match biome {
        Biome::Qingzhu => if d > 0.3 { TileKind::BambooForest } else { TileKind::Grass },
        Biome::Cangsong => if d > 0.3 { TileKind::PineForest } else if d < -0.5 { TileKind::Mountain } else { TileKind::Stone },
        Biome::Yueze => if d > 0.0 { TileKind::Reed } else if d < -0.4 { TileKind::DeepWater } else { TileKind::ShallowWater },
        Biome::Zhuyang => if d > 0.3 { TileKind::Maple } else if d < -0.4 { TileKind::Sand } else { TileKind::Grass },
        Biome::Heishi => if d > 0.3 { TileKind::Ruin } else if d < -0.4 { TileKind::Mountain } else { TileKind::Ash },
    }
}

pub fn find_safe_spawn(grid: &Grid<Tile>, seed: u64) -> TileCoord {
    use rand::{Rng, SeedableRng};
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed ^ 0xDEAD_BEEF);
    for _ in 0..1000 {
        let x = rng.gen_range(0..WORLD_WIDTH) as i16;
        let y = rng.gen_range(0..WORLD_HEIGHT) as i16;
        let c = TileCoord::new(x, y);
        if let Some(t) = grid.get(c) {
            if t.is_walkable() { return c; }
        }
    }
    TileCoord::new(WORLD_WIDTH as i16 / 2, WORLD_HEIGHT as i16 / 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn determinism() {
        let a = generate(123);
        let b = generate(123);
        for ((_, ta), (_, tb)) in a.iter().zip(b.iter()) {
            assert_eq!(ta, tb);
        }
    }
    #[test]
    fn safe_spawn_is_walkable() {
        let g = generate(123);
        let s = find_safe_spawn(&g, 123);
        assert!(g.get(s).unwrap().is_walkable());
    }
    #[test]
    fn world_size_correct() {
        let g = generate(1);
        assert_eq!(g.width, 80);
        assert_eq!(g.height, 80);
    }
}
```

- [ ] **Step 2: cargo test**

Run: `cargo test -p world gen`
Expected: 3 tests pass。

- [ ] **Step 3: commit**

```bash
git add crates/world/src/gen.rs
git commit -m "feat(world): seed-deterministic map generation (80x80, 5 biomes)"
```

---

## Task 8: World 主结构 + 基础 step

**Files:**
- Modify: `crates/world/src/world.rs`

- [ ] **Step 1: 实现 world.rs（核心 + step + join/move）**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{
    agent::{Agent, AgentId, AgentState},
    action::Action,
    clock::WorldClock,
    coord::{Direction, TileCoord},
    event::TickEvent,
    gen,
    grid::Grid,
    tile::Tile,
};

#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("name '{0}' already taken")]
    NameTaken(String),
    #[error("agent {0} not found")]
    AgentNotFound(String),
    #[error("world full (max {0})")]
    WorldFull(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub seed: u64,
    pub clock: WorldClock,
    pub grid: Grid<Tile>,
    pub agents: HashMap<AgentId, Agent>,
    pub max_agents: usize,
    #[serde(skip)]
    pending_events: Vec<TickEvent>,
}

impl World {
    pub fn bootstrap(seed: u64) -> Self {
        Self {
            seed,
            clock: WorldClock::new(),
            grid: gen::generate(seed),
            agents: HashMap::new(),
            max_agents: 64,
            pending_events: Vec::new(),
        }
    }

    pub fn join(&mut self, name: String) -> Result<AgentId, WorldError> {
        if self.agents.values().any(|a| a.name == name) {
            return Err(WorldError::NameTaken(name));
        }
        if self.agents.len() >= self.max_agents {
            return Err(WorldError::WorldFull(self.max_agents));
        }
        let id = AgentId::new(format!("ag_{:08x}", rand_for_id(self.seed, self.clock.tick, self.agents.len() as u64)));
        let pos = gen::find_safe_spawn(&self.grid, self.seed.wrapping_add(self.agents.len() as u64));
        let agent = Agent::new_at(id.clone(), name.clone(), pos, self.clock.tick);
        self.agents.insert(id.clone(), agent);
        self.pending_events.push(TickEvent::AgentJoined { agent: id.clone(), name, at: pos });
        Ok(id)
    }

    pub fn leave(&mut self, id: &AgentId) -> Result<(), WorldError> {
        let a = self.agents.remove(id).ok_or_else(|| WorldError::AgentNotFound(id.0.clone()))?;
        self.pending_events.push(TickEvent::AgentLeft { agent: id.clone(), name: a.name });
        Ok(())
    }

    /// 推进一个 tick。actions 是本 tick 收到的 agent 动作。
    pub fn step(&mut self, actions: Vec<(AgentId, Action)>) -> Vec<TickEvent> {
        // 1. 排序：按 agent_id 字典序（决定论，speed 等后续 milestone 加权）
        let mut acts = actions;
        acts.sort_by(|a, b| a.0.0.cmp(&b.0.0));

        // 2. 结算
        for (aid, action) in acts {
            self.resolve(&aid, action);
        }

        // 3. 自然系统（M1 仅时钟）
        let was_day = self.clock.tick_in_day() < 30;
        let was_season = self.clock.season();
        self.clock.advance();
        let is_day = self.clock.tick_in_day() < 30;
        if was_day != is_day {
            let day = self.clock.tick as u32 / crate::clock::TICKS_PER_DAY;
            if is_day {
                self.pending_events.push(TickEvent::DayStarted { day });
            } else if self.clock.is_night() {
                self.pending_events.push(TickEvent::NightStarted { day });
            }
        }
        if self.clock.season() != was_season {
            self.pending_events.push(TickEvent::SeasonChanged { to: self.clock.season() });
        }

        std::mem::take(&mut self.pending_events)
    }

    fn resolve(&mut self, aid: &AgentId, action: Action) {
        let Some(agent) = self.agents.get(aid) else { return; };
        if !matches!(agent.state, AgentState::Alive) { return; }
        match action {
            Action::Move { dir } => self.resolve_move(aid.clone(), dir),
            Action::Wait => {},
            Action::Observe => {},
        }
    }

    fn resolve_move(&mut self, aid: AgentId, dir: Direction) {
        let Some(agent) = self.agents.get(&aid) else { return; };
        let from = agent.pos;
        let to = from.step(dir);

        let walkable = self.grid.get(to).map(|t| t.is_walkable()).unwrap_or(false);
        let occupied = self.agents.values().any(|a| a.pos == to && a.id != aid);

        if !walkable {
            self.pending_events.push(TickEvent::AgentMoveFailed { agent: aid, reason: "blocked".into() });
            return;
        }
        if occupied {
            self.pending_events.push(TickEvent::AgentMoveFailed { agent: aid, reason: "occupied".into() });
            return;
        }
        let a = self.agents.get_mut(&aid).unwrap();
        a.pos = to;
        a.last_action_tick = self.clock.tick;
        self.pending_events.push(TickEvent::AgentMoved { agent: aid, from, to });
    }

    pub fn agent_count(&self) -> usize { self.agents.len() }
}

fn rand_for_id(seed: u64, tick: u64, n: u64) -> u32 {
    use rand::{Rng, SeedableRng};
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(
        seed.wrapping_mul(0x100000001b3).wrapping_add(tick).wrapping_add(n));
    r.gen()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn join_assigns_unique_id_and_walkable_pos() {
        let mut w = World::bootstrap(7);
        let id1 = w.join("alice".into()).unwrap();
        let id2 = w.join("bob".into()).unwrap();
        assert_ne!(id1, id2);
        let pos1 = w.agents[&id1].pos;
        assert!(w.grid.get(pos1).unwrap().is_walkable());
    }
    #[test]
    fn duplicate_name_rejected() {
        let mut w = World::bootstrap(7);
        w.join("alice".into()).unwrap();
        let err = w.join("alice".into()).unwrap_err();
        assert!(matches!(err, WorldError::NameTaken(_)));
    }
    #[test]
    fn move_event_emitted_on_walkable() {
        let mut w = World::bootstrap(7);
        let id = w.join("alice".into()).unwrap();
        // 找一个方向能走的
        let orig = w.agents[&id].pos;
        for dir in Direction::ALL {
            let target = orig.step(dir);
            if w.grid.get(target).map(|t| t.is_walkable()).unwrap_or(false) {
                let events = w.step(vec![(id.clone(), Action::Move { dir })]);
                assert!(events.iter().any(|e| matches!(e, TickEvent::AgentMoved { .. })));
                return;
            }
        }
        panic!("no walkable direction (seed-dependent flake; retry with different seed)");
    }
    #[test]
    fn step_advances_clock() {
        let mut w = World::bootstrap(7);
        let before = w.clock.tick;
        w.step(vec![]);
        assert_eq!(w.clock.tick, before + 1);
    }
    #[test]
    fn determinism_of_step() {
        let mut a = World::bootstrap(42);
        let mut b = World::bootstrap(42);
        a.step(vec![]);
        b.step(vec![]);
        assert_eq!(a.clock, b.clock);
        // grid 已确定，agents 都为空，对比序列化字节
        assert_eq!(bincode::serialize(&a).unwrap(), bincode::serialize(&b).unwrap());
    }
}
```

- [ ] **Step 2: cargo test**

Run: `cargo test -p world world`
Expected: 5 tests pass。

- [ ] **Step 3: commit**

```bash
git add crates/world/src/world.rs
git commit -m "feat(world): World struct with join/leave/move + deterministic step"
```

---

## Task 9: Observation 投影 + fog of war

**Files:**
- Modify: `crates/world/src/observation.rs`
- Modify: `crates/world/src/world.rs`（加 `pub fn observe`）

- [ ] **Step 1: observation.rs**

```rust
use serde::{Deserialize, Serialize};
use crate::{
    agent::{AgentId, AgentState, AgentStatus},
    clock::{DayPhase, Season},
    coord::TileCoord,
    event::TickEvent,
    tile::Tile,
};

pub const VISION_RADIUS: u16 = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub tick: u64,
    pub clock: ClockView,
    #[serde(rename = "self")]
    pub self_: SelfView,
    pub vision: VisionView,
    pub visible_entities: Vec<VisibleEntity>,
    pub recent_events: Vec<TickEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockView {
    pub day: u32,
    pub season: Season,
    pub phase: DayPhase,
    pub tick_in_day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfView {
    pub id: AgentId,
    pub name: String,
    pub pos: TileCoord,
    pub status: AgentStatus,
    pub state: AgentState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionView {
    pub radius: u16,
    pub tiles: Vec<VisibleTile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibleTile {
    pub pos: TileCoord,
    pub tile: Tile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisibleEntity {
    Agent { id: AgentId, name: String, pos: TileCoord, hp: i16 },
}
```

- [ ] **Step 2: 在 world.rs 加 observe**

在 `impl World` 末尾添加：

```rust
    pub fn observe(&self, viewer: &AgentId) -> Option<crate::observation::Observation> {
        use crate::observation::*;
        let agent = self.agents.get(viewer)?;
        let center = agent.pos;
        let r = VISION_RADIUS as i16;

        let mut tiles = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                let c = TileCoord::new(center.x + dx, center.y + dy);
                if c.manhattan(center) > VISION_RADIUS { continue; }
                if let Some(t) = self.grid.get(c) {
                    tiles.push(VisibleTile { pos: c, tile: *t });
                }
            }
        }

        let mut entities = Vec::new();
        for (id, a) in &self.agents {
            if id == viewer { continue; }
            if a.pos.manhattan(center) <= VISION_RADIUS {
                entities.push(VisibleEntity::Agent {
                    id: id.clone(), name: a.name.clone(), pos: a.pos, hp: a.status.hp,
                });
            }
        }

        Some(Observation {
            tick: self.clock.tick,
            clock: ClockView {
                day: self.clock.tick as u32 / crate::clock::TICKS_PER_DAY,
                season: self.clock.season(),
                phase: self.clock.phase(),
                tick_in_day: self.clock.tick_in_day(),
            },
            self_: SelfView {
                id: agent.id.clone(),
                name: agent.name.clone(),
                pos: agent.pos,
                status: agent.status,
                state: agent.state,
            },
            vision: VisionView { radius: VISION_RADIUS, tiles },
            visible_entities: entities,
            recent_events: Vec::new(),  // M2 加 cursor 维护
        })
    }
```

- [ ] **Step 3: 加测试到 world.rs 测试模块末尾**

```rust
    #[test]
    fn observe_returns_tiles_within_radius() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        let obs = w.observe(&id).unwrap();
        let center = w.agents[&id].pos;
        assert!(obs.vision.tiles.len() > 1);
        for t in &obs.vision.tiles {
            assert!(t.pos.manhattan(center) <= crate::observation::VISION_RADIUS);
        }
    }
    #[test]
    fn observe_sees_other_agent_nearby() {
        let mut w = World::bootstrap(42);
        let a = w.join("alice".into()).unwrap();
        // 第二个 agent 强制放到 a 附近
        let pos_a = w.agents[&a].pos;
        let near = TileCoord::new(pos_a.x + 1, pos_a.y);
        if w.grid.get(near).map(|t| t.is_walkable()).unwrap_or(false) {
            let b = w.join("bob".into()).unwrap();
            w.agents.get_mut(&b).unwrap().pos = near;
            let obs = w.observe(&a).unwrap();
            assert!(obs.visible_entities.iter().any(|e| matches!(
                e, crate::observation::VisibleEntity::Agent { name, .. } if name == "bob"
            )));
        }
    }
```

- [ ] **Step 4: cargo test + commit**

Run: `cargo test -p world`
Expected: 全部 pass（含 observe 2 个新测试）。

```bash
git add crates/world/src/observation.rs crates/world/src/world.rs
git commit -m "feat(world): observation projection with vision radius"
```

---

## Task 10: server crate 骨架 + 配置

**Files:**
- Create: `crates/server/Cargo.toml`
- Create: `crates/server/src/main.rs`
- Create: `crates/server/src/config.rs`
- Create: `crates/server/src/state.rs`

- [ ] **Step 1: server/Cargo.toml**

```toml
[package]
name = "server"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
world = { path = "../world" }
anyhow.workspace = true
thiserror.workspace = true
serde.workspace = true
serde_json.workspace = true
bincode.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
chrono.workspace = true
axum = { version = "0.7", features = ["ws", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "macros", "chrono"] }
uuid = { version = "1.8", features = ["v4"] }
rand.workspace = true
```

- [ ] **Step 2: server/src/config.rs**

```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub db_path: PathBuf,
    pub tick_ms: u64,
    pub world_seed: u64,
    pub snapshot_every: u64,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self {
            bind_addr: std::env::var("LINGYUAN_BIND").unwrap_or_else(|_| "127.0.0.1:7777".into()),
            db_path: std::env::var("LINGYUAN_DB").map(PathBuf::from).unwrap_or_else(|_| "data/world.db".into()),
            tick_ms: std::env::var("LINGYUAN_TICK_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(2000),
            world_seed: std::env::var("LINGYUAN_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(42),
            snapshot_every: std::env::var("LINGYUAN_SNAPSHOT_EVERY").ok().and_then(|s| s.parse().ok()).unwrap_or(60),
        }
    }
}
```

- [ ] **Step 3: server/src/state.rs**

```rust
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use world::{Action, AgentId, Observation, World};

#[derive(Clone)]
pub struct AppState {
    pub world: Arc<Mutex<World>>,
    pub actions_tx: mpsc::Sender<ActionEnvelope>,
    pub frames_tx: broadcast::Sender<TickFrame>,
    pub db_tx: mpsc::Sender<crate::db::DbWrite>,
    pub config: crate::config::ServerConfig,
}

#[derive(Debug, Clone)]
pub struct ActionEnvelope {
    pub agent: AgentId,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct TickFrame {
    pub tick: u64,
    pub clock: world::WorldClock,
    pub events: Vec<world::TickEvent>,
    pub spectator_view: SpectatorView,
    pub observations: std::collections::HashMap<AgentId, Observation>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpectatorView {
    pub tick: u64,
    pub clock: world::WorldClock,
    pub agents: Vec<SpectatorAgent>,
    pub events: Vec<world::TickEvent>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpectatorAgent {
    pub id: AgentId,
    pub name: String,
    pub pos: world::TileCoord,
    pub hp: i16,
}
```

- [ ] **Step 4: server/src/main.rs（占位入口）**

```rust
mod config;
mod state;
mod db;
mod persistence;
mod auth;
mod tick_loop;
mod routes;

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::info;
use world::World;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,server=debug".into()))
        .init();

    let cfg = config::ServerConfig::from_env();
    std::fs::create_dir_all(cfg.db_path.parent().unwrap_or_else(|| std::path::Path::new(".")))?;

    let db = db::Db::open(&cfg.db_path).await?;
    db.migrate().await?;

    let world = db.load_or_bootstrap(cfg.world_seed).await?;
    info!(tick = world.clock.tick, agents = world.agent_count(), "world loaded");
    let world = Arc::new(Mutex::new(world));

    let (actions_tx, actions_rx) = mpsc::channel(1024);
    let (frames_tx, _) = broadcast::channel(64);
    let (db_tx, db_rx) = mpsc::channel(256);

    let state = state::AppState {
        world: world.clone(),
        actions_tx,
        frames_tx: frames_tx.clone(),
        db_tx,
        config: cfg.clone(),
    };

    // db writer
    tokio::spawn(db::writer_task(db.clone(), db_rx));

    // tick loop
    tokio::spawn(tick_loop::run(state.clone(), actions_rx));

    // http
    let app = routes::router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    info!(addr = %cfg.bind_addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 5: 创建占位 stub 文件让编译先通过**

为 `db.rs / persistence.rs / auth.rs / tick_loop.rs / routes/mod.rs` 各写最小 stub，让 main.rs 引用的符号有定义：

`crates/server/src/db.rs`：
```rust
use std::path::Path;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use world::World;

#[derive(Clone)]
pub struct Db { pub pool: SqlitePool }

#[derive(Debug)]
pub enum DbWrite { Frame(crate::state::TickFrame), Snapshot(World) }

impl Db {
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = SqlitePool::connect(&url).await?;
        Ok(Self { pool })
    }
    pub async fn migrate(&self) -> anyhow::Result<()> { Ok(()) }
    pub async fn load_or_bootstrap(&self, seed: u64) -> anyhow::Result<World> {
        Ok(World::bootstrap(seed))
    }
}

pub async fn writer_task(_db: Db, mut rx: mpsc::Receiver<DbWrite>) {
    while let Some(_w) = rx.recv().await {
        // stub - filled in Task 12
    }
}
```

`crates/server/src/persistence.rs`：
```rust
// stub - filled in Task 12
```

`crates/server/src/auth.rs`：
```rust
// stub - filled in Task 13
```

`crates/server/src/tick_loop.rs`：
```rust
use tokio::sync::mpsc;
use crate::state::{ActionEnvelope, AppState};

pub async fn run(_state: AppState, mut _rx: mpsc::Receiver<ActionEnvelope>) {
    // filled in Task 11
}
```

`crates/server/src/routes/mod.rs`：
```rust
use axum::Router;
use crate::state::AppState;

pub fn router(_state: AppState) -> Router {
    Router::new().route("/health", axum::routing::get(|| async { "ok" }))
}
```

- [ ] **Step 6: cargo build + commit**

Run: `cargo build -p server`
Expected: 编译通过，可能 warning（unused 等）。

```bash
git add crates/server
git commit -m "feat(server): crate skeleton (config, state, stubs)"
```

---

## Task 11: tick_loop 实装

**Files:**
- Modify: `crates/server/src/tick_loop.rs`

- [ ] **Step 1: 实装 tick_loop**

```rust
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use crate::{
    db::DbWrite,
    state::{ActionEnvelope, AppState, SpectatorAgent, SpectatorView, TickFrame},
};

pub async fn run(state: AppState, mut rx: mpsc::Receiver<ActionEnvelope>) {
    let mut ticker = tokio::time::interval(Duration::from_millis(state.config.tick_ms));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;

        // 排干所有 pending action（非阻塞）
        let mut actions = Vec::new();
        while let Ok(env) = rx.try_recv() {
            actions.push((env.agent, env.action));
        }

        let mut w = state.world.lock().await;
        let events = w.step(actions);

        // 投影
        let observations = w.agents.keys().cloned().filter_map(|id| {
            w.observe(&id).map(|obs| (id, obs))
        }).collect();

        let spectator = SpectatorView {
            tick: w.clock.tick,
            clock: w.clock,
            agents: w.agents.values().map(|a| SpectatorAgent {
                id: a.id.clone(),
                name: a.name.clone(),
                pos: a.pos,
                hp: a.status.hp,
            }).collect(),
            events: events.clone(),
        };

        let frame = TickFrame {
            tick: w.clock.tick,
            clock: w.clock,
            events,
            spectator_view: spectator,
            observations,
        };

        // 持久化（不阻塞）
        if state.db_tx.try_send(DbWrite::Frame(frame.clone())).is_err() {
            warn!("db writer queue full, dropping frame {}", frame.tick);
        }
        if w.clock.tick > 0 && w.clock.tick % state.config.snapshot_every == 0 {
            let snap = w.clone();
            let _ = state.db_tx.try_send(DbWrite::Snapshot(snap));
            debug!(tick = w.clock.tick, "queued snapshot");
        }

        // 广播
        let _ = state.frames_tx.send(frame);

        if w.clock.tick % 30 == 0 {
            info!(tick = w.clock.tick, agents = w.agent_count(), "tick");
        }
    }
}
```

- [ ] **Step 2: cargo build**

Run: `cargo build -p server`
Expected: 编译通过。

- [ ] **Step 3: commit**

```bash
git add crates/server/src/tick_loop.rs
git commit -m "feat(server): tick_loop with action drain + broadcast + db queue"
```

---

## Task 12: SQLite 持久化 + 崩溃恢复

**Files:**
- Create: `crates/server/migrations/0001_initial.sql`
- Modify: `crates/server/src/db.rs`
- Modify: `crates/server/src/persistence.rs`

- [ ] **Step 1: 写 migration**

`crates/server/migrations/0001_initial.sql`:
```sql
CREATE TABLE IF NOT EXISTS snapshots (
  tick       INTEGER PRIMARY KEY,
  bin        BLOB NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
  tick       INTEGER NOT NULL,
  seq        INTEGER NOT NULL,
  event_json TEXT NOT NULL,
  PRIMARY KEY (tick, seq)
);

CREATE TABLE IF NOT EXISTS agents_meta (
  agent_id    TEXT PRIMARY KEY,
  name        TEXT UNIQUE NOT NULL,
  token_hash  TEXT NOT NULL,
  joined_at   INTEGER NOT NULL,
  total_lives INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_events_tick ON events(tick);
```

- [ ] **Step 2: 实装 persistence.rs**

```rust
use serde::{Deserialize, Serialize};
use world::{TickEvent, World};

pub fn serialize_world(w: &World) -> anyhow::Result<Vec<u8>> {
    Ok(bincode::serialize(w)?)
}

pub fn deserialize_world(bytes: &[u8]) -> anyhow::Result<World> {
    Ok(bincode::deserialize(bytes)?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub tick: i64,
    pub seq: i64,
    pub event: TickEvent,
}
```

- [ ] **Step 3: 实装 db.rs（替换 stub）**

```rust
use std::path::Path;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tracing::{error, info};
use world::World;
use crate::persistence::{deserialize_world, serialize_world};

#[derive(Clone)]
pub struct Db { pub pool: SqlitePool }

#[derive(Debug)]
pub enum DbWrite {
    Frame(crate::state::TickFrame),
    Snapshot(World),
    UpsertAgentMeta { agent_id: String, name: String, token_hash: String, joined_at: i64 },
}

const MIGRATION_SQL: &str = include_str!("../migrations/0001_initial.sql");

impl Db {
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = SqlitePool::connect(&url).await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        for stmt in MIGRATION_SQL.split(';') {
            let s = stmt.trim();
            if s.is_empty() { continue; }
            sqlx::query(s).execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn load_or_bootstrap(&self, seed: u64) -> anyhow::Result<World> {
        let row: Option<(i64, Vec<u8>)> = sqlx::query_as("SELECT tick, bin FROM snapshots ORDER BY tick DESC LIMIT 1")
            .fetch_optional(&self.pool).await?;
        let Some((snap_tick, bin)) = row else {
            info!(seed, "no snapshot, bootstrapping world");
            return Ok(World::bootstrap(seed));
        };
        let mut world = deserialize_world(&bin)?;
        info!(snap_tick, "loaded snapshot");

        // replay events with tick > snap_tick
        let evt_rows: Vec<(i64, i64, String)> = sqlx::query_as(
            "SELECT tick, seq, event_json FROM events WHERE tick > ? ORDER BY tick ASC, seq ASC"
        ).bind(snap_tick).fetch_all(&self.pool).await?;
        // M1: 我们只重放时钟相关效果（事件已经记录，但 world step 是真值源）；
        // 为简化与正确，恢复策略：从 snapshot 开始 N tick = events 里最大 tick - snap_tick，
        // 用空动作 step 推进到那个 tick，因为本期 step 是确定性的、动作日志暂不重放。
        if let Some((max_tick, _, _)) = evt_rows.last() {
            let needed = (*max_tick as u64).saturating_sub(world.clock.tick);
            for _ in 0..needed { world.step(vec![]); }
            info!(target_tick = max_tick, "replayed clock to match event log tail");
        }
        Ok(world)
    }
}

pub async fn writer_task(db: Db, mut rx: mpsc::Receiver<DbWrite>) {
    while let Some(w) = rx.recv().await {
        if let Err(e) = handle(&db, w).await {
            error!(error = %e, "db write failed");
        }
    }
}

async fn handle(db: &Db, w: DbWrite) -> anyhow::Result<()> {
    match w {
        DbWrite::Frame(frame) => {
            let mut tx = db.pool.begin().await?;
            for (seq, evt) in frame.events.iter().enumerate() {
                let json = serde_json::to_string(evt)?;
                sqlx::query("INSERT INTO events(tick, seq, event_json) VALUES(?, ?, ?)")
                    .bind(frame.tick as i64)
                    .bind(seq as i64)
                    .bind(json)
                    .execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        DbWrite::Snapshot(world) => {
            let bin = serialize_world(&world)?;
            let now = chrono::Utc::now().timestamp();
            sqlx::query("INSERT OR REPLACE INTO snapshots(tick, bin, created_at) VALUES(?, ?, ?)")
                .bind(world.clock.tick as i64)
                .bind(bin)
                .bind(now)
                .execute(&db.pool).await?;
        }
        DbWrite::UpsertAgentMeta { agent_id, name, token_hash, joined_at } => {
            sqlx::query(
                "INSERT INTO agents_meta(agent_id, name, token_hash, joined_at, total_lives) VALUES(?, ?, ?, ?, 0)
                 ON CONFLICT(agent_id) DO UPDATE SET name=excluded.name, token_hash=excluded.token_hash"
            )
            .bind(agent_id).bind(name).bind(token_hash).bind(joined_at)
            .execute(&db.pool).await?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: cargo build**

Run: `cargo build -p server`
Expected: 编译通过。

- [ ] **Step 5: commit**

```bash
git add crates/server/migrations crates/server/src/db.rs crates/server/src/persistence.rs
git commit -m "feat(server): SQLite persistence + crash recovery (snapshot+events)"
```

---

## Task 13: auth + token store

**Files:**
- Modify: `crates/server/src/auth.rs`

- [ ] **Step 1: 实装 auth.rs**

```rust
use axum::{extract::FromRequestParts, http::{request::Parts, StatusCode}};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use world::AgentId;

pub fn new_token() -> String {
    Uuid::new_v4().to_string().replace('-', "")
}

pub fn hash_token(tok: &str) -> String {
    let mut h = Sha256::new();
    h.update(tok.as_bytes());
    hex_lower(&h.finalize())
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes { s.push_str(&format!("{:02x}", b)); }
    s
}

#[derive(Debug, Clone)]
pub struct AuthAgent { pub agent_id: AgentId }

#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthAgent {
    type Rejection = (StatusCode, &'static str);
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header = parts.headers.get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token"))?;
        let agent_id = parts.headers.get("x-agent-id")
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "missing x-agent-id"))?;
        // 校验 hash 留给 routes 层（需 DB）；此处先组装结构，routes 自行查 DB 比对
        let _ = header;
        Ok(AuthAgent { agent_id: AgentId::new(agent_id) })
    }
}
```

- [ ] **Step 2: 加 sha2 + hex 依赖**

修改 `crates/server/Cargo.toml`，在 `[dependencies]` 里加：

```toml
sha2 = "0.10"
```

- [ ] **Step 3: cargo build + commit**

Run: `cargo build -p server`
Expected: 编译通过。

```bash
git add crates/server
git commit -m "feat(server): bearer token auth extractor"
```

---

## Task 14: REST routes — health, clock, join, observe, act, leave

**Files:**
- Create: `crates/server/src/routes/health.rs`
- Create: `crates/server/src/routes/clock.rs`
- Create: `crates/server/src/routes/join.rs`
- Create: `crates/server/src/routes/observe.rs`
- Create: `crates/server/src/routes/act.rs`
- Create: `crates/server/src/routes/leave.rs`
- Modify: `crates/server/src/routes/mod.rs`

- [ ] **Step 1: health.rs**

```rust
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct Health { pub ok: bool, pub version: &'static str }

pub async fn health() -> Json<Health> {
    Json(Health { ok: true, version: env!("CARGO_PKG_VERSION") })
}
```

- [ ] **Step 2: clock.rs**

```rust
use axum::{extract::State, Json};
use serde::Serialize;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ClockResp {
    pub tick: u64,
    pub day: u32,
    pub season: world::Season,
    pub phase: world::DayPhase,
    pub tick_in_day: u32,
}

pub async fn clock(State(s): State<AppState>) -> Json<ClockResp> {
    let w = s.world.lock().await;
    Json(ClockResp {
        tick: w.clock.tick,
        day: w.clock.tick as u32 / world::clock::TICKS_PER_DAY,
        season: w.clock.season(),
        phase: w.clock.phase(),
        tick_in_day: w.clock.tick_in_day(),
    })
}
```

- [ ] **Step 3: join.rs**

```rust
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use crate::{auth::{hash_token, new_token}, db::DbWrite, state::AppState};

#[derive(Deserialize)]
pub struct JoinReq { pub name: String }

#[derive(Serialize)]
pub struct JoinResp {
    pub agent_id: String,
    pub token: String,
    pub spawn_at: world::TileCoord,
    pub tick: u64,
}

pub async fn join(State(s): State<AppState>, Json(req): Json<JoinReq>) -> Result<Json<JoinResp>, (StatusCode, String)> {
    if req.name.is_empty() || req.name.len() > 32 {
        return Err((StatusCode::BAD_REQUEST, "name must be 1..=32 chars".into()));
    }
    let mut w = s.world.lock().await;
    let id = w.join(req.name.clone()).map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;
    let pos = w.agents[&id].pos;
    let tick = w.clock.tick;
    drop(w);

    let token = new_token();
    let _ = s.db_tx.send(DbWrite::UpsertAgentMeta {
        agent_id: id.0.clone(),
        name: req.name,
        token_hash: hash_token(&token),
        joined_at: chrono::Utc::now().timestamp(),
    }).await;

    Ok(Json(JoinResp { agent_id: id.0, token, spawn_at: pos, tick }))
}
```

- [ ] **Step 4: observe.rs**

```rust
use axum::{extract::State, http::StatusCode, Json};
use crate::{auth::AuthAgent, state::AppState};

pub async fn observe(State(s): State<AppState>, AuthAgent { agent_id }: AuthAgent) -> Result<Json<world::Observation>, (StatusCode, &'static str)> {
    let w = s.world.lock().await;
    w.observe(&agent_id).map(Json).ok_or((StatusCode::NOT_FOUND, "agent not found"))
}
```

- [ ] **Step 5: act.rs**

```rust
use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use crate::{auth::AuthAgent, state::{ActionEnvelope, AppState}};

#[derive(Serialize)]
pub struct ActResp { pub accepted: bool, pub queued_for_tick: u64 }

pub async fn act(
    State(s): State<AppState>,
    AuthAgent { agent_id }: AuthAgent,
    Json(action): Json<world::Action>,
) -> Result<Json<ActResp>, (StatusCode, String)> {
    let tick = { s.world.lock().await.clock.tick + 1 };
    s.actions_tx.send(ActionEnvelope { agent: agent_id, action }).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(ActResp { accepted: true, queued_for_tick: tick }))
}
```

- [ ] **Step 6: leave.rs**

```rust
use axum::{extract::State, http::StatusCode};
use crate::{auth::AuthAgent, state::AppState};

pub async fn leave(State(s): State<AppState>, AuthAgent { agent_id }: AuthAgent) -> Result<(), (StatusCode, String)> {
    let mut w = s.world.lock().await;
    w.leave(&agent_id).map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(())
}
```

- [ ] **Step 7: routes/mod.rs（替换 stub）**

```rust
use axum::{routing::{get, post}, Router};
use tower_http::cors::CorsLayer;
use crate::state::AppState;

mod health;
mod clock;
mod join;
mod observe;
mod act;
mod leave;
pub mod ws;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/v1/world/clock", get(clock::clock))
        .route("/api/v1/join", post(join::join))
        .route("/api/v1/observe", get(observe::observe))
        .route("/api/v1/act", post(act::act))
        .route("/api/v1/leave", post(leave::leave))
        .route("/ws/spectator", get(ws::spectator_ws))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
```

- [ ] **Step 8: ws.rs 占位（下个 task 实装）**

`crates/server/src/routes/ws.rs`：
```rust
use axum::{extract::{State, WebSocketUpgrade}, response::Response};
use crate::state::AppState;

pub async fn spectator_ws(_ws: WebSocketUpgrade, State(_s): State<AppState>) -> Response {
    axum::http::Response::builder()
        .status(503)
        .body(axum::body::Body::from("ws not yet wired"))
        .unwrap()
}
```

- [ ] **Step 9: cargo build + commit**

Run: `cargo build -p server`
Expected: 编译通过。

```bash
git add crates/server
git commit -m "feat(server): REST routes (health/clock/join/observe/act/leave)"
```

---

## Task 15: spectator WebSocket

**Files:**
- Modify: `crates/server/src/routes/ws.rs`

- [ ] **Step 1: 实装 ws.rs**

```rust
use axum::{
    extract::{ws::{Message, WebSocket}, State, WebSocketUpgrade},
    response::Response,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::Serialize;
use tracing::{debug, info};
use crate::state::AppState;

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SpectatorMsg<'a> {
    Snapshot {
        tick: u64,
        clock: &'a world::WorldClock,
        grid_width: u16,
        grid_height: u16,
        tiles: Vec<TileMsg>,
        agents: Vec<&'a crate::state::SpectatorAgent>,
    },
    Tick {
        view: &'a crate::state::SpectatorView,
    },
}

#[derive(Serialize)]
struct TileMsg {
    pos: world::TileCoord,
    kind: world::TileKind,
    biome: world::Biome,
}

pub async fn spectator_ws(ws: WebSocketUpgrade, State(s): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle(socket, s))
}

async fn handle(socket: WebSocket, s: AppState) {
    let (mut tx, mut rx) = socket.split();
    info!("spectator connected");

    // 1. 立即发 snapshot
    {
        let w = s.world.lock().await;
        let tiles: Vec<TileMsg> = w.grid.iter().map(|(pos, t)| TileMsg {
            pos, kind: t.kind, biome: t.biome,
        }).collect();
        let agents: Vec<crate::state::SpectatorAgent> = w.agents.values().map(|a| crate::state::SpectatorAgent {
            id: a.id.clone(),
            name: a.name.clone(),
            pos: a.pos,
            hp: a.status.hp,
        }).collect();
        let msg = SpectatorMsg::Snapshot {
            tick: w.clock.tick,
            clock: &w.clock,
            grid_width: w.grid.width,
            grid_height: w.grid.height,
            tiles,
            agents: agents.iter().collect(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        if tx.send(Message::Text(json)).await.is_err() {
            return;
        }
    }

    // 2. 订阅 frames + 转发
    let mut frames = s.frames_tx.subscribe();
    let send_loop = async {
        while let Ok(f) = frames.recv().await {
            let msg = SpectatorMsg::Tick { view: &f.spectator_view };
            let json = serde_json::to_string(&msg).unwrap();
            if tx.send(Message::Text(json)).await.is_err() { break; }
        }
    };
    let recv_loop = async {
        while let Some(Ok(m)) = rx.next().await {
            debug!(?m, "spectator msg");
            if matches!(m, Message::Close(_)) { break; }
        }
    };
    tokio::select! { _ = send_loop => {}, _ = recv_loop => {} }
    info!("spectator disconnected");
}
```

- [ ] **Step 2: 加 futures-util 依赖**

修改 `crates/server/Cargo.toml` 加：

```toml
futures-util = "0.3"
```

- [ ] **Step 3: cargo build + 端到端冒烟**

Run: `cargo run -p server &`  
（注意：让它在后台跑）

然后：
Run: `curl -s http://localhost:7777/health` → 应输出 `{"ok":true,"version":"0.1.0"}`  
Run: `curl -s http://localhost:7777/api/v1/world/clock` → 应输出 `{"tick":<N>,...}`，N 在变。

跑完 kill 后台 server：`pkill -f "target/debug/server"`。

- [ ] **Step 4: commit**

```bash
git add crates/server
git commit -m "feat(server): spectator WebSocket (snapshot + tick frames)"
```

---

## Task 16: server 集成测试

**Files:**
- Create: `crates/server/tests/integration.rs`

- [ ] **Step 1: 写集成测试**

```rust
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

#[tokio::test]
async fn server_starts_and_advances_clock() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    let mut child = Command::new(env!("CARGO_BIN_EXE_server"))
        .env("LINGYUAN_BIND", "127.0.0.1:17777")
        .env("LINGYUAN_DB", db_path.to_str().unwrap())
        .env("LINGYUAN_TICK_MS", "100") // fast tick for test
        .spawn().unwrap();

    sleep(Duration::from_millis(500)).await;

    let r1: serde_json::Value = reqwest::get("http://127.0.0.1:17777/api/v1/world/clock")
        .await.unwrap().json().await.unwrap();
    sleep(Duration::from_millis(500)).await;
    let r2: serde_json::Value = reqwest::get("http://127.0.0.1:17777/api/v1/world/clock")
        .await.unwrap().json().await.unwrap();

    let t1 = r1["tick"].as_u64().unwrap();
    let t2 = r2["tick"].as_u64().unwrap();
    assert!(t2 > t1, "clock should advance ({} -> {})", t1, t2);

    child.kill().await.ok();
}

#[tokio::test]
async fn agent_can_join_and_observe() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    let mut child = Command::new(env!("CARGO_BIN_EXE_server"))
        .env("LINGYUAN_BIND", "127.0.0.1:17778")
        .env("LINGYUAN_DB", db_path.to_str().unwrap())
        .env("LINGYUAN_TICK_MS", "100")
        .spawn().unwrap();
    sleep(Duration::from_millis(500)).await;

    let join_resp: serde_json::Value = reqwest::Client::new()
        .post("http://127.0.0.1:17778/api/v1/join")
        .json(&serde_json::json!({ "name": "alice" }))
        .send().await.unwrap().json().await.unwrap();
    let agent_id = join_resp["agent_id"].as_str().unwrap();
    let token = join_resp["token"].as_str().unwrap();

    let obs: serde_json::Value = reqwest::Client::new()
        .get("http://127.0.0.1:17778/api/v1/observe")
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Agent-Id", agent_id)
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(obs["self"]["name"], "alice");
    assert!(obs["vision"]["tiles"].as_array().unwrap().len() > 1);

    child.kill().await.ok();
}
```

- [ ] **Step 2: 加 dev-dependencies**

修改 `crates/server/Cargo.toml`，加：

```toml
[dev-dependencies]
reqwest = { version = "0.12", features = ["json"] }
tempfile = "3"
tokio = { version = "1.36", features = ["full", "process"] }
serde_json.workspace = true
```

- [ ] **Step 3: cargo test**

Run: `cargo test -p server --test integration -- --test-threads=1`
Expected: 2 tests pass。

- [ ] **Step 4: commit**

```bash
git add crates/server
git commit -m "test(server): integration smoke (clock advances, join + observe)"
```

---

## Task 17: `survivor` CLI 骨架 + token store

**Files:**
- Create: `crates/cli/Cargo.toml`
- Create: `crates/cli/src/main.rs`
- Create: `crates/cli/src/commands.rs`
- Create: `crates/cli/src/client.rs`
- Create: `crates/cli/src/token_store.rs`
- Create: `crates/cli/src/render.rs`

- [ ] **Step 1: cli/Cargo.toml**

```toml
[package]
name = "cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "survivor"
path = "src/main.rs"

[dependencies]
world = { path = "../world" }
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json"] }
dirs = "5"
```

- [ ] **Step 2: token_store.rs**

```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenFile {
    pub agent_id: String,
    pub token: String,
    pub server: String,
    pub name: String,
}

pub fn store_path() -> PathBuf {
    let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push(".lingyuan");
    p.push("token.json");
    p
}

pub fn load() -> anyhow::Result<TokenFile> {
    let bytes = std::fs::read(store_path())?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn save(t: &TokenFile) -> anyhow::Result<()> {
    let p = store_path();
    if let Some(parent) = p.parent() { std::fs::create_dir_all(parent)?; }
    std::fs::write(&p, serde_json::to_vec_pretty(t)?)?;
    Ok(())
}

pub fn clear() -> anyhow::Result<()> {
    let p = store_path();
    if p.exists() { std::fs::remove_file(p)?; }
    Ok(())
}
```

- [ ] **Step 3: client.rs**

```rust
use anyhow::anyhow;
use serde::{de::DeserializeOwned, Serialize};
use crate::token_store::TokenFile;

pub struct Client { http: reqwest::Client, base: String, token: TokenFile }

impl Client {
    pub fn from_token(t: TokenFile) -> Self {
        Self { http: reqwest::Client::new(), base: t.server.clone(), token: t }
    }

    pub async fn observe<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let r = self.http.get(format!("{}/api/v1/observe", self.base))
            .header("Authorization", format!("Bearer {}", self.token.token))
            .header("X-Agent-Id", &self.token.agent_id)
            .send().await?;
        if !r.status().is_success() {
            let s = r.status();
            let t = r.text().await.unwrap_or_default();
            return Err(anyhow!("{}: {}", s, t));
        }
        Ok(r.json().await?)
    }

    pub async fn act<A: Serialize, R: DeserializeOwned>(&self, action: &A) -> anyhow::Result<R> {
        let r = self.http.post(format!("{}/api/v1/act", self.base))
            .header("Authorization", format!("Bearer {}", self.token.token))
            .header("X-Agent-Id", &self.token.agent_id)
            .json(action).send().await?;
        if !r.status().is_success() {
            return Err(anyhow!("{}: {}", r.status(), r.text().await.unwrap_or_default()));
        }
        Ok(r.json().await?)
    }

    pub async fn leave(&self) -> anyhow::Result<()> {
        self.http.post(format!("{}/api/v1/leave", self.base))
            .header("Authorization", format!("Bearer {}", self.token.token))
            .header("X-Agent-Id", &self.token.agent_id)
            .send().await?;
        Ok(())
    }
}

pub async fn join_remote(server: &str, name: &str) -> anyhow::Result<TokenFile> {
    let r = reqwest::Client::new()
        .post(format!("{}/api/v1/join", server))
        .json(&serde_json::json!({ "name": name }))
        .send().await?;
    if !r.status().is_success() {
        return Err(anyhow!("{}: {}", r.status(), r.text().await.unwrap_or_default()));
    }
    let v: serde_json::Value = r.json().await?;
    Ok(TokenFile {
        agent_id: v["agent_id"].as_str().unwrap().to_string(),
        token: v["token"].as_str().unwrap().to_string(),
        server: server.to_string(),
        name: name.to_string(),
    })
}
```

- [ ] **Step 4: render.rs**

```rust
pub fn render_markdown(obs: &serde_json::Value) -> String {
    let name = obs["self"]["name"].as_str().unwrap_or("?");
    let tick = obs["tick"].as_u64().unwrap_or(0);
    let clock = &obs["clock"];
    let status = &obs["self"]["status"];
    let mut s = String::new();
    s.push_str(&format!(
        "## You are {} — tick {}, {} 季 {} 日 {} 时\n\n",
        name, tick,
        clock["season"].as_str().unwrap_or("?"),
        clock["day"].as_u64().unwrap_or(0),
        clock["tick_in_day"].as_u64().unwrap_or(0),
    ));
    s.push_str(&format!(
        "**Status:** HP {}/100 · 饥 {}/100 · 力 {}/100 · 温 {} · 灵识 {}\n\n",
        status["hp"], status["hunger"], status["stamina"], status["warmth"], status["sanity"]
    ));
    s.push_str("**You see:**\n");
    let pos = &obs["self"]["pos"];
    s.push_str(&format!("- ({},{}) you\n", pos["x"], pos["y"]));
    if let Some(arr) = obs["visible_entities"].as_array() {
        for e in arr {
            if e["kind"] == "agent" {
                s.push_str(&format!(
                    "- ({},{}) **{}** [agent, HP {}]\n",
                    e["pos"]["x"], e["pos"]["y"], e["name"], e["hp"]
                ));
            }
        }
    }
    if let Some(tiles) = obs["vision"]["tiles"].as_array() {
        s.push_str(&format!("\n*({} tiles visible)*\n", tiles.len()));
    }
    s
}
```

- [ ] **Step 5: commands.rs**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "survivor", version, about = "灵渊 agent CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// 注册并保存 token
    Join {
        #[arg(long)] name: String,
        #[arg(long, default_value = "http://localhost:7777")] server: String,
    },
    /// 主动离开
    Leave,
    /// 当前 observation
    Observe {
        #[arg(long, default_value = "markdown", value_parser = ["markdown", "json"])]
        format: String,
    },
    /// 排队下一动作
    Act {
        /// 动作动词：move | wait
        verb: String,
        /// 例：--dir=north
        #[arg(long, value_parser = clap::value_parser!(String))]
        dir: Option<String>,
    },
    /// 删 token 文件
    Clear,
}
```

- [ ] **Step 6: main.rs**

```rust
mod client;
mod commands;
mod render;
mod token_store;

use anyhow::Context;
use clap::Parser;
use commands::{Cli, Cmd};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Join { name, server } => {
            let tok = client::join_remote(&server, &name).await?;
            token_store::save(&tok)?;
            println!("joined as {} (id {}) on {}", tok.name, tok.agent_id, tok.server);
        }
        Cmd::Leave => {
            let t = token_store::load().context("not joined yet")?;
            client::Client::from_token(t).leave().await?;
            token_store::clear()?;
            println!("left");
        }
        Cmd::Observe { format } => {
            let t = token_store::load().context("not joined yet")?;
            let c = client::Client::from_token(t);
            let obs: serde_json::Value = c.observe().await?;
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&obs)?),
                _ => print!("{}", render::render_markdown(&obs)),
            }
        }
        Cmd::Act { verb, dir } => {
            let t = token_store::load().context("not joined yet")?;
            let c = client::Client::from_token(t);
            let action = match verb.as_str() {
                "move" => {
                    let d = dir.context("--dir required for move")?;
                    serde_json::json!({"kind":"move","data":{"dir":d}})
                }
                "wait" => serde_json::json!({"kind":"wait","data":null}),
                v => anyhow::bail!("unknown verb {v}"),
            };
            let r: serde_json::Value = c.act(&action).await?;
            println!("{}", serde_json::to_string_pretty(&r)?);
        }
        Cmd::Clear => {
            token_store::clear()?;
            println!("cleared");
        }
    }
    Ok(())
}
```

- [ ] **Step 7: 修复 Action 的 wait 反序列化**

由于 `Action::Wait` 是 unit variant，serde 默认对 `{"kind":"wait","data":null}` 可能不接受。改 `crates/world/src/action.rs`，对 Wait 加 `#[serde(default)]` 或改用 `data: {}`。最稳：

```rust
// crates/world/src/action.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Action {
    Move { dir: Direction },
    #[default]
    Wait,
    Observe,
}
```

并把 CLI 里的 wait json 改为：`{"kind":"wait"}` 或 `{"kind":"wait","data":null}` —— serde with `tag/content` 在 unit variant 时会接受 `{"kind":"wait"}`。CLI 改：

```rust
"wait" => serde_json::json!({"kind":"wait"}),
```

加单测到 `crates/world/src/action.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deserialize_wait_no_data() {
        let a: Action = serde_json::from_str(r#"{"kind":"wait"}"#).unwrap();
        assert_eq!(a, Action::Wait);
    }
    #[test]
    fn deserialize_move() {
        let a: Action = serde_json::from_str(r#"{"kind":"move","data":{"dir":"north"}}"#).unwrap();
        assert_eq!(a, Action::Move { dir: Direction::North });
    }
}
```

- [ ] **Step 8: cargo build + cargo test**

Run: `cargo build && cargo test -p world action`
Expected: 编译通过，2 个新测试通过。

- [ ] **Step 9: commit**

```bash
git add crates/cli crates/world/src/action.rs
git commit -m "feat(cli): survivor binary (join/observe/act/leave/clear)"
```

---

## Task 18: 端到端冒烟（server + CLI）

**Files:**
- 仅运行测试，不修改代码

- [ ] **Step 1: 启动 server 后台**

```bash
cd /Users/e0_7/projects/games/lingyuan
rm -rf data
LINGYUAN_TICK_MS=500 cargo run -p server > /tmp/lingyuan.log 2>&1 &
SERVER_PID=$!
sleep 2
```

- [ ] **Step 2: 跑 CLI 流程**

```bash
target/debug/survivor clear || true
target/debug/survivor join --name alice --server http://localhost:7777
target/debug/survivor observe --format markdown
target/debug/survivor act move --dir=north
sleep 1
target/debug/survivor observe --format markdown
target/debug/survivor leave
```

Expected: 两次 observe 之间 agent 的 pos 在 N/S 方向变化（如果北方可走）；leave 后再 observe 会 401/404。

- [ ] **Step 3: 检查持久化**

```bash
ls -l data/world.db
sqlite3 data/world.db 'SELECT COUNT(*) FROM events; SELECT COUNT(*) FROM snapshots;'
```

Expected: world.db 存在，events 行数 > 0，snapshots ≥ 1。

- [ ] **Step 4: kill server，verify 重启后恢复**

```bash
kill $SERVER_PID
sleep 1
LINGYUAN_TICK_MS=500 cargo run -p server > /tmp/lingyuan.log 2>&1 &
sleep 2
curl -s http://localhost:7777/api/v1/world/clock | jq .tick
```

Expected: 重启后 tick **不是 0**，而是接续上次 snapshot 之后的值（恢复成功）。

- [ ] **Step 5: cleanup + commit smoke 脚本**

把上面步骤写成 `scripts/smoke.sh` 入仓：

```bash
mkdir -p scripts
```

文件内容（写到 `scripts/smoke.sh`）：

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
rm -rf data
cargo build -p server -p cli
LINGYUAN_TICK_MS=500 ./target/debug/server > /tmp/lingyuan.log 2>&1 &
PID=$!
trap "kill $PID 2>/dev/null || true" EXIT
sleep 2
./target/debug/survivor clear || true
./target/debug/survivor join --name alice --server http://localhost:7777
./target/debug/survivor observe --format markdown
./target/debug/survivor act move --dir=north
sleep 1
./target/debug/survivor observe --format markdown
./target/debug/survivor leave
echo "✅ smoke ok"
```

```bash
chmod +x scripts/smoke.sh
git add scripts/smoke.sh
git commit -m "test: scripts/smoke.sh end-to-end CLI 冒烟"
```

---

## Task 19: 前端 Vite + PixiJS 骨架

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/tsconfig.json`
- Create: `frontend/vite.config.ts`
- Create: `frontend/index.html`
- Create: `frontend/src/main.ts`
- Create: `frontend/src/ws.ts`
- Create: `frontend/src/types.ts`
- Create: `frontend/src/stage/world-stage.ts`
- Create: `frontend/src/stage/tile-layer.ts`
- Create: `frontend/src/stage/agent-layer.ts`
- Create: `frontend/src/hud/style.css`

- [ ] **Step 1: package.json**

```json
{
  "name": "lingyuan-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "pixi.js": "^8.1.0"
  },
  "devDependencies": {
    "typescript": "^5.4.0",
    "vite": "^5.2.0"
  }
}
```

- [ ] **Step 2: tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "noImplicitAny": true,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "useDefineForClassFields": true,
    "lib": ["ES2022", "DOM"]
  },
  "include": ["src"]
}
```

- [ ] **Step 3: vite.config.ts**

```ts
import { defineConfig } from 'vite';
export default defineConfig({
  server: { port: 5173, host: '127.0.0.1' },
});
```

- [ ] **Step 4: index.html**

```html
<!doctype html>
<html lang="zh">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>灵渊 · 观战</title>
    <link rel="stylesheet" href="/src/hud/style.css" />
  </head>
  <body>
    <header id="topbar">灵渊 · <span id="clock">连接中…</span></header>
    <main>
      <div id="stage"></div>
      <aside id="right-panel">
        <section><h3>在世</h3><ul id="agents"></ul></section>
        <section><h3>最近事件</h3><ul id="events"></ul></section>
      </aside>
    </main>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 5: hud/style.css**

```css
:root {
  --jade: #5C8C6A; --gold: #D9A441; --ink: #2A2826;
  --cinnabar: #B83A2E; --moon: #F2EFE4;
}
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: 'PingFang SC', system-ui, sans-serif; background: var(--ink); color: var(--moon); height: 100vh; display: flex; flex-direction: column; }
#topbar { padding: 12px 20px; border-bottom: 1px solid var(--gold); font-size: 18px; letter-spacing: 1px; }
main { flex: 1; display: grid; grid-template-columns: 1fr 280px; min-height: 0; }
#stage { background: #1a1816; overflow: hidden; }
#stage canvas { display: block; }
#right-panel { border-left: 1px solid var(--gold); padding: 16px; background: rgba(242,239,228,0.04); overflow-y: auto; }
#right-panel h3 { color: var(--gold); font-size: 13px; letter-spacing: 2px; margin-bottom: 8px; }
#right-panel section { margin-bottom: 24px; }
#right-panel ul { list-style: none; }
#right-panel li { padding: 4px 0; font-size: 13px; border-bottom: 1px dotted rgba(242,239,228,0.1); }
```

- [ ] **Step 6: types.ts**

```ts
export interface TileMsg { pos: { x: number; y: number }; kind: string; biome: string; }
export interface AgentMsg { id: string; name: string; pos: { x: number; y: number }; hp: number; }
export interface ClockMsg { tick: number; }

export interface SnapshotMsg {
  kind: 'snapshot';
  tick: number;
  clock: ClockMsg;
  grid_width: number;
  grid_height: number;
  tiles: TileMsg[];
  agents: AgentMsg[];
}

export interface TickMsg {
  kind: 'tick';
  view: {
    tick: number;
    clock: { tick: number };
    agents: AgentMsg[];
    events: Array<{ kind: string; data: unknown }>;
  };
}

export type ServerMsg = SnapshotMsg | TickMsg;
```

- [ ] **Step 7: ws.ts**

```ts
import type { ServerMsg } from './types';

export function connect(url: string, onMsg: (m: ServerMsg) => void): WebSocket {
  const ws = new WebSocket(url);
  ws.onopen = () => console.log('[ws] open', url);
  ws.onmessage = (e) => {
    try { onMsg(JSON.parse(e.data) as ServerMsg); } catch (err) { console.error('[ws] bad msg', err); }
  };
  ws.onclose = () => {
    console.warn('[ws] closed, retrying in 2s');
    setTimeout(() => connect(url, onMsg), 2000);
  };
  ws.onerror = (e) => console.error('[ws] error', e);
  return ws;
}
```

- [ ] **Step 8: stage/world-stage.ts**

```ts
import { Application, Container } from 'pixi.js';
import { TileLayer } from './tile-layer';
import { AgentLayer } from './agent-layer';
import type { AgentMsg, TileMsg } from '../types';

export class WorldStage {
  app!: Application;
  root!: Container;
  tiles!: TileLayer;
  agents!: AgentLayer;

  async mount(el: HTMLElement) {
    this.app = new Application();
    await this.app.init({ background: '#1a1816', resizeTo: el, antialias: false });
    el.appendChild(this.app.canvas);
    this.root = new Container();
    this.app.stage.addChild(this.root);
    this.tiles = new TileLayer();
    this.agents = new AgentLayer();
    this.root.addChild(this.tiles.container);
    this.root.addChild(this.agents.container);
  }

  setGrid(width: number, height: number, tiles: TileMsg[]) {
    this.tiles.render(width, height, tiles);
    // 居中
    const tileSize = this.tiles.tileSize;
    const totalW = width * tileSize;
    const totalH = height * tileSize;
    const sx = this.app.renderer.width / totalW;
    const sy = this.app.renderer.height / totalH;
    const s = Math.min(sx, sy);
    this.root.scale.set(s);
    this.root.x = (this.app.renderer.width - totalW * s) / 2;
    this.root.y = (this.app.renderer.height - totalH * s) / 2;
  }

  setAgents(agents: AgentMsg[]) {
    this.agents.render(agents, this.tiles.tileSize);
  }
}
```

- [ ] **Step 9: stage/tile-layer.ts**

```ts
import { Container, Graphics } from 'pixi.js';
import type { TileMsg } from '../types';

const TILE_COLOR: Record<string, number> = {
  grass: 0x5C8C6A,
  bamboo_forest: 0x3F6E4D,
  pine_forest: 0x2E5C3F,
  reed: 0x8FA76A,
  maple: 0xB83A2E,
  sand: 0xD9A441,
  stone: 0x6F6A60,
  mountain: 0x3A3632,
  shallow_water: 0x4A7A8C,
  deep_water: 0x2A5260,
  ruin: 0x5C4A3E,
  road: 0x8C7A5C,
  ash: 0x2A2826,
};

export class TileLayer {
  container = new Container();
  tileSize = 12;

  render(width: number, height: number, tiles: TileMsg[]) {
    this.container.removeChildren();
    const g = new Graphics();
    for (const t of tiles) {
      const c = TILE_COLOR[t.kind] ?? 0xff00ff;
      g.rect(t.pos.x * this.tileSize, t.pos.y * this.tileSize, this.tileSize, this.tileSize).fill(c);
    }
    this.container.addChild(g);
  }
}
```

- [ ] **Step 10: stage/agent-layer.ts**

```ts
import { Container, Graphics, Text } from 'pixi.js';
import type { AgentMsg } from '../types';

export class AgentLayer {
  container = new Container();

  render(agents: AgentMsg[], tileSize: number) {
    this.container.removeChildren();
    for (const a of agents) {
      const g = new Graphics();
      const cx = a.pos.x * tileSize + tileSize / 2;
      const cy = a.pos.y * tileSize + tileSize / 2;
      // hash color
      let h = 0;
      for (const ch of a.id) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
      const color = 0xD9A441 ^ (h & 0xFFFFFF);
      g.circle(cx, cy, tileSize * 0.4).fill(color);
      g.circle(cx, cy, tileSize * 0.4).stroke({ color: 0xF2EFE4, width: 1 });
      this.container.addChild(g);
      const label = new Text({ text: a.name, style: { fontSize: 8, fill: 0xF2EFE4, fontFamily: 'monospace' } });
      label.x = cx - label.width / 2;
      label.y = cy - tileSize;
      this.container.addChild(label);
    }
  }
}
```

- [ ] **Step 11: main.ts**

```ts
import './hud/style.css';
import { connect } from './ws';
import { WorldStage } from './stage/world-stage';
import type { ServerMsg, SnapshotMsg } from './types';

const stage = new WorldStage();
const stageEl = document.getElementById('stage')!;
const clockEl = document.getElementById('clock')!;
const agentsEl = document.getElementById('agents')! as HTMLUListElement;
const eventsEl = document.getElementById('events')! as HTMLUListElement;

let snapshot: SnapshotMsg | null = null;

await stage.mount(stageEl);

connect('ws://127.0.0.1:7777/ws/spectator', (m: ServerMsg) => {
  if (m.kind === 'snapshot') {
    snapshot = m;
    stage.setGrid(m.grid_width, m.grid_height, m.tiles);
    stage.setAgents(m.agents);
    clockEl.textContent = `tick ${m.tick}`;
  } else if (m.kind === 'tick') {
    if (!snapshot) return;
    stage.setAgents(m.view.agents);
    clockEl.textContent = `tick ${m.view.tick}`;
    renderAgents(m.view.agents);
    pushEvents(m.view.events);
  }
});

function renderAgents(agents: { name: string; hp: number }[]) {
  agentsEl.innerHTML = '';
  for (const a of agents) {
    const li = document.createElement('li');
    li.textContent = `${a.name}  HP ${a.hp}`;
    agentsEl.appendChild(li);
  }
}

function pushEvents(events: { kind: string; data?: unknown }[]) {
  for (const e of events) {
    const li = document.createElement('li');
    li.textContent = `${e.kind} ${e.data ? JSON.stringify(e.data) : ''}`;
    eventsEl.prepend(li);
    while (eventsEl.children.length > 50) eventsEl.removeChild(eventsEl.lastChild!);
  }
}
```

- [ ] **Step 12: install + build smoke**

```bash
cd /Users/e0_7/projects/games/lingyuan/frontend
pnpm install || npm install
pnpm build || npx vite build
```

Expected: build succeeds, generates `dist/`.

- [ ] **Step 13: commit**

```bash
cd /Users/e0_7/projects/games/lingyuan
git add frontend
git commit -m "feat(frontend): Vite + PixiJS observer (tiles + agents)"
```

---

## Task 20: 整体 smoke：server + frontend + 2 个 CLI agent 同台

**Files:**
- Create: `scripts/demo.sh`

- [ ] **Step 1: 写 demo 脚本**

`scripts/demo.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build -p server -p cli
( cd frontend && (pnpm install || npm install) && (pnpm build || npx vite build) )

rm -rf data
LINGYUAN_TICK_MS=1000 ./target/debug/server > /tmp/lingyuan-server.log 2>&1 &
SERVER_PID=$!
( cd frontend && (pnpm dev || npx vite) > /tmp/lingyuan-fe.log 2>&1 ) &
FE_PID=$!

trap "kill $SERVER_PID $FE_PID 2>/dev/null || true" EXIT
sleep 3

echo "Open http://127.0.0.1:5173 in your browser."

./target/debug/survivor clear || true
./target/debug/survivor join --name alice --server http://localhost:7777
# 第二个 agent 用独立 token store？目前 token 文件单一，先 demo 第二个用 raw curl
SECOND=$(curl -s -X POST http://localhost:7777/api/v1/join -H 'Content-Type: application/json' -d '{"name":"bob"}')
BID=$(echo "$SECOND" | grep -o '"agent_id":"[^"]*"' | cut -d'"' -f4)
BTOK=$(echo "$SECOND" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
echo "alice joined (CLI), bob joined (curl, id=$BID)"

# 让 alice 随机走 20 tick
for i in $(seq 1 20); do
  DIR=$(echo "north south east west" | tr ' ' '\n' | shuf -n1)
  ./target/debug/survivor act move --dir="$DIR" >/dev/null || true
  curl -s -X POST http://localhost:7777/api/v1/act \
    -H 'Content-Type: application/json' \
    -H "Authorization: Bearer $BTOK" \
    -H "X-Agent-Id: $BID" \
    -d "{\"kind\":\"move\",\"data\":{\"dir\":\"$DIR\"}}" >/dev/null || true
  sleep 1.2
done

echo "Demo done. Press Ctrl-C to stop server + frontend."
wait
```

```bash
chmod +x scripts/demo.sh
git add scripts/demo.sh
git commit -m "test: scripts/demo.sh 多 agent 同台 demo"
```

- [ ] **Step 2: 运行 demo 跑 30 秒确认能起来**

Run: `bash scripts/demo.sh` 后台运行 30s 然后 Ctrl-C。  
Expected: server log 显示 alice 和 bob join，每 tick log 出现，前端日志 build ok，浏览器打开 :5173 能看到 80×80 网格 + 两个圆点在动。

(此 step 是手工目视确认；如果 CI 跑无 X server，跳过 step 2 仅做 step 1。)

---

## Self-Review

跑完上面 20 个 task，spec 覆盖情况：

| Spec 段 | 对应 Task |
|---------|-----------|
| §3.1 Workspace 布局 | Task 0 |
| §3.2 运行时拓扑 | Task 10-15 |
| §3.3 World 数据模型（M1 子集：agents + grid + clock）| Task 1-9 |
| §3.4 Tick loop | Task 11 |
| §3.5 持久化 | Task 12 |
| §3.6 事件日志 | Task 12 |
| §4.1 REST surface（M1 子集：health/clock/join/observe/act/leave）| Task 14 |
| §4.2 Observation schema（M1 子集）| Task 9 |
| §4.3 Action schema（M1 子集：move/wait）| Task 5, 17 |
| §4.5 CLI（M1 子集）| Task 17 |
| §5.1-5.3 UI（M1 子集：tile + agent layer + WS）| Task 15, 19 |
| §7.1 World 单测 | Task 1-9 内联 |
| §7.2 集成测试 | Task 16, 20 |
| §7.3 决定论 | Task 8 末测试 |

**留给后续里程碑的 spec 内容**：所有 §2.x 玩法机制（饥饿/合成/战斗/建筑/季节/boss）、§4.6 skill markdown 完整版、§5.4 水墨 UI 设计语言、§5.5 sprite GPT 工作流、§6 错误处理高级部分、§7.4 性能基准、附录配方/术语。这些会在 M3-M10 plan 里逐次实施。

**类型一致性检查**：`Action::Wait`/`Action::Move`/`Action::Observe` 在 world、server::routes::act、cli::main 三处一致。`AgentId` 在所有 crate 都从 world 引。`TickEvent` 命名一致。

**占位符扫描**：无 TODO/TBD；每个 code step 都给了完整代码。

---

## Plan complete

文件：`/Users/e0_7/projects/games/lingyuan/docs/superpowers/plans/2026-05-27-m1-m2-skeleton-and-world.md`

执行方式：用户已设置「一直推就行」目标，将直接进入执行（不再二次确认），用 `superpowers:executing-plans` skill 串行跑完 20 个 task。
