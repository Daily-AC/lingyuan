use crate::{building::BuildingKind, item::ItemKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CraftStation {
    Hand,
    Campfire,
    CookingStove,
}

#[derive(Debug, Clone, Copy)]
pub struct Recipe {
    pub id: &'static str,
    pub inputs: &'static [(ItemKind, u16)],
    pub output: RecipeOutput,
    pub station: CraftStation,
}

#[derive(Debug, Clone, Copy)]
pub enum RecipeOutput {
    Item(ItemKind, u16),
}

pub fn recipes() -> &'static [Recipe] {
    &[
        Recipe {
            id: "bamboo_spear",
            inputs: &[(ItemKind::Flint, 1), (ItemKind::Bamboo, 1)],
            output: RecipeOutput::Item(ItemKind::BambooSpear, 1),
            station: CraftStation::Hand,
        },
        Recipe {
            id: "rope",
            inputs: &[(ItemKind::Vine, 2)],
            output: RecipeOutput::Item(ItemKind::Rope, 1),
            station: CraftStation::Hand,
        },
        Recipe {
            id: "clay_pot",
            inputs: &[(ItemKind::Reed, 3), (ItemKind::Clay, 1)],
            output: RecipeOutput::Item(ItemKind::ClayPot, 1),
            station: CraftStation::Hand,
        },
        Recipe {
            id: "campfire_kit",
            inputs: &[(ItemKind::Pinewood, 3), (ItemKind::Flint, 1)],
            output: RecipeOutput::Item(ItemKind::CampfireKit, 1),
            station: CraftStation::Hand,
        },
        Recipe {
            id: "cooking_stove_kit",
            inputs: &[(ItemKind::Stone, 5), (ItemKind::Clay, 3)],
            output: RecipeOutput::Item(ItemKind::CookingStoveKit, 1),
            station: CraftStation::Hand,
        },
        Recipe {
            id: "stone_axe",
            inputs: &[
                (ItemKind::Stone, 3),
                (ItemKind::Pinewood, 1),
                (ItemKind::Rope, 1),
            ],
            output: RecipeOutput::Item(ItemKind::StoneAxe, 1),
            station: CraftStation::CookingStove,
        },
        Recipe {
            id: "cook_mushroom",
            inputs: &[(ItemKind::Mushroom, 1)],
            output: RecipeOutput::Item(ItemKind::CookedMushroom, 1),
            station: CraftStation::Campfire,
        },
        Recipe {
            id: "cook_berry",
            inputs: &[(ItemKind::RedBerry, 1)],
            output: RecipeOutput::Item(ItemKind::CookedBerry, 1),
            station: CraftStation::Campfire,
        },
        Recipe {
            id: "rice_cake",
            inputs: &[(ItemKind::Reed, 2), (ItemKind::Mushroom, 1)],
            output: RecipeOutput::Item(ItemKind::RiceCake, 1),
            station: CraftStation::CookingStove,
        },
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
