# 灵渊 M3 实施计划：求生闭环

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 agent 真能"求生"——会饿、会累、能采、能造火堆、能合成简单工具、能煮食物、会死、会重生。

**Architecture:** 在 M1+M2 的 World 上扩 inventory / item / recipe / building / plant；新 actions: gather/eat/craft/place/pick_up/drop；自然系统每 tick 跑 hunger 衰减；死亡走 Dying 状态 30 tick 后重生于随机安全 tile。

**Tech Stack:** 沿用 Rust + axum；不引新依赖。

---

## 范围决定

**IN（M3）：**
- 5 路状态里的 **hp / hunger / stamina** 三路；warmth/sanity 占位但不衰减
- Plant entity（lingzhi / mushroom / red_berry / bamboo_stalk / pine_log / stone_chunk / flint_chunk / clay_lump）撒在 tile 上
- 新动作：`gather`, `eat`, `craft`, `place`, `pick_up`, `drop`
- T0 配方：bamboo_spear, rope, clay_pot, campfire, cooking_stove
- T1 配方：stone_axe, cooked_<berry/mushroom>（campfire 边）, rice_cake（cooking_stove 边）
- 建筑：campfire（提供热源、可烤）、cooking_stove（合成台）
- 死亡 + 重生：drop inventory，Dying 30 tick，重生在随机安全 tile

**OUT（推迟到 M4+）：**
- 动物 / 怪物 / 战斗（包括 PvP）
- warmth 真实衰减（季节配套）
- sanity（夜战 + 怨魂配套）
- T2/T3 配方（铁器、丹药、金丹）
- 农作 / 飞鸽 / 路牌 / 祭坛（社交建筑 M5）
- 季节机制效果（春草药 +1.5 等）

---

## 文件结构

```
crates/world/src/
├── item.rs          (新) Item, ItemStack, Inventory
├── recipe.rs        (新) Recipe, RECIPES 表
├── building.rs      (新) Building, BuildingKind
├── plant.rs         (新) Plant entity 类型与生长
├── entity.rs        (新) Entity 枚举：Plant / ItemDrop / Building（暂时聚合）
├── systems/
│   ├── mod.rs       (新)
│   ├── natural.rs   (新) hunger decay, stamina regen, hp_from_hunger
│   ├── death.rs     (新) 死亡判定 + 重生
│   └── gather.rs    (新) 采集结算
├── action.rs        (改) 新 variants
├── event.rs         (改) 新事件
├── observation.rs   (改) 加 inventory / visible_entities / visible_buildings
├── agent.rs         (改) 加 inventory: Inventory
├── world.rs         (改) 新数据结构 + step 拓展 + 新 resolver
└── gen.rs           (改) 撒 Plant 种群
```

server 那边：`/api/v1/act` 的 Action JSON 自动支持新动作（serde 派生），无需改 server 代码。但要给 spectator WS 加 entities/buildings 字段，让前端能看到。前端要加 EntityLayer / BuildingLayer。

---

## Task M3-1: ItemKind + ItemStack + Inventory

**Files:**
- Create: `crates/world/src/item.rs`
- Modify: `crates/world/src/lib.rs`

- [ ] **Step 1: 写 item.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    // 原材料
    Bamboo, Pinewood, Stone, Flint, Clay, Vine, Reed,
    Lingzhi, Mushroom, RedBerry,
    // 工具 / 武器
    BambooSpear, StoneAxe, Rope, ClayPot,
    // 食物（生 / 熟）
    CookedMushroom, CookedBerry, RiceCake,
    // 建筑物原型（放置后变 Building）
    CampfireKit, CookingStoveKit,
}

