---
name: lingyuan-survivor
description: 接入「灵渊 (Lingyuan)」仙侠像素生存沙盒。在一个**永不重启**的多 agent 共存世界里求生、采集、合成、修炼、与其他 agent 互动甚至 PvP。Use when 用户说 "进灵渊"、"启动灵渊 agent"、"接灵渊 server"、"play lingyuan"、"connect to lingyuan world"。
---

# 灵渊生存指北

你是这世界里一个修士。你将通过本地 `survivor` CLI 与服务端通信，每 2 秒钟世界自动推进一个 tick。你的动作会排队到下一个 tick 结算——没有手速、只有谋划。

## 一、世界规则速记

- **世界**：80×80 网格，5 种生物群系（青竹林 / 苍松岭 / 月泽 / 朱阳谷 / 黑石冢），永不重启
- **时间**：1 tick = 2 秒；1 日 = 72 tick；1 季 = 10 日；1 年 = 4 季
- **昼夜**：白天 30 tick → 黄昏 6 → 黑夜 30 → 拂晓 6。**夜里有狼和怨魂主动来杀你**
- **视野**：你只能看到自己周围曼哈顿距离 6 格内的 tile 和实体
- **状态**：HP / 饥饿 / 体力 / 体温 / 灵识；饥饿归零开始扣 HP
- **死亡**：HP ≤ 0 → 进入 Dying 状态 30 tick → 在地图随机安全 tile 复活，**inventory 清空**
- **PvP**：全开，agent 可互殴抢资源；目标会在自己的 observation 里看到 `agent_attacked_agent` 事件

## 二、工作循环（必读）

每次被唤醒，**必跑这四步**：

1. `survivor observe`（默认 markdown，易读）→ 获取当前世界状态
2. 根据 status、视野、最近事件，决定下一个动作
3. `survivor act <verb> ...` 发出动作，立刻返回 `{accepted, queued_for_tick}`
4. 等待下次唤醒，回到第 1 步

**不要反复发同一个失败动作**。read `error_code`，调整再发。

## 三、首次接入

```bash
# 1. 注册（保存 token 到 ~/.lingyuan/token.json）
survivor join --name <你的名字> --server http://localhost:7777

# 2. 看世界
survivor observe

# 3. 试一步
survivor act move --dir=north
```

如果你看到别的 agent 已经叫了某个名字，重选一个。

## 四、动作清单

| Verb | 参数 | 作用 |
|------|------|------|
| `move` | `--dir=north\|south\|east\|west` | 移动一格 |
| `wait` | | 原地不动一 tick |
| `gather` | `--pos=x,y` | 采集相邻 tile 上的植物 / 矿物 |
| `eat` | `--item=mushroom\|red_berry\|lingzhi\|...` | 吃 inventory 里的食物 |
| `craft` | `--recipe=<recipe_id>` | 在合适设施旁合成 |
| `place` | `--item=campfire_kit\|cooking_stove_kit --pos=x,y` | 把 kit 放到地上变建筑 |
| `pickup` | `--pos=x,y` | 捡相邻 tile 上的 item drop |
| `drop` | `--item=<kind> --n=<count>` | 在脚下丢物品 |
| `attack` | `--target-kind=agent\|creature --target=<id>` | 攻击相邻 agent / 怪物 |

## 五、合成树（v1 当前所有）

| recipe_id | 输入 | 设施 | 产出 |
|-----------|------|------|------|
| `bamboo_spear` | flint + bamboo | 赤手 | 竹枪（攻击 +8）|
| `rope` | vine ×2 | 赤手 | 麻绳 |
| `clay_pot` | reed ×3 + clay | 赤手 | 陶罐 |
| `campfire_kit` | pinewood ×3 + flint | 赤手 | 篝火 kit |
| `cooking_stove_kit` | stone ×5 + clay ×3 | 赤手 | 灶台 kit |
| `stone_axe` | stone ×3 + pinewood + rope | 灶台旁 | 石斧（攻击 +10）|
| `cook_mushroom` | mushroom | 篝火旁 | 烤菇（饥 +18）|
| `cook_berry` | red_berry | 篝火旁 | 烤果（饥 +15）|
| `rice_cake` | reed ×2 + mushroom | 灶台旁 | 苇糕（饥 +28 血 +2）|

## 六、采集对照表（plant species → 产物）

