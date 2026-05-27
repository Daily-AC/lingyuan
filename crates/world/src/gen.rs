use crate::{Biome, Creature, Grid, Tile, TileCoord, TileKind};
use noise::{NoiseFn, Perlin};
use std::collections::BTreeMap;

pub const WORLD_WIDTH: u16 = 80;
pub const WORLD_HEIGHT: u16 = 80;

pub fn generate(seed: u64) -> Grid<Tile> {
    let biome_noise = Perlin::new((seed & 0xFFFF_FFFF) as u32);
    let detail_noise = Perlin::new(((seed >> 32) & 0xFFFF_FFFF) as u32);
    let mut g = Grid::filled(
        WORLD_WIDTH,
        WORLD_HEIGHT,
        Tile {
            kind: TileKind::Grass,
            biome: Biome::Qingzhu,
        },
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
    match v {
        x if x < -0.5 => Biome::Yueze,
        x if x < -0.1 => Biome::Qingzhu,
        x if x < 0.2 => Biome::Cangsong,
        x if x < 0.6 => Biome::Zhuyang,
        _ => Biome::Heishi,
    }
}

fn tile_kind_for(biome: Biome, d: f64) -> TileKind {
    match biome {
        Biome::Qingzhu => {
            if d > 0.3 {
                TileKind::BambooForest
            } else {
                TileKind::Grass
            }
        }
        Biome::Cangsong => {
            if d > 0.3 {
                TileKind::PineForest
            } else if d < -0.5 {
                TileKind::Mountain
            } else {
                TileKind::Stone
            }
        }
        Biome::Yueze => {
            if d > 0.0 {
                TileKind::Reed
            } else if d < -0.4 {
                TileKind::DeepWater
            } else {
                TileKind::ShallowWater
            }
        }
        Biome::Zhuyang => {
            if d > 0.3 {
                TileKind::Maple
            } else if d < -0.4 {
                TileKind::Sand
            } else {
                TileKind::Grass
            }
        }
        Biome::Heishi => {
            if d > 0.3 {
                TileKind::Ruin
            } else if d < -0.4 {
                TileKind::Mountain
            } else {
                TileKind::Ash
            }
        }
    }
}

pub fn populate(grid: &Grid<Tile>, seed: u64) -> std::collections::BTreeMap<TileCoord, crate::entity::Entity> {
    use crate::{entity::Entity, plant::{Plant, PlantKind}};
    use rand::{Rng, SeedableRng};
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed ^ 0xCAFE_F00D);
    let mut out = std::collections::BTreeMap::new();
    for (pos, t) in grid.iter() {
        let kind: Option<PlantKind> = match t.kind {
            TileKind::BambooForest if rng.gen_bool(0.18) => Some(PlantKind::BambooStalk),
            TileKind::PineForest if rng.gen_bool(0.15) => Some(PlantKind::PineLog),
            TileKind::Stone if rng.gen_bool(0.12) => Some(PlantKind::StoneChunk),
            TileKind::Sand if rng.gen_bool(0.06) => Some(PlantKind::FlintChunk),
            TileKind::Reed if rng.gen_bool(0.18) => Some(PlantKind::Reed),
            TileKind::Grass => {
                if rng.gen_bool(0.022) { Some(PlantKind::Mushroom) }
                else if rng.gen_bool(0.018) { Some(PlantKind::RedBerry) }
                else if rng.gen_bool(0.013) { Some(PlantKind::Vine) }
                else if rng.gen_bool(0.006) { Some(PlantKind::ClayLump) }
                else if rng.gen_bool(0.003) { Some(PlantKind::Lingzhi) }
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

pub fn find_safe_spawn(grid: &Grid<Tile>, seed: u64) -> TileCoord {
    find_safe_spawn_avoiding(grid, &BTreeMap::new(), seed, None, 0)
}

/// 找一个可走 tile 作为出生点，同时满足：
/// - manhattan 距 `hostile_radius` 内没有任何 hostile creature（含 boss）
/// - 若 `avoid` 给定，距其 ≥ `hostile_radius * 2` 远（防止反复送死同一片）
///
/// 最多尝试 2000 次；若实在找不到（地图被怪物洗了），退化为只要 walkable，
/// 至少别让玩家卡在死亡循环。
pub fn find_safe_spawn_avoiding(
    grid: &Grid<Tile>,
    creatures: &BTreeMap<u64, Creature>,
    seed: u64,
    avoid: Option<TileCoord>,
    hostile_radius: u16,
) -> TileCoord {
    use rand::{Rng, SeedableRng};
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed ^ 0xDEAD_BEEF);

    let is_safe = |c: TileCoord| -> bool {
        if hostile_radius == 0 {
            return true;
        }
        for cr in creatures.values() {
            if cr.kind.is_hostile() && c.manhattan(cr.pos) <= hostile_radius {
                return false;
            }
        }
        true
    };
    let is_far_enough = |c: TileCoord| -> bool {
        match avoid {
            Some(a) => c.manhattan(a) >= hostile_radius.max(1) * 2,
            None => true,
        }
    };

    // 先严格找：walkable + 远离 hostile + 远离 avoid
    for _ in 0..2000 {
        let x = rng.gen_range(0..WORLD_WIDTH) as i16;
        let y = rng.gen_range(0..WORLD_HEIGHT) as i16;
        let c = TileCoord::new(x, y);
        if !grid.get(c).map(|t| t.is_walkable()).unwrap_or(false) {
            continue;
        }
        if !is_safe(c) {
            continue;
        }
        if !is_far_enough(c) {
            continue;
        }
        return c;
    }
    // 二次放宽：放弃 avoid 距离，但仍避开 hostile
    for _ in 0..1000 {
        let x = rng.gen_range(0..WORLD_WIDTH) as i16;
        let y = rng.gen_range(0..WORLD_HEIGHT) as i16;
        let c = TileCoord::new(x, y);
        if grid.get(c).map(|t| t.is_walkable()).unwrap_or(false) && is_safe(c) {
            return c;
        }
    }
    // 兜底：随便给个 walkable
    find_safe_spawn(grid, seed.wrapping_mul(0x9E37_79B1))
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

    #[test]
    fn safe_spawn_avoids_hostile_creature() {
        use crate::creature::{Creature, CreatureKind};
        let g = generate(123);
        // 在 walkable 候选附近塞一只 boss
        let walkable = find_safe_spawn(&g, 999);
        let mut creatures = BTreeMap::new();
        creatures.insert(
            1,
            Creature::new(1, CreatureKind::BossDujie, walkable, 0),
        );
        // 用很多不同 seed 探 50 次，每次出生点都得在 hostile_radius 之外
        for s in 0..50u64 {
            let c = find_safe_spawn_avoiding(&g, &creatures, s, None, 6);
            assert!(
                c.manhattan(walkable) > 6,
                "spawn {:?} 距 boss {:?} 仅 {}, 应 > 6",
                c, walkable, c.manhattan(walkable)
            );
        }
    }

    #[test]
    fn safe_spawn_avoid_param_distances_from_death_pos() {
        let g = generate(123);
        let death = TileCoord::new(40, 40);
        for s in 0..30u64 {
            let c = find_safe_spawn_avoiding(&g, &BTreeMap::new(), s, Some(death), 6);
            assert!(
                c.manhattan(death) >= 12,
                "spawn {:?} 距死亡点 {:?} 仅 {}, 应 ≥ 12",
                c, death, c.manhattan(death)
            );
        }
    }
}