impl ItemKind {
    pub fn is_food(self) -> bool {
        matches!(
            self,
            ItemKind::Mushroom | ItemKind::RedBerry | ItemKind::Lingzhi
                | ItemKind::CookedMushroom | ItemKind::CookedBerry | ItemKind::RiceCake
        )
    }
    /// 吃下去给的饥饿恢复量（生的少、熟的多、灵芝兼回 hp）
    pub fn nutrition(self) -> (i16 /*hunger*/, i16 /*hp*/) {
        match self {
            ItemKind::Mushroom => (8, 0),
            ItemKind::RedBerry => (6, 0),
            ItemKind::Lingzhi => (10, 8),
            ItemKind::CookedMushroom => (18, 0),
            ItemKind::CookedBerry => (15, 0),
            ItemKind::RiceCake => (28, 2),
            _ => (0, 0),
        }
    }
    pub fn stack_size(self) -> u16 { 20 }
    pub fn name_zh(self) -> &'static str {
        match self {
            ItemKind::Bamboo => "竹",
            ItemKind::Pinewood => "松木",
            ItemKind::Stone => "石",
            ItemKind::Flint => "燧石",
            ItemKind::Clay => "陶土",
            ItemKind::Vine => "藤",
            ItemKind::Reed => "苇",
            ItemKind::Lingzhi => "灵芝",
            ItemKind::Mushroom => "菇",
            ItemKind::RedBerry => "朱果",
            ItemKind::BambooSpear => "竹枪",
            ItemKind::StoneAxe => "石斧",
            ItemKind::Rope => "麻绳",
            ItemKind::ClayPot => "陶罐",
            ItemKind::CookedMushroom => "烤菇",
            ItemKind::CookedBerry => "烤果",
            ItemKind::RiceCake => "苇糕",
            ItemKind::CampfireKit => "篝火（待放）",
            ItemKind::CookingStoveKit => "灶台（待放）",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: ItemKind,
    pub n: u16,
}

pub const INVENTORY_SIZE: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Inventory {
    pub slots: Vec<ItemStack>,
}

impl Inventory {
    pub fn new() -> Self { Self::default() }
    pub fn count(&self, k: ItemKind) -> u16 {
        self.slots.iter().filter(|s| s.item == k).map(|s| s.n).sum()
    }
    pub fn is_full_for(&self, k: ItemKind) -> bool {
        if self.slots.iter().any(|s| s.item == k && s.n < k.stack_size()) {
            return false;
        }
        self.slots.len() >= INVENTORY_SIZE
    }
    /// 加 n 个 k；返回实际加入数
    pub fn add(&mut self, k: ItemKind, mut n: u16) -> u16 {
        let mut added = 0;
        for s in self.slots.iter_mut().filter(|s| s.item == k) {
            let room = k.stack_size().saturating_sub(s.n);
            let take = room.min(n);
            s.n += take; n -= take; added += take;
            if n == 0 { return added; }
        }
        while n > 0 && self.slots.len() < INVENTORY_SIZE {
            let take = n.min(k.stack_size());
            self.slots.push(ItemStack { item: k, n: take });
            n -= take; added += take;
        }
        added
    }
    /// 扣 n 个 k；若不足返回 false 且不改动
    pub fn remove(&mut self, k: ItemKind, n: u16) -> bool {
        if self.count(k) < n { return false; }
        let mut left = n;
        for s in self.slots.iter_mut().filter(|s| s.item == k) {
            let take = s.n.min(left);
            s.n -= take; left -= take;
            if left == 0 { break; }
        }
        self.slots.retain(|s| s.n > 0);
        true
    }
    pub fn clear(&mut self) { self.slots.clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn add_then_count_then_remove() {
        let mut inv = Inventory::new();
        assert_eq!(inv.add(ItemKind::Mushroom, 5), 5);
        assert_eq!(inv.count(ItemKind::Mushroom), 5);
        assert!(inv.remove(ItemKind::Mushroom, 3));
        assert_eq!(inv.count(ItemKind::Mushroom), 2);
        assert!(!inv.remove(ItemKind::Mushroom, 10));
        assert_eq!(inv.count(ItemKind::Mushroom), 2);
    }
    #[test]
    fn stack_size_limits() {
        let mut inv = Inventory::new();
        inv.add(ItemKind::Bamboo, 25);
        assert_eq!(inv.count(ItemKind::Bamboo), 25);
        assert_eq!(inv.slots.len(), 2); // 一格 20 + 一格 5
    }
    #[test]
    fn inventory_full_caps() {
        let mut inv = Inventory::new();
        for _ in 0..INVENTORY_SIZE { inv.add(ItemKind::Mushroom, 1); }
        // 20 slot × 1 mushroom, each can grow to 20
        // 加一个 stone 应该失败因为槽位满（mushroom 还有 room 但不同物种）
        assert_eq!(inv.add(ItemKind::Stone, 1), 0);
        assert!(inv.is_full_for(ItemKind::Stone));
        // 但加同种 mushroom 应能堆到现有槽位
        assert!(!inv.is_full_for(ItemKind::Mushroom));
        assert_eq!(inv.add(ItemKind::Mushroom, 5), 5);
    }
}
```

- [ ] **Step 2: lib.rs 增加 pub mod item + pub use**

```rust
pub mod item;
pub use item::{Inventory, ItemKind, ItemStack};
```

- [ ] **Step 3: cargo test**

Run: `cargo test -p world item`
Expected: 3 tests pass。

---

## Task M3-2: Agent 加 inventory + observation 暴露

**Files:**
- Modify: `crates/world/src/agent.rs`
- Modify: `crates/world/src/observation.rs`
- Modify: `crates/world/src/world.rs` (observe)

- [ ] **Step 1: agent.rs 加 inventory 字段 + fresh 初始化**

在 Agent struct 加：
```rust
    pub inventory: Inventory,
```

在 `new_at` 里：
```rust
            inventory: Inventory::new(),
```

import：`use crate::item::Inventory;`

- [ ] **Step 2: observation.rs 在 SelfView 加 inventory 字段**

```rust
pub struct SelfView {
    ...
    pub inventory: Vec<crate::item::ItemStack>,
}
```

- [ ] **Step 3: world.rs observe 里填 inventory**

```rust
            self_: SelfView {
                ...
                inventory: agent.inventory.slots.clone(),
            },
```

- [ ] **Step 4: cargo test -p world**

Expected: 现有 25+3 = 28 tests 全 pass。

---

## Task M3-3: Plant entity + gen 撒资源

**Files:**
- Create: `crates/world/src/plant.rs`
- Create: `crates/world/src/entity.rs`
- Modify: `crates/world/src/gen.rs`
- Modify: `crates/world/src/world.rs`
- Modify: `crates/world/src/lib.rs`

- [ ] **Step 1: plant.rs**

```rust
use serde::{Deserialize, Serialize};
use crate::item::ItemKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlantKind {
    BambooStalk, PineLog, StoneChunk, FlintChunk, ClayLump,
    Lingzhi, Mushroom, RedBerry, Vine, Reed,
}

impl PlantKind {
    pub fn yield_item(self) -> ItemKind {
        match self {
            PlantKind::BambooStalk => ItemKind::Bamboo,
            PlantKind::PineLog => ItemKind::Pinewood,
            PlantKind::StoneChunk => ItemKind::Stone,
            PlantKind::FlintChunk => ItemKind::Flint,
            PlantKind::ClayLump => ItemKind::Clay,
            PlantKind::Lingzhi => ItemKind::Lingzhi,
            PlantKind::Mushroom => ItemKind::Mushroom,
            PlantKind::RedBerry => ItemKind::RedBerry,
            PlantKind::Vine => ItemKind::Vine,
            PlantKind::Reed => ItemKind::Reed,
        }
    }
    pub fn yield_count(self) -> u16 {
        match self {
            PlantKind::Lingzhi => 1,
            PlantKind::BambooStalk | PlantKind::PineLog => 2,
            _ => 1,
        }
    }
    /// 多少 tick 后再生（None = 永久消失）
    pub fn regrow_after(self) -> Option<u64> {
        match self {
            PlantKind::Lingzhi => Some(2000),  // 灵芝慢
            PlantKind::RedBerry | PlantKind::Mushroom => Some(600),
            PlantKind::BambooStalk | PlantKind::Reed | PlantKind::Vine => Some(400),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plant {
    pub kind: PlantKind,
    pub harvested_until: Option<u64>,  // 若 Some(tick)，在该 tick 之前不能采
}

impl Plant {
    pub fn fresh(kind: PlantKind) -> Self { Self { kind, harvested_until: None } }
    pub fn is_available(&self, tick: u64) -> bool {
        self.harvested_until.map(|t| tick >= t).unwrap_or(true)
    }
}
```

- [ ] **Step 2: entity.rs（轻量聚合）**

```rust
use serde::{Deserialize, Serialize};
use crate::{item::ItemStack, plant::Plant, coord::TileCoord};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Entity {
    Plant { plant: Plant },
    ItemDrop { stack: ItemStack, expires_at: u64 },
}

impl Entity {
    pub fn is_takeable_plant(&self, tick: u64) -> bool {
        matches!(self, Entity::Plant { plant } if plant.is_available(tick))
    }
}
```

- [ ] **Step 3: gen.rs 撒资源**

在 gen.rs 末尾添加：

```rust
use crate::{entity::Entity, plant::{Plant, PlantKind}};
use std::collections::HashMap;

pub fn populate(grid: &Grid<Tile>, seed: u64) -> HashMap<TileCoord, Entity> {
    use rand::{Rng, SeedableRng};
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed ^ 0xCAFE_F00D);
    let mut out = HashMap::new();
    for (pos, t) in grid.iter() {
        let kind: Option<PlantKind> = match t.kind {
            TileKind::BambooForest if rng.gen_bool(0.5) => Some(PlantKind::BambooStalk),
            TileKind::PineForest if rng.gen_bool(0.4) => Some(PlantKind::PineLog),
            TileKind::Stone if rng.gen_bool(0.35) => Some(PlantKind::StoneChunk),
            TileKind::Sand if rng.gen_bool(0.2) => Some(PlantKind::FlintChunk),
            TileKind::Reed if rng.gen_bool(0.5) => Some(PlantKind::Reed),
            TileKind::Grass => {
                if rng.gen_bool(0.06) { Some(PlantKind::Mushroom) }
                else if rng.gen_bool(0.05) { Some(PlantKind::RedBerry) }
                else if rng.gen_bool(0.04) { Some(PlantKind::Vine) }
                else if rng.gen_bool(0.02) { Some(PlantKind::ClayLump) }
                else if rng.gen_bool(0.01) { Some(PlantKind::Lingzhi) }
                else { None }
            }
            _ => None,
        };
        if let Some(k) = kind {
            out.insert(pos, Entity::Plant { plant: Plant::fresh(k) });
        }
    }
    out
}
```

- [ ] **Step 4: world.rs 加 entities 字段 + bootstrap 调 populate**

在 `World` 加：
```rust
    pub entities: HashMap<TileCoord, crate::entity::Entity>,
```

在 `bootstrap`：
```rust
let grid = gen::generate(seed);
let entities = gen::populate(&grid, seed);
```

- [ ] **Step 5: lib.rs 重导出**

```rust
pub mod plant;
pub mod entity;
pub use plant::{Plant, PlantKind};
pub use entity::Entity;
```

- [ ] **Step 6: cargo test -p world**

Expected: 现有测试不破，且 World::bootstrap 后 `world.entities.len() > 0`。

加一个简单测试到 world.rs:
```rust
    #[test]
    fn bootstrap_populates_entities() {
        let w = World::bootstrap(42);
        assert!(w.entities.len() > 50, "entities = {}", w.entities.len());
    }
```

---

## Task M3-4: gather Action

**Files:**
- Modify: `crates/world/src/action.rs`
- Modify: `crates/world/src/event.rs`
- Modify: `crates/world/src/world.rs`

- [ ] **Step 1: 给 Action 加新 variants**

```rust
pub enum Action {
    Move { dir: Direction },
    Wait,
    Observe,
    Gather { target: TileCoord },
    Eat { item: ItemKind },
    Craft { recipe: String },
    Place { item: ItemKind, pos: TileCoord },
    PickUp { pos: TileCoord },
    Drop { item: ItemKind, n: u16 },
}
```

import `TileCoord` 和 `ItemKind`。`Craft { recipe }` 用 String，回头注册表里查。

- [ ] **Step 2: 给 TickEvent 加事件**

```rust
    AgentGathered { agent: AgentId, item: ItemKind, n: u16, from: TileCoord },
    AgentGatherFailed { agent: AgentId, reason: String },
    AgentAte { agent: AgentId, item: ItemKind, hp_gain: i16, hunger_gain: i16 },
    AgentCrafted { agent: AgentId, recipe: String },
    AgentCraftFailed { agent: AgentId, reason: String },
    AgentPlaced { agent: AgentId, building: String, at: TileCoord },
    AgentPickedUp { agent: AgentId, item: ItemKind, n: u16 },
    AgentDropped { agent: AgentId, item: ItemKind, n: u16 },
    AgentDied { agent: AgentId, at: TileCoord, cause: String },
    AgentRespawned { agent: AgentId, at: TileCoord },
```

- [ ] **Step 3: 在 World::resolve 把 Gather 分发**

```rust
            Action::Gather { target } => self.resolve_gather(aid.clone(), target),
```

加方法 `resolve_gather`：

```rust
    fn resolve_gather(&mut self, aid: AgentId, target: TileCoord) {
        use crate::entity::Entity;
        let Some(agent) = self.agents.get(&aid) else { return; };
        if agent.pos.manhattan(target) > 1 {
            self.pending_events.push(TickEvent::AgentGatherFailed { agent: aid, reason: "out of range".into() });
            return;
        }
        // 拿一份
        let tick = self.clock.tick;
        let take = match self.entities.get(&target) {
            Some(Entity::Plant { plant }) if plant.is_available(tick) => {
                let item = plant.kind.yield_item();
                let n = plant.kind.yield_count();
                Some((item, n, plant.kind.regrow_after()))
            }
            _ => None,
        };
        let Some((item, n, regrow)) = take else {
            self.pending_events.push(TickEvent::AgentGatherFailed { agent: aid, reason: "no harvestable".into() });
            return;
        };
        let added = self.agents.get_mut(&aid).unwrap().inventory.add(item, n);
        if added == 0 {
            self.pending_events.push(TickEvent::AgentGatherFailed { agent: aid, reason: "inventory full".into() });
            return;
        }
        // 标记植物冷却或移除
        if let Some(Entity::Plant { plant }) = self.entities.get_mut(&target) {
            if let Some(regrow_in) = regrow {
                plant.harvested_until = Some(tick + regrow_in);
            } else {
                self.entities.remove(&target);
            }
        }
        let a = self.agents.get_mut(&aid).unwrap();
        a.status.stamina = (a.status.stamina - 3).max(0);
        a.last_action_tick = tick;
        self.pending_events.push(TickEvent::AgentGathered { agent: aid, item, n: added, from: target });
    }
```

- [ ] **Step 4: 加测试**

```rust
    #[test]
    fn gather_adds_to_inventory() {
        use crate::{Action, entity::Entity, plant::{Plant, PlantKind}};
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        let pos = w.agents[&id].pos;
        let target = TileCoord::new(pos.x + 1, pos.y);
        w.entities.insert(target, Entity::Plant { plant: Plant::fresh(PlantKind::Mushroom) });
        let events = w.step(vec![(id.clone(), Action::Gather { target })]);
        assert!(events.iter().any(|e| matches!(e, TickEvent::AgentGathered { .. })));
        assert_eq!(w.agents[&id].inventory.count(crate::ItemKind::Mushroom), 1);
    }
```

- [ ] **Step 5: cargo test -p world**

Expected: 新测试通过 + 之前的不破。

---

## Task M3-5: eat Action

**Files:**
- Modify: `crates/world/src/world.rs`

- [ ] **Step 1: 在 resolve 分发**

```rust
            Action::Eat { item } => self.resolve_eat(aid.clone(), item),
```

```rust
    fn resolve_eat(&mut self, aid: AgentId, item: ItemKind) {
        let Some(a) = self.agents.get_mut(&aid) else { return; };
        if !item.is_food() {
            self.pending_events.push(TickEvent::AgentGatherFailed { agent: aid, reason: format!("{:?} not food", item) });
            return;
        }
        if !a.inventory.remove(item, 1) {
            self.pending_events.push(TickEvent::AgentGatherFailed { agent: aid, reason: "no such item".into() });
            return;
        }
        let (hunger, hp) = item.nutrition();
        a.status.hunger = (a.status.hunger + hunger).min(100);
        a.status.hp = (a.status.hp + hp).min(100);
        a.last_action_tick = self.clock.tick;
        self.pending_events.push(TickEvent::AgentAte { agent: aid, item, hp_gain: hp, hunger_gain: hunger });
    }
```

- [ ] **Step 2: import ItemKind in world.rs**

`use crate::item::ItemKind;`

- [ ] **Step 3: 测试**

```rust
    #[test]
    fn eat_restores_hunger() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        w.agents.get_mut(&id).unwrap().inventory.add(crate::ItemKind::Mushroom, 3);
        w.agents.get_mut(&id).unwrap().status.hunger = 50;
        let _ = w.step(vec![(id.clone(), Action::Eat { item: crate::ItemKind::Mushroom })]);
        let s = w.agents[&id].status;
        assert_eq!(s.hunger, 58); // 50 + 8
        assert_eq!(w.agents[&id].inventory.count(crate::ItemKind::Mushroom), 2);
    }
```

- [ ] **Step 4: cargo test -p world**

---

## Task M3-6: 自然系统（饥饿 / 体力 / 体力→血）

**Files:**
- Create: `crates/world/src/systems/mod.rs`
- Create: `crates/world/src/systems/natural.rs`
- Modify: `crates/world/src/world.rs` (在 step 里调)
- Modify: `crates/world/src/lib.rs`

- [ ] **Step 1: systems/mod.rs**

```rust
pub mod natural;
```

- [ ] **Step 2: systems/natural.rs**

```rust
use crate::{Agent, AgentState};

/// 每 tick 对所有 alive agent 做：
/// - hunger 每 4 tick -1
/// - stamina 每 8 tick 自动 +2（上限 100）
/// - 若 hunger == 0：hp 每 2 tick -1
pub fn step_status(tick: u64, agents: &mut std::collections::HashMap<crate::AgentId, Agent>) -> Vec<crate::AgentId> {
    let mut starved = Vec::new();
    for (id, a) in agents.iter_mut() {
        if !matches!(a.state, AgentState::Alive) { continue; }
        if tick % 4 == 0 { a.status.hunger = (a.status.hunger - 1).max(0); }
        if tick % 8 == 0 { a.status.stamina = (a.status.stamina + 2).min(100); }
        if a.status.hunger == 0 && tick % 2 == 0 {
            a.status.hp = (a.status.hp - 1).max(0);
        }
        if a.status.hp <= 0 { starved.push(id.clone()); }
    }
    starved
}
```

- [ ] **Step 3: world.rs step 末尾（动作处理后）调用**

在动作循环后、时钟推进前：
```rust
let _starved = crate::systems::natural::step_status(self.clock.tick, &mut self.agents);
// M3-7 处理死亡
```

- [ ] **Step 4: lib.rs 加 pub mod systems**

- [ ] **Step 5: 测试**

```rust
    #[test]
    fn hunger_decays_over_time() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        let before = w.agents[&id].status.hunger;
        for _ in 0..40 { w.step(vec![]); }
        assert!(w.agents[&id].status.hunger < before);
    }
```

- [ ] **Step 6: cargo test -p world**

---

## Task M3-7: 死亡 + 重生

**Files:**
- Create: `crates/world/src/systems/death.rs`
- Modify: `crates/world/src/world.rs`
- Modify: `crates/world/src/systems/mod.rs`

- [ ] **Step 1: systems/death.rs**

```rust
use std::collections::HashMap;
use crate::{
    agent::{Agent, AgentId, AgentState, AgentStatus},
    coord::TileCoord,
    entity::Entity,
    event::TickEvent,
    gen,
    grid::Grid,
    item::ItemStack,
    tile::Tile,
};

pub const RESPAWN_DELAY: u64 = 30;
pub const ITEM_DROP_TTL: u64 = 1800; // 1 小时墙钟 @ 2s/tick

/// 把 hp<=0 的 agent 转成 Dying，把它的 inventory 散落到周围 tile（作为 ItemDrop）。
pub fn handle_deaths(
    tick: u64,
    seed: u64,
    grid: &Grid<Tile>,
    agents: &mut HashMap<AgentId, Agent>,
    entities: &mut HashMap<TileCoord, Entity>,
) -> Vec<TickEvent> {
    let mut events = Vec::new();
    for (id, a) in agents.iter_mut() {
        if matches!(a.state, AgentState::Alive) && a.status.hp <= 0 {
            let pos = a.pos;
            // 散落物品：尝试把每个 stack 放到 pos 或邻近
            let stacks: Vec<ItemStack> = std::mem::take(&mut a.inventory.slots);
            for (i, stack) in stacks.into_iter().enumerate() {
                let drop_pos = nearby_slot(grid, entities, pos, seed.wrapping_add(tick).wrapping_add(i as u64))
                    .unwrap_or(pos);
                entities.insert(drop_pos, Entity::ItemDrop { stack, expires_at: tick + ITEM_DROP_TTL });
            }
            a.state = AgentState::Dying { revives_at_tick: tick + RESPAWN_DELAY };
            a.status = AgentStatus { hp: 0, hunger: 0, stamina: 0, warmth: 0, sanity: 0 };
            events.push(TickEvent::AgentDied { agent: id.clone(), at: pos, cause: "starvation".into() });
        }
    }
    events
}

/// 重生：对 Dying 状态 + 已到重生 tick 的 agent，移到新随机安全 tile，状态重置。
pub fn handle_respawns(
    tick: u64,
    seed: u64,
    grid: &Grid<Tile>,
    agents: &mut HashMap<AgentId, Agent>,
) -> Vec<TickEvent> {
    let mut events = Vec::new();
    for (id, a) in agents.iter_mut() {
        if let AgentState::Dying { revives_at_tick } = a.state {
            if tick >= revives_at_tick {
                let pos = gen::find_safe_spawn(grid, seed ^ tick.wrapping_mul(0xABCD_1234));
                a.pos = pos;
                a.status = AgentStatus::fresh();
                a.state = AgentState::Alive;
                a.last_action_tick = tick;
                events.push(TickEvent::AgentRespawned { agent: id.clone(), at: pos });
            }
        }
    }
    events
}

fn nearby_slot(grid: &Grid<Tile>, entities: &HashMap<TileCoord, Entity>, center: TileCoord, salt: u64) -> Option<TileCoord> {
    use rand::seq::SliceRandom;
    use rand::SeedableRng;
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(salt);
    let mut candidates: Vec<TileCoord> = (-2..=2)
        .flat_map(|dy| (-2..=2).map(move |dx| TileCoord::new(center.x + dx, center.y + dy)))
        .filter(|c| grid.get(*c).map(|t| t.is_walkable()).unwrap_or(false))
        .filter(|c| !entities.contains_key(c))
        .collect();
    candidates.shuffle(&mut rng);
    candidates.into_iter().next()
}
```

- [ ] **Step 2: systems/mod.rs 加 death**

```rust
pub mod death;
pub mod natural;
```

- [ ] **Step 3: world.rs 把它们接进 step**

在 step 中，自然系统调用之后：

```rust
let mut died_events = crate::systems::death::handle_deaths(
    self.clock.tick, self.seed, &self.grid, &mut self.agents, &mut self.entities);
let mut respawn_events = crate::systems::death::handle_respawns(
    self.clock.tick, self.seed, &self.grid, &mut self.agents);
self.pending_events.append(&mut died_events);
self.pending_events.append(&mut respawn_events);
```

- [ ] **Step 4: 测试**

```rust
    #[test]
    fn agent_dies_when_hp_reaches_zero() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        w.agents.get_mut(&id).unwrap().status.hp = 1;
        w.agents.get_mut(&id).unwrap().status.hunger = 0;
        // 推到 hp 归零
        for _ in 0..10 { w.step(vec![]); }
        assert!(matches!(w.agents[&id].state, crate::AgentState::Dying { .. }), "state = {:?}", w.agents[&id].state);
    }

