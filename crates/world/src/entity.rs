use crate::{item::ItemStack, plant::Plant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Entity {
    Plant { plant: Plant },
    ItemDrop { stack: ItemStack, expires_at: u64 },
}
