use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use serde::{Deserialize, Serialize};

use tcod::colors::*;
use tcod::console::*;
use tcod::input::*;
use tcod::map::Map as FovMap;

use crate::map::*;
use crate::panel::*;
use crate::game_object::*;
use crate::menu::*;

// window size
pub const SCREEN_WIDTH: i32 = 80;
pub const SCREEN_HEIGHT: i32 = 50;

// 20 frames per second maximum
pub const LIMIT_FPS: i32 = 20;

#[derive(Serialize, Deserialize)]
pub struct Game {
    pub game_map: Map,
    pub messages: Messages,
    pub inventory: Vec<GameObject>,
    pub dungeon_level: u32,
}

pub struct Tcod {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
    pub key: Key,
    pub mouse: Mouse,
}

pub fn new_game(tcod: &mut Tcod) -> (Game, Vec<GameObject>) {
    // create player object
    let mut player = GameObject::new(0, 0, '@', "player", WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter {
        max_hp: 100,
        hp: 100,
        defense: 1,
        power: 4,
        xp: 0,
        on_death: DeathCallback::Player,
    });

    let mut objects = vec![player];

    let mut game = Game {
        game_map: make_map(&mut objects, 1),
        messages: Messages::new(),
        inventory: vec![],
        dungeon_level: 1
    };

    initialize_fov(tcod, &game.game_map);

    game.messages.add(
        "Welcom stranger! Prepare to parish in the Tombs of the Ancient Kinds.",
        RED,
    );

    (game, objects)
}

fn save_game(game: &Game, objects: &[GameObject]) -> Result<(), Box<dyn Error>> {
    let save_data = serde_json::to_string(&(game, objects))?;
    let mut file = File::create("savegame")?;
    file.write_all(save_data.as_bytes())?;
    Ok(())
}

pub fn load_game() -> Result<(Game, Vec<GameObject>), Box<dyn Error>> {
    let mut json_save_state = String::new();
    let mut file = File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<(Game, Vec<GameObject>)>(&json_save_state)?;
    Ok(result)
}

pub fn initialize_fov(tcod: &mut Tcod, map: &Map) {
    // populate FOV map, according to the generated map
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x,
                y,
                !map[x as usize][y as usize].block_sight,
                !map[x as usize][y as usize].blocked,
            );
        }
    }

    // unexplored areas start black (default background color)
    tcod.con.clear();
}

pub fn play_game(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<GameObject>) {
    // force FOV "recompute" first time through the game loop
    let mut previous_player_position = (-1, -1);

    while !tcod.root.window_closed() {
        // clear the screen of the previous frame
        tcod.con.clear();

        match check_for_event(MOUSE | KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => tcod.key = k,
            _ => tcod.key = Default::default(),
        }

        let fov_recompute = previous_player_position != (objects[PLAYER].pos());
        render_all(tcod, game, &objects, fov_recompute);

        tcod.root.flush();

        // level up if needed
        level_up(tcod, game, objects);

        // handle keys and exit game if needed
        previous_player_position = objects[PLAYER].pos();
        let player_action = handle_keys(tcod, game, objects);
        if player_action == PlayerAction::Exit {
            save_game(game, objects).unwrap();
            break;
        }

        // let monsters take their turn
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, tcod, game, objects);
                }
            }
        }
    }
}

fn handle_keys(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<GameObject>) -> PlayerAction {
    use tcod::input::KeyCode::*;
    use PlayerAction::*;
    let player_alive = objects[PLAYER].alive;

    match (tcod.key, tcod.key.text(), player_alive) {
        (
            Key {
                code: Enter,
                alt: true,
                ..
            },
            _,
            _,
        ) => {
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }
        (Key { code: Escape, .. }, _, _) => Exit,
        // movement keys
        (Key { code: Up, .. }, _, true) | (Key { code: NumPad8, .. }, _, true) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        }
        (Key { code: Down, .. }, _, true) | (Key { code: NumPad2, .. }, _, true) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        }
        (Key { code: Left, .. }, _, true) | (Key { code: NumPad4, .. }, _, true) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        }
        (Key { code: Right, .. }, _, true) | (Key { code: NumPad6, .. }, _, true) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        }
        (Key { code: Home, .. }, _, true) | (Key { code: NumPad7, .. }, _, true) => {
            player_move_or_attack(-1, -1, game, objects);
            TookTurn
        }
        (Key { code: PageUp, .. }, _, true) | (Key { code: NumPad9, .. }, _, true) => {
            player_move_or_attack(1, -1, game, objects);
            TookTurn
        }
        (Key { code: End, .. }, _, true) | (Key { code: NumPad1, .. }, _, true) => {
            player_move_or_attack(-1, 1, game, objects);
            TookTurn
        }
        (Key { code: PageDown, .. }, _, true) | (Key { code: NumPad3, .. }, _, true) => {
            player_move_or_attack(1, 1, game, objects);
            TookTurn
        }
        (Key { code: NumPad5, .. }, _, true) => {
            TookTurn // do nothing, i.e. wait for the monster to come to you
        }
        // "get" - pick up item
        (Key { code: Text, .. }, "g", true) => {
            let item_id = objects
                .iter()
                .position(|o| o.pos() == objects[PLAYER].pos() && o.item.is_some());
            if let Some(item_id) = item_id {
                pick_item_up(item_id, game, objects);
            }
            DidntTakeTurn
        }
        // "drop" - drop item
        (Key { code: Text, .. }, "d", true) => {
            //show the inventory; if an item is selected, drop it
            let inventory_index = inventory_menu(&game.inventory, "", &mut tcod.root);
            if let Some(inventory_index) = inventory_index {
                drop_item(inventory_index, game, objects);
            }
            DidntTakeTurn
        }
        // view inventory
        (Key { code: Text, .. }, "i", true) => {
            let inventory_index = inventory_menu(
                &game.inventory,
                "Press the key next to an item to use it, or any other to cancel.\n",
                &mut tcod.root,
            );
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, tcod, game, objects);
            };
            DidntTakeTurn
        }
        // take stairs
        (Key {code: Text, ..}, "<", true) => {
            let player_on_stairs = objects
                .iter()
                .any(|object| 
                    object.pos() == objects[PLAYER].pos() 
                    && object.name == "stairs");
            if player_on_stairs {
                next_level(tcod, game, objects);
            }
            DidntTakeTurn
        }
        // view character information
        (Key {code: Text, ..}, "c", true) => {
            character_information_msgbox(&objects[PLAYER], LEVEL_UP_BASE, LEVEL_UP_FACTOR, &mut tcod.root);
            DidntTakeTurn
        }
        _ => DidntTakeTurn,
    }
}