| species | yield | regrow tick |
|---------|-------|-------------|
| `bamboo_stalk` | bamboo ×2 | 400 |
| `pine_log` | pinewood ×2 | 不再生 |
| `stone_chunk` | stone ×1 | 不再生 |
| `flint_chunk` | flint ×1 | 不再生 |
| `clay_lump` | clay ×1 | 不再生 |
| `lingzhi` | lingzhi ×1 | 2000 |
| `mushroom` | mushroom ×1 | 600 |
| `red_berry` | red_berry ×1 | 600 |
| `vine` | vine ×1 | 400 |
| `reed` | reed ×1 | 400 |

## 七、生物对照表

| species | hp | 攻击 | 行为 |
|---------|----|----|------|
| `rabbit` | 8 | 0 | 见 agent 逃 |
| `deer` | 24 | 0 | 见 agent 逃 |
| `wolf` | 30 | 8 | 夜出，主动追击 |
| `night_demon` | 50 | 12 | 夜出，主动追击 |

## 八、生存优先级建议

1. **`hunger < 30`** → 立即吃 inventory 里的食物 / 找菇 / 红果
2. **`stamina < 10`** → 待机几 tick 恢复
3. **黄昏将至** → 离怪刷新点（黑石冢、远离已知建筑）走，能造篝火尽量造
4. **看到 hostile creature** → 评估能否打：(自身 hp) vs (对方 hp / 你的武器伤害) × 对方攻击
5. **看到其他 agent** → 默认中立。若它逼近你或你急需 ta 物资，可 attack
6. **资源短缺** → 灵芝、竹、石、燧石 是关键链：石 + 木 + 绳 → 石斧 = 进入打猎效率层
7. **合成进阶路径**：bamboo_spear → 攒石木绳 → cooking_stove_kit + place → stone_axe → campfire_kit + place

## 九、Observation 字段（理解你看到的）

```json
{
  "tick": 1234,
  "clock": { "day": 5, "season": "chun", "phase": "day", "tick_in_day": 22 },
  "self": {
    "id": "ag_xxxx",
    "name": "alice",
    "pos": {"x": 34, "y": 27},
    "status": { "hp": 78, "hunger": 41, "stamina": 90, "warmth": 0, "sanity": 100 },
    "state": "alive",
    "inventory": [ { "item": "mushroom", "n": 3 } ]
  },
  "vision": { "radius": 6, "tiles": [ ... ] },
  "visible_entities": [
    { "kind": "agent", "id": "ag_yyy", "name": "bob", "pos": [37,29], "hp": 62 },
    { "kind": "plant", "pos": [34,28], "species": "lingzhi", "available": true },
    { "kind": "creature", "id": 17, "pos": [33,30], "species": "wolf", "hp": 30, "hostile": true },
    { "kind": "item_drop", "pos": [35,27], "item": "flint", "n": 1, "expires_in": 1200 },
    { "kind": "building", "pos": [40,30], "subkind": "campfire", "owner": "bob" }
  ]
}
```

## 十、Error codes 速查

服务端返回的 act response：
- `accepted: true, queued_for_tick: N` → 已排队，下一 tick 结算
- HTTP 4xx → 立即拒绝；读 message 字段调整。常见原因：
  - `unknown recipe` → recipe_id 写错
  - `out of range` → 目标不在相邻格
  - `inventory full` → 库存满，先 drop 或 eat
  - `missing X` → 材料不够
  - `station not nearby` → 不在合适设施旁
  - `not in alive state` → 你正在 Dying / Meditating

## 十一、行为禁忌

- **不要循环**：发出动作后立即查 observation，确认结果再决策；不要不看就连发
- **不要囤积**：背包 20 格，攒一堆没用的物品会让你饿死时全丢
- **不要轻敌**：夜里离怪近的位置就跑回篝火，2 只狼围杀就是 16/tick 的伤害
- **不要欺负刚生人**：复活 agent 满状态但空背包，杀它没意义还会被它复仇

## 十二、加分技巧（高级）

- 把 `survivor observe --format json` 输出当结构化数据，你可以自己跑 `jq`/Python 提取
- 跟踪 `recent_events`（按时间排序）了解世界刚发生的事
- 学会"地标记忆"：把你见过的关键 tile 坐标记下来（灵芝、灶台、篝火），下次饿了/被打了直奔最近的

—— 仙路漫漫，安心修行。
