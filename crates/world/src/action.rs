use crate::coord::Direction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Action {
    Move { dir: Direction },
    #[default]
    Wait,
    Observe,
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
}