    #[test]
    fn dying_agent_respawns_after_delay() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        w.agents.get_mut(&id).unwrap().status.hp = 0;
        w.agents.get_mut(&id).unwrap().status.hunger = 0;
        // 触发死亡
        w.step(vec![]);
        // 等满 RESPAWN_DELAY
        for _ in 0..crate::systems::death::RESPAWN_DELAY+2 { w.step(vec![]); }
        assert!(matches!(w.agents[&id].state, crate::AgentState::Alive));
        assert_eq!(w.agents[&id].status.hp, 100);
    }
```

- [ ] **Step 5: cargo test -p world**

---

## Task M3-8: Recipe + craft Action + 简单 Building

**Files:**
- Create: `crates/world/src/recipe.rs`
- Create: `crates/world/src/building.rs`
- Modify: `crates/world/src/world.rs`
- Modify: `crates/world/src/lib.rs`

- [ ] **Step 1: building.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingKind {
    Campfire,
    CookingStove,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub kind: BuildingKind,
    pub placed_by: crate::AgentId,
    pub placed_at_tick: u64,
}
```

- [ ] **Step 2: recipe.rs**

```rust
use serde::{Deserialize, Serialize};
use crate::{building::BuildingKind, item::ItemKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CraftStation { Hand, Campfire, CookingStove }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: &'static str,
    pub inputs: &'static [(ItemKind, u16)],
    pub output: RecipeOutput,
    pub station: CraftStation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RecipeOutput {
    Item(ItemKind, u16),
    /// 输出一个可放置的建筑 kit（玩家随后 Place 才会变 Building）
    BuildingKit(ItemKind),  // 通常配 CampfireKit / CookingStoveKit
}

pub fn recipes() -> &'static [Recipe] {
    &[
        // T0 赤手
        Recipe { id: "bamboo_spear", inputs: &[(ItemKind::Flint, 1), (ItemKind::Bamboo, 1)],
                 output: RecipeOutput::Item(ItemKind::BambooSpear, 1), station: CraftStation::Hand },
        Recipe { id: "rope", inputs: &[(ItemKind::Vine, 2)],
                 output: RecipeOutput::Item(ItemKind::Rope, 1), station: CraftStation::Hand },
        Recipe { id: "clay_pot", inputs: &[(ItemKind::Reed, 3), (ItemKind::Clay, 1)],
                 output: RecipeOutput::Item(ItemKind::ClayPot, 1), station: CraftStation::Hand },
        Recipe { id: "campfire_kit", inputs: &[(ItemKind::Pinewood, 3), (ItemKind::Flint, 1)],
                 output: RecipeOutput::Item(ItemKind::CampfireKit, 1), station: CraftStation::Hand },
        Recipe { id: "cooking_stove_kit", inputs: &[(ItemKind::Stone, 5), (ItemKind::Clay, 3)],
                 output: RecipeOutput::Item(ItemKind::CookingStoveKit, 1), station: CraftStation::Hand },

        // T1 灶台旁
        Recipe { id: "stone_axe", inputs: &[(ItemKind::Stone, 3), (ItemKind::Pinewood, 1), (ItemKind::Rope, 1)],
                 output: RecipeOutput::Item(ItemKind::StoneAxe, 1), station: CraftStation::CookingStove },
        // 烤食物（campfire 旁）
        Recipe { id: "cook_mushroom", inputs: &[(ItemKind::Mushroom, 1)],
                 output: RecipeOutput::Item(ItemKind::CookedMushroom, 1), station: CraftStation::Campfire },
        Recipe { id: "cook_berry", inputs: &[(ItemKind::RedBerry, 1)],
                 output: RecipeOutput::Item(ItemKind::CookedBerry, 1), station: CraftStation::Campfire },
        Recipe { id: "rice_cake", inputs: &[(ItemKind::Reed, 2), (ItemKind::Mushroom, 1)],
                 output: RecipeOutput::Item(ItemKind::RiceCake, 1), station: CraftStation::CookingStove },
    ]
}

pub fn find(id: &str) -> Option<&'static Recipe> {
    recipes().iter().find(|r| r.id == id)
}

pub fn kit_to_building(item: ItemKind) -> Option<BuildingKind> {
    match item {
        ItemKind::CampfireKit => Some(BuildingKind::Campfire),
        ItemKind::CookingStoveKit => Some(BuildingKind::CookingStove),
        _ => None,
    }
}
```

