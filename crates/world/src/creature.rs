use crate::{coord::TileCoord, AgentId, ItemKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreatureKind {
    Rabbit,
    Deer,
    Wolf,
    NightDemon,
    /// boss: 渡劫者，全图通告
    BossDujie,
}

impl CreatureKind {
    pub fn max_hp(self) -> i16 {
        match self {
            CreatureKind::Rabbit => 8,
            CreatureKind::Deer => 24,
            CreatureKind::Wolf => 30,
            CreatureKind::NightDemon => 50,
            CreatureKind::BossDujie => 800,
        }
    }
    pub fn attack(self) -> i16 {
        match self {
            CreatureKind::Rabbit | CreatureKind::Deer => 0,
            CreatureKind::Wolf => 8,
            CreatureKind::NightDemon => 12,
            CreatureKind::BossDujie => 35,
        }
    }
    pub fn vision(self) -> u16 {
        match self {
            CreatureKind::BossDujie => 12,
            _ => 5,
        }
    }
    pub fn is_hostile(self) -> bool {
        matches!(
            self,
            CreatureKind::Wolf | CreatureKind::NightDemon | CreatureKind::BossDujie
        )
    }
    pub fn is_boss(self) -> bool {
        matches!(self, CreatureKind::BossDujie)
    }
    /// 死亡掉落（v1 都返回空，等 raw_meat ItemKind 加进来）
    pub fn loot(self) -> &'static [(ItemKind, u16)] {
        &[]
    }
}

pub type CreatureId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creature {
    pub id: CreatureId,
    pub kind: CreatureKind,
    pub hp: i16,
    pub pos: TileCoord,
    pub target: Option<AgentId>,
    pub last_action_tick: u64,
}

impl Creature {
    pub fn new(id: CreatureId, kind: CreatureKind, pos: TileCoord, tick: u64) -> Self {
        Self {
            id,
            kind,
            hp: kind.max_hp(),
            pos,
            target: None,
            last_action_tick: tick,
        }
    }
}
