use crate::map::*;
use crate::panel::*;
use crate::game_object::*;

use tcod::colors::*;
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


pub fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[GameObject], fov_recompute: bool) {
    if fov_recompute  {
        // recompute FOV if needed (the player moved or something)
        let player = &objects[PLAYER];
        tcod.fov.compute_fov(
            player.x, 
            player.y, 
            TORCH_RADIUS,
            FOV_LIGHT_WALLS,
            FOV_ALGO
        );
    }    

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visable = tcod.fov.is_in_fov(x, y);
            let wall = game.game_map[x as usize][y as usize].block_sight;
            let color = match(visable, wall) {
                (false, true) => COLOR_DARK_WALL,
                (false, false) => COLOR_DARK_GROUND,
                (true, true) => COLOR_LIGHT_WALL,
                (true, false) => COLOR_LIGHT_GROUND,
            };
                
            let explored = &mut game.game_map[x as usize][y as usize].explored;
            if visable {
                *explored = true
            }
            if *explored {
                tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }


    // draw objects in the list
    let mut to_draw: Vec<_> = objects
        .iter()
        .filter(|o| 
                tcod.fov.is_in_fov(o.x, o.y)
                    || (o.always_visible && game.game_map[o.x as usize][o.y as usize].explored)
            )
        .collect();
    to_draw.sort_by(|o1, o2|o1.blocks.cmp(&o2.blocks));
    for object in &to_draw {
        object.draw(&mut tcod.con);
    }

    blit(
        &tcod.con,                      // The offscreen console
        (0, 0),                         // Starting coordinates
        (MAP_WIDTH, MAP_HEIGHT),        // size to blit
        &mut tcod.root,                 // blit destination
        (0, 0),                         // Coordinates to blit to
        1.0,                            // Forground opaque
        1.0,                            // Background opaque
    );

    // prepare to render the GUI panel
    tcod.panel.set_default_background(BLACK);
    tcod.panel.clear();

    // print game messages, one line at a time
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.messages.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 { break; }
        tcod.panel.set_default_background(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    // show player's stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(
        &mut tcod.panel,
        1, 1,
        BAR_WIDTH,
        "HP", hp, max_hp,
        LIGHT_RED,
        DARKER_RED
    );

    // display names of objects under the mouse
    tcod.panel.set_default_background(LIGHT_GREY);
    tcod.panel.print_ex(
        1,
        0,
        BackgroundFlag::None,
        TextAlignment::Left,
        get_names_under_mouse(tcod.mouse, objects, &tcod.fov)
    );

    // blit the contents of `panel` to the root console
    blit(
        &tcod.panel,                    // The panel
        (0, 0),                         //  Starting coordinates
        (SCREEN_WIDTH, PANEL_HEIGHT),  // size to blit
        &mut tcod.root,                 // blit destination
        (0, PANEL_Y),                         // Coordinates to blit to
        1.0,                            // Forground opaque
        1.0,                            // Background opaque
    );
}


fn get_names_under_mouse(mouse: Mouse, objects: &[GameObject], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);
    let names = objects
        .iter()
        .filter(|obj| obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}
