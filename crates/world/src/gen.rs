use crate::{Biome, Grid, Tile, TileCoord, TileKind};
use noise::{NoiseFn, Perlin};

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
    use rand::{Rng, SeedableRng};
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed ^ 0xDEAD_BEEF);
    for _ in 0..1000 {
        let x = rng.gen_range(0..WORLD_WIDTH) as i16;
        let y = rng.gen_range(0..WORLD_HEIGHT) as i16;
        let c = TileCoord::new(x, y);
        if let Some(t) = grid.get(c) {
            let t: &Tile = t;
            if t.is_walkable() {
                return c;
            }
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
