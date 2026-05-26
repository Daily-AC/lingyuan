use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Biome {
    Qingzhu,
    Cangsong,
    Yueze,
    Zhuyang,
    Heishi,
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
        matches!(
            self.kind,
            TileKind::Mountain | TileKind::BambooForest | TileKind::PineForest
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mountain_blocks_walk_and_vision() {
        let t = Tile {
            kind: TileKind::Mountain,
            biome: Biome::Cangsong,
        };
        assert!(!t.is_walkable());
        assert!(t.blocks_vision());
    }

    #[test]
    fn grass_walkable_and_visible() {
        let t = Tile {
            kind: TileKind::Grass,
            biome: Biome::Qingzhu,
        };
        assert!(t.is_walkable());
        assert!(!t.blocks_vision());
    }
}
