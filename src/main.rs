use tcod::console::*;
use tcod::map::{Map as FovMap};

mod game;
mod game_object;
mod map;
mod menu;
mod panel;
mod transition;
mod equipment;

fn main() {
    tcod::system::set_fps(game::LIMIT_FPS);

    let root = Root::initializer()
        .font("assets/arial12x12.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(game::SCREEN_WIDTH, game::SCREEN_HEIGHT)
        .title("Rust/libtcod tutorial")
        .init();

    let mut tcod = game::Tcod { 
        root, 
        con: Offscreen::new(map::MAP_WIDTH, map::MAP_HEIGHT), 
        panel: Offscreen::new(game::SCREEN_WIDTH, panel::PANEL_HEIGHT),
        fov: FovMap::new(map::MAP_WIDTH, map::MAP_HEIGHT),
        key: Default::default(),
        mouse: Default::default()
    };

    menu::main_menu(&mut tcod);
}
