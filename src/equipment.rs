use serde::{Deserialize, Serialize};
use crate::game_object::GameObject;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Equipment {
    pub slot: Slot,
    pub equipped: bool,
    pub max_hp_bonus: i32,
    pub power_bonus: i32,
    pub defense_bonus: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Slot {
    LeftHand,
    RightHand,
    Head
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Slot::LeftHand => write!(f, "left hand"),
            Slot::RightHand => write!(f, "right hand"),
            Slot::Head => write!(f, "head"),
        }
    }
}

pub fn get_equipped_in_slot(slot: Slot, inventory: &[GameObject]) -> Option<usize> {
    for (inventory_id, item) in inventory.iter().enumerate() {
        if item.equipment.as_ref().map_or(false, |e| e.equipped && e.slot == slot) {
            return Some(inventory_id);
        }
    }
    None
}
