use crate::ItemKind;

pub fn weapon_damage(weapon: Option<ItemKind>) -> i16 {
    match weapon {
        Some(ItemKind::BambooSpear) => 8,
        Some(ItemKind::StoneAxe) => 10,
        _ => 3,
    }
}

pub fn resolve_attack_damage(weapon: Option<ItemKind>) -> i16 {
    weapon_damage(weapon)
}