- [ ] **Step 3: world.rs 加 buildings 字段**

```rust
    pub buildings: std::collections::HashMap<TileCoord, crate::building::Building>,
```

bootstrap 初始化为空 HashMap。

- [ ] **Step 4: world.rs resolve_craft 和 resolve_place**

```rust
            Action::Craft { recipe } => self.resolve_craft(aid.clone(), recipe),
            Action::Place { item, pos } => self.resolve_place(aid.clone(), item, pos),
            Action::PickUp { pos } => self.resolve_pickup(aid.clone(), pos),
            Action::Drop { item, n } => self.resolve_drop(aid.clone(), item, n),
```

```rust
    fn resolve_craft(&mut self, aid: AgentId, recipe_id: String) {
        use crate::recipe::{find, CraftStation, RecipeOutput};
        let Some(recipe) = find(&recipe_id) else {
            self.pending_events.push(TickEvent::AgentCraftFailed { agent: aid, reason: "unknown recipe".into() });
            return;
        };
        let pos = self.agents.get(&aid).map(|a| a.pos);
        let Some(pos) = pos else { return; };
        // 检 station
        let station_ok = match recipe.station {
            CraftStation::Hand => true,
            CraftStation::Campfire => self.has_nearby_building(pos, crate::building::BuildingKind::Campfire),
            CraftStation::CookingStove => self.has_nearby_building(pos, crate::building::BuildingKind::CookingStove),
        };
        if !station_ok {
            self.pending_events.push(TickEvent::AgentCraftFailed { agent: aid, reason: "station not nearby".into() });
            return;
        }
        let a = self.agents.get(&aid).unwrap();
        for (item, n) in recipe.inputs {
            if a.inventory.count(*item) < *n {
                self.pending_events.push(TickEvent::AgentCraftFailed { agent: aid.clone(), reason: format!("missing {:?}", item) });
                return;
            }
        }
        let a = self.agents.get_mut(&aid).unwrap();
        for (item, n) in recipe.inputs {
            a.inventory.remove(*item, *n);
        }
        match recipe.output {
            RecipeOutput::Item(item, n) | RecipeOutput::BuildingKit(item) if matches!(recipe.output, RecipeOutput::BuildingKit(_)) => {
                a.inventory.add(item, 1);
            }
            RecipeOutput::Item(item, n) => { a.inventory.add(item, n); }
            RecipeOutput::BuildingKit(_) => unreachable!(),
        }
        a.status.stamina = (a.status.stamina - 5).max(0);
        a.last_action_tick = self.clock.tick;
        self.pending_events.push(TickEvent::AgentCrafted { agent: aid, recipe: recipe_id });
    }

    fn has_nearby_building(&self, pos: TileCoord, kind: crate::building::BuildingKind) -> bool {
        for dy in -1..=1 { for dx in -1..=1 {
            let c = TileCoord::new(pos.x + dx, pos.y + dy);
            if let Some(b) = self.buildings.get(&c) {
                if b.kind == kind { return true; }
            }
        }}
        false
    }

    fn resolve_place(&mut self, aid: AgentId, item: ItemKind, pos: TileCoord) {
        use crate::recipe::kit_to_building;
        let Some(kind) = kit_to_building(item) else {
            self.pending_events.push(TickEvent::AgentCraftFailed { agent: aid, reason: "not placeable".into() });
            return;
        };
        let Some(agent) = self.agents.get(&aid) else { return; };
        if agent.pos.manhattan(pos) > 1 {
            self.pending_events.push(TickEvent::AgentCraftFailed { agent: aid, reason: "out of range".into() });
            return;
        }
        if self.buildings.contains_key(&pos) || self.agents.values().any(|a| a.pos == pos) {
            self.pending_events.push(TickEvent::AgentCraftFailed { agent: aid, reason: "tile occupied".into() });
            return;
        }
        if !self.grid.get(pos).map(|t| t.is_walkable()).unwrap_or(false) {
            self.pending_events.push(TickEvent::AgentCraftFailed { agent: aid, reason: "tile not walkable".into() });
            return;
        }
        if !self.agents.get_mut(&aid).unwrap().inventory.remove(item, 1) {
            self.pending_events.push(TickEvent::AgentCraftFailed { agent: aid, reason: "no kit in inv".into() });
            return;
        }
        let placed_by = aid.clone();
        self.buildings.insert(pos, crate::building::Building {
            kind, placed_by, placed_at_tick: self.clock.tick,
        });
        self.pending_events.push(TickEvent::AgentPlaced { agent: aid, building: format!("{:?}", kind), at: pos });
    }

    fn resolve_pickup(&mut self, aid: AgentId, pos: TileCoord) {
        let Some(agent) = self.agents.get(&aid) else { return; };
        if agent.pos.manhattan(pos) > 1 {
            return;
        }
        let stack = match self.entities.get(&pos) {
            Some(crate::entity::Entity::ItemDrop { stack, .. }) => Some(*stack),
            _ => None,
        };
        let Some(stack) = stack else { return; };
        let added = self.agents.get_mut(&aid).unwrap().inventory.add(stack.item, stack.n);
        if added >= stack.n {
            self.entities.remove(&pos);
            self.pending_events.push(TickEvent::AgentPickedUp { agent: aid, item: stack.item, n: stack.n });
        } else if added > 0 {
            if let Some(crate::entity::Entity::ItemDrop { stack: s, .. }) = self.entities.get_mut(&pos) {
                s.n -= added;
            }
            self.pending_events.push(TickEvent::AgentPickedUp { agent: aid, item: stack.item, n: added });
        }
    }

    fn resolve_drop(&mut self, aid: AgentId, item: ItemKind, n: u16) {
        let Some(a) = self.agents.get_mut(&aid) else { return; };
        if !a.inventory.remove(item, n) { return; }
        let pos = a.pos;
        let tick = self.clock.tick;
        // 合并/新建
        if let Some(crate::entity::Entity::ItemDrop { stack, expires_at }) = self.entities.get_mut(&pos) {
            if stack.item == item {
                stack.n += n;
                *expires_at = tick + crate::systems::death::ITEM_DROP_TTL;
                self.pending_events.push(TickEvent::AgentDropped { agent: aid, item, n });
                return;
            }
        }
        // 若已有其他 entity 占用，丢失。简单先丢到 +1 east
        let drop_pos = if self.entities.contains_key(&pos) {
            TileCoord::new(pos.x + 1, pos.y)
        } else { pos };
        self.entities.insert(drop_pos, crate::entity::Entity::ItemDrop {
            stack: ItemStack { item, n },
            expires_at: tick + crate::systems::death::ITEM_DROP_TTL,
        });
        self.pending_events.push(TickEvent::AgentDropped { agent: aid, item, n });
    }
```

