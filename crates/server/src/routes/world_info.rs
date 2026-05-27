use crate::state::AppState;
use axum::{extract::State, Json};
use serde::Serialize;
use world::{
    clock::{DAYS_PER_SEASON, TICKS_PER_DAY},
    item::INVENTORY_SIZE,
    observation::VISION_RADIUS,
    recipe::{recipes, CraftStation, Recipe, RecipeOutput},
    ItemKind,
};

#[derive(Serialize)]
pub struct WorldInfo {
    pub clock: ClockView,
    pub constants: Constants,
    pub recipes: Vec<RecipeView>,
    pub items: Vec<ItemView>,
}

#[derive(Serialize)]
pub struct ClockView {
    pub tick: u64,
    pub day: u32,
    pub season: world::Season,
    pub phase: world::DayPhase,
    pub tick_in_day: u32,
}

#[derive(Serialize)]
pub struct Constants {
    pub vision_radius: u16,
    /// 攻击 / 采集 / 放置 / 拾取 都受这个 manhattan 距离限制
    pub interaction_range: u16,
    pub inventory_slots: usize,
    pub stack_size: u16,
    pub max_hp: i16,
    pub max_hunger: i16,
    pub max_stamina: i16,
    /// hunger 每 N tick -1
    pub hunger_decay_period_ticks: u64,
    /// stamina 每 N tick +2
    pub stamina_regen_period_ticks: u64,
    /// hunger==0 时每 N tick hp -1
    pub starvation_hp_loss_period_ticks: u64,
    pub ticks_per_day: u32,
    pub days_per_season: u32,
    /// 武器伤害（None 即赤手）
    pub weapon_damage: Vec<WeaponDamage>,
}

#[derive(Serialize)]
pub struct WeaponDamage {
    pub item: Option<ItemKind>,
    pub damage: i16,
}

#[derive(Serialize)]
pub struct RecipeView {
    pub id: &'static str,
    pub inputs: Vec<ItemStackView>,
    pub output: ItemStackView,
    pub station: CraftStation,
}

#[derive(Serialize)]
pub struct ItemStackView {
    pub item: ItemKind,
    pub n: u16,
}

#[derive(Serialize)]
pub struct ItemView {
    pub id: ItemKind,
    pub name_zh: &'static str,
    pub is_food: bool,
    pub nutrition: Option<NutritionView>,
    pub stack_size: u16,
}

#[derive(Serialize)]
pub struct NutritionView {
    pub hunger: i16,
    pub hp: i16,
}

fn make_recipe(r: &Recipe) -> RecipeView {
    let inputs = r
        .inputs
        .iter()
        .map(|(item, n)| ItemStackView { item: *item, n: *n })
        .collect();
    let output = match r.output {
        RecipeOutput::Item(item, n) => ItemStackView { item, n },
    };
    RecipeView {
        id: r.id,
        inputs,
        output,
        station: r.station,
    }
}

fn make_item(k: ItemKind) -> ItemView {
    let nutrition = if k.is_food() {
        let (h, hp) = k.nutrition();
        Some(NutritionView { hunger: h, hp })
    } else {
        None
    };
    ItemView {
        id: k,
        name_zh: k.name_zh(),
        is_food: k.is_food(),
        nutrition,
        stack_size: k.stack_size(),
    }
}

pub async fn world_info(State(s): State<AppState>) -> Json<WorldInfo> {
    let w = s.world.lock().await;
    let clock = ClockView {
        tick: w.clock.tick,
        day: w.clock.tick as u32 / TICKS_PER_DAY,
        season: w.clock.season(),
        phase: w.clock.phase(),
        tick_in_day: w.clock.tick_in_day(),
    };
    drop(w);

    let constants = Constants {
        vision_radius: VISION_RADIUS,
        interaction_range: 1,
        inventory_slots: INVENTORY_SIZE,
        stack_size: 20,
        max_hp: 100,
        max_hunger: 100,
        max_stamina: 100,
        hunger_decay_period_ticks: 4,
        stamina_regen_period_ticks: 8,
        starvation_hp_loss_period_ticks: 2,
        ticks_per_day: TICKS_PER_DAY,
        days_per_season: DAYS_PER_SEASON,
        weapon_damage: vec![
            WeaponDamage { item: None, damage: world::combat::weapon_damage(None) },
            WeaponDamage {
                item: Some(ItemKind::BambooSpear),
                damage: world::combat::weapon_damage(Some(ItemKind::BambooSpear)),
            },
            WeaponDamage {
                item: Some(ItemKind::StoneAxe),
                damage: world::combat::weapon_damage(Some(ItemKind::StoneAxe)),
            },
        ],
    };

    let recipes = recipes().iter().map(make_recipe).collect();
    let items = ItemKind::all().iter().copied().map(make_item).collect();

    Json(WorldInfo {
        clock,
        constants,
        recipes,
        items,
    })
}
