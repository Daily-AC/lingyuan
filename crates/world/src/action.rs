use crate::{agent::AgentId, coord::Direction, coord::TileCoord, item::ItemKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "target_kind", content = "target_id", rename_all = "snake_case")]
pub enum AttackTarget {
    Agent(AgentId),
    Creature(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Action {
    Move { dir: Direction },
    #[default]
    Wait,
    Observe,
    Gather { target: TileCoord },
    Eat { item: ItemKind },
    Craft { recipe: String },
    Place { item: ItemKind, pos: TileCoord },
    PickUp { pos: TileCoord },
    Drop { item: ItemKind, n: u16 },
    Attack { target: AttackTarget },
}

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
        let a: Action =
            serde_json::from_str(r#"{"kind":"move","data":{"dir":"north"}}"#).unwrap();
        assert_eq!(
            a,
            Action::Move {
                dir: Direction::North
            }
        );
    }

    #[test]
    fn deserialize_gather() {
        let a: Action =
            serde_json::from_str(r#"{"kind":"gather","data":{"target":{"x":3,"y":4}}}"#).unwrap();
        assert_eq!(
            a,
            Action::Gather {
                target: TileCoord::new(3, 4)
            }
        );
    }

    #[test]
    fn deserialize_craft() {
        let a: Action =
            serde_json::from_str(r#"{"kind":"craft","data":{"recipe":"bamboo_spear"}}"#).unwrap();
        assert_eq!(
            a,
            Action::Craft {
                recipe: "bamboo_spear".into()
            }
        );
    }
}