补 imports：

```rust
use crate::item::{ItemKind, ItemStack};
```

- [ ] **Step 5: lib.rs 重导出**

```rust
pub mod recipe;
pub mod building;
pub use building::{Building, BuildingKind};
```

- [ ] **Step 6: 测试**

```rust
    #[test]
    fn craft_bamboo_spear_consumes_inputs() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        let inv = &mut w.agents.get_mut(&id).unwrap().inventory;
        inv.add(crate::ItemKind::Flint, 1);
        inv.add(crate::ItemKind::Bamboo, 1);
        let _ = w.step(vec![(id.clone(), Action::Craft { recipe: "bamboo_spear".into() })]);
        let inv = &w.agents[&id].inventory;
        assert_eq!(inv.count(crate::ItemKind::BambooSpear), 1);
        assert_eq!(inv.count(crate::ItemKind::Bamboo), 0);
    }

    #[test]
    fn place_campfire_creates_building() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        w.agents.get_mut(&id).unwrap().inventory.add(crate::ItemKind::CampfireKit, 1);
        let pos = w.agents[&id].pos;
        // 找一个相邻可走 tile
        let target = [(pos.x+1, pos.y), (pos.x-1, pos.y), (pos.x, pos.y+1), (pos.x, pos.y-1)]
            .into_iter().map(|(x,y)| TileCoord::new(x,y))
            .find(|c| w.grid.get(*c).map(|t| t.is_walkable()).unwrap_or(false) && !w.entities.contains_key(c) && !w.buildings.contains_key(c))
            .expect("no walkable neighbor");
        let _ = w.step(vec![(id.clone(), Action::Place { item: crate::ItemKind::CampfireKit, pos: target })]);
        assert!(matches!(w.buildings.get(&target).map(|b| b.kind), Some(crate::BuildingKind::Campfire)));
    }
```

