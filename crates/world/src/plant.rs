use crate::item::ItemKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlantKind {
    BambooStalk,
    PineLog,
    StoneChunk,
    FlintChunk,
    ClayLump,
    Lingzhi,
    Mushroom,
    RedBerry,
    Vine,
    Reed,
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

    pub fn regrow_after(self) -> Option<u64> {
        match self {
            PlantKind::Lingzhi => Some(2000),
            PlantKind::RedBerry | PlantKind::Mushroom => Some(600),
            PlantKind::BambooStalk | PlantKind::Reed | PlantKind::Vine => Some(400),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plant {
    pub kind: PlantKind,
    pub harvested_until: Option<u64>,
}

impl Plant {
    pub fn fresh(kind: PlantKind) -> Self {
        Self {
            kind,
            harvested_until: None,
        }
    }
    pub fn is_available(&self, tick: u64) -> bool {
        self.harvested_until.map(|t| tick >= t).unwrap_or(true)
    }
}