fn next_level(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<GameObject>) {
    game.messages.add(
        "You take a moment to rest, and recover your strength.", 
        VIOLET
    );
    let heal_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp / 2);
    objects[PLAYER].heal(heal_hp);

    game.messages.add(
        "After a rare moment of peace, you descend deeper into \
        the heart of the dundeon...", 
        RED
    );  
    game.dungeon_level += 1;
    game.game_map = make_map(objects, game.dungeon_level);
    initialize_fov(tcod, &game.game_map);
}

pub fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[GameObject], fov_recompute: bool) {
    if fov_recompute {
        // recompute FOV if needed (the player moved or something)
        let player = &objects[PLAYER];
        tcod.fov
            .compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visable = tcod.fov.is_in_fov(x, y);
            let wall = game.game_map[x as usize][y as usize].block_sight;
            let color = match (visable, wall) {
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
                tcod.con
                    .set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }

    // draw objects in the list
    let mut to_draw: Vec<_> = objects
        .iter()
        .filter(|o| {
            tcod.fov.is_in_fov(o.x, o.y)
                || (o.always_visible && game.game_map[o.x as usize][o.y as usize].explored)
        })
        .collect();
    to_draw.sort_by(|o1, o2| o1.blocks.cmp(&o2.blocks));
    for object in &to_draw {
        object.draw(&mut tcod.con);
    }

    blit(
        &tcod.con,               // The offscreen console
        (0, 0),                  // Starting coordinates
        (MAP_WIDTH, MAP_HEIGHT), // size to blit
        &mut tcod.root,          // blit destination
        (0, 0),                  // Coordinates to blit to
        1.0,                     // Forground opaque
        1.0,                     // Background opaque
    );

    // prepare to render the GUI panel
    tcod.panel.set_default_background(BLACK);
    tcod.panel.clear();

    // print game messages, one line at a time
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.messages.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_background(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    // show player's stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(
        &mut tcod.panel,
        1,
        1,
        BAR_WIDTH,
        "HP",
        hp,
        max_hp,
        LIGHT_RED,
        DARKER_RED,
    );

    // display dungeon level
    tcod.panel.print_ex(
        1, 3, // (pos x,y)
        BackgroundFlag::None, 
        TextAlignment::Left,
        format!("Dungeon level: {}", game.dungeon_level)
    );

    // display names of objects under the mouse
    tcod.panel.set_default_background(LIGHT_GREY);
    tcod.panel.print_ex(
        1,
        0,
        BackgroundFlag::None,
        TextAlignment::Left,
        get_names_under_mouse(tcod.mouse, objects, &tcod.fov),
    );

    // blit the contents of `panel` to the root console
    blit(
        &tcod.panel,                  // The panel
        (0, 0),                       //  Starting coordinates
        (SCREEN_WIDTH, PANEL_HEIGHT), // size to blit
        &mut tcod.root,               // blit destination
        (0, PANEL_Y),                 // Coordinates to blit to
        1.0,                          // Forground opaque
        1.0,                          // Background opaque
    );
}

fn ai_take_turn(
    monster_id: usize,
    tcod: &Tcod,
    game: &mut Game,
    objects: &mut [GameObject],
) {
    use Ai::*;
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Basic => ai_basic(monster_id, tcod, game, objects),
            Confused {
                previous_ai,
                num_turns,
            } => ai_confused(monster_id, tcod, game, objects, previous_ai, num_turns),
        };
        objects[monster_id].ai = Some(new_ai);
    }
}

fn ai_basic(monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [GameObject]) -> Ai {
    // a basic monster takes its turn.  If you can see it, it can see you
    let (monster_x, monster_y) = objects[monster_id].pos();
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) > 2.0 {
            // move towards player if far away
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &game.game_map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            // close enough, attack!
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, game);
        }
    }
    Ai::Basic
}

fn ai_confused(
    monster_id: usize,
    _tcod: &Tcod,
    game: &mut Game,
    objects: &mut [GameObject],
    previous_ai: Box<Ai>,
    num_turns: i32,
) -> Ai {
    use rand::Rng;

    if num_turns >= 0 {
        // still confused
        // move a random direction and decrease confused turn count
        move_by(
            monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            &game.game_map,
            objects,
        );
        Ai::Confused {
            previous_ai: previous_ai,
            num_turns: num_turns - 1,
        }
    } else {
        *previous_ai
    }
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