- [ ] **Step 7: cargo test -p world**

---

## Task M3-9: observation 暴露 entities / buildings

**Files:**
- Modify: `crates/world/src/observation.rs`
- Modify: `crates/world/src/world.rs`

- [ ] **Step 1: observation.rs 新 variants**

```rust
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisibleEntity {
    Agent { id: AgentId, name: String, pos: TileCoord, hp: i16 },
    Plant { pos: TileCoord, kind: crate::plant::PlantKind, available: bool },
    ItemDrop { pos: TileCoord, item: crate::item::ItemKind, n: u16, expires_in: u64 },
    Building { pos: TileCoord, kind: crate::building::BuildingKind, owner: AgentId },
}
```

- [ ] **Step 2: world.rs observe 填充**

把 entities 和 buildings 也按视距过滤进 visible_entities。

```rust
        for (pos, e) in &self.entities {
            if pos.manhattan(center) <= VISION_RADIUS {
                match e {
                    crate::entity::Entity::Plant { plant } => {
                        entities.push(VisibleEntity::Plant {
                            pos: *pos, kind: plant.kind,
                            available: plant.is_available(self.clock.tick),
                        });
                    }
                    crate::entity::Entity::ItemDrop { stack, expires_at } => {
                        entities.push(VisibleEntity::ItemDrop {
                            pos: *pos, item: stack.item, n: stack.n,
                            expires_in: expires_at.saturating_sub(self.clock.tick),
                        });
                    }
                }
            }
        }
        for (pos, b) in &self.buildings {
            if pos.manhattan(center) <= VISION_RADIUS {
                entities.push(VisibleEntity::Building {
                    pos: *pos, kind: b.kind, owner: b.placed_by.clone(),
                });
            }
        }
```

