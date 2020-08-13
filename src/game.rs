use crate::map::Map;
use crate::panel::Messages;
use crate::game_object::GameObject;

use tcod::console::*;
use tcod::map::{Map as FovMap};
use tcod::input::{Key, Mouse};

// window size
pub const SCREEN_WIDTH: i32 = 80;
pub const SCREEN_HEIGHT: i32 = 50;

// 20 frames per second maximum
pub const LIMIT_FPS: i32 = 20;

pub struct Game {
    pub game_map: Map,
    pub messages: Messages,
    pub inventory: Vec<GameObject>
}

pub struct Tcod {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
    pub key: Key,
    pub mouse: Mouse
}