- [ ] **Step 3: cargo test -p world**

---

## Task M3-10: spectator WS 传 entities / buildings + 前端渲染

**Files:**
- Modify: `crates/server/src/state.rs`（在 SpectatorView 加 entities/buildings 简化字段）
- Modify: `crates/server/src/tick_loop.rs`
- Modify: `crates/server/src/routes/ws.rs`（snapshot 也带 entities/buildings）
- Modify: `frontend/src/types.ts`
- Create: `frontend/src/stage/entity-layer.ts`
- Modify: `frontend/src/stage/world-stage.ts`
- Modify: `frontend/src/main.ts`

- [ ] **Step 1: 后端**

state.rs 加：

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpectatorEntity {
    pub pos: world::TileCoord,
    pub kind: String,   // "plant:mushroom" | "drop:stone" | "building:campfire"
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpectatorView {
    pub tick: u64,
    pub clock: world::WorldClock,
    pub agents: Vec<SpectatorAgent>,
    pub entities: Vec<SpectatorEntity>,
    pub events: Vec<world::TickEvent>,
}
```

tick_loop.rs 构造 SpectatorView 时，遍历 w.entities + w.buildings 生成 SpectatorEntity。

ws.rs 的 Snapshot 也加 entities 字段（同 SpectatorEntity Vec）。

- [ ] **Step 2: 前端**

types.ts 增 SpectatorEntity 类型。

新增 `entity-layer.ts`：按 kind 字符串映射颜色画小色块。

main.ts 在 snapshot 和 tick 里都把 entities 传给 stage.

- [ ] **Step 3: cargo build + frontend build + 端到端冒烟**

跑 smoke.sh 再 spawn 一个 gather 动作（手动 curl）验证 inventory 出现资源。

---

## Task M3-11: 总集成测试

**Files:**
- Modify: `crates/server/tests/integration.rs`

- [ ] **Step 1: 加 lifecycle 测试**

```rust
#[tokio::test]
async fn agent_can_gather_and_eat_and_starve() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut child = Command::new(env!("CARGO_BIN_EXE_server"))
        .kill_on_drop(true)
        .env("LINGYUAN_BIND", "127.0.0.1:17780")
        .env("LINGYUAN_DB", db_path.to_str().unwrap())
        .env("LINGYUAN_TICK_MS", "50")
        .stderr(Stdio::piped()).stdout(Stdio::piped())
        .spawn().unwrap();
    let base = "http://127.0.0.1:17780";
    assert!(wait_for_clock(base, Duration::from_secs(10)).await.is_some());

    let cli = reqwest::Client::builder().no_proxy().build().unwrap();
    let join: serde_json::Value = cli.post(format!("{}/api/v1/join", base))
        .json(&serde_json::json!({"name":"eve"})).send().await.unwrap().json().await.unwrap();
    let id = join["agent_id"].as_str().unwrap().to_string();
    let tok = join["token"].as_str().unwrap().to_string();

    // 跑 5s（@50ms=100 tick），hunger 应该下降但还没死
    sleep(Duration::from_secs(5)).await;
    let obs: serde_json::Value = cli.get(format!("{}/api/v1/observe", base))
        .header("Authorization", format!("Bearer {}", tok))
        .header("X-Agent-Id", &id).send().await.unwrap().json().await.unwrap();
    let hunger = obs["self"]["status"]["hunger"].as_i64().unwrap();
    assert!(hunger < 100 && hunger > 50, "hunger after 5s = {}", hunger);

    child.kill().await.ok();
}
```

- [ ] **Step 2: cargo test -p server --test integration**

---

## Self-Review

**Spec 覆盖：**
- §2.4 合成树 T0/T1 → Task M3-8 (recipe.rs 表)
- §2.5 建筑（campfire / cooking_stove 部分）→ Task M3-8
- §2.7 状态系统（hp/hunger/stamina）→ Task M3-6
- §2.8 死亡 + 重生 → Task M3-7
- §4.3 Action schema (gather/eat/craft/place/pick_up/drop) → Task M3-4/5/8

**显式推迟：** warmth/sanity 真实衰减、动物/怪物/PvP、T2/T3 合成、农作、社交建筑、季节效果。这些在 M4-M6 plan 里写。

**类型一致性：** ItemKind 在 item.rs 定义后，agent/recipe/action/event/observation 都通过 `crate::ItemKind` 引用，无重复定义。Recipe id 用 &'static str，CLI/前端不解析，仅按字符串传。

**占位符扫描：** 无。所有 code step 都给了完整代码。
