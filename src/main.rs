use tcod::colors::*;
use tcod::console::*;
use tcod::map::{Map as FovMap};
use tcod::input::{self, Event, Key, Mouse};
use rand::Rng;

mod game_object;
mod panel;
mod menu;
mod map;
mod game;


fn main() {
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

    tcod::system::set_fps(game::LIMIT_FPS);

    let mut player = game_object::GameObject::new(0, 0, '@', "player", WHITE, true);
    player.alive = true;
    player.fighter = Some(game_object::Fighter {
        max_hp: 30,
        hp: 30,
        defense: 2,
        power: 5,
        on_death: game_object::DeathCallback::Player
    });
   
    let mut objects = vec![player];
    let mut game = game::Game {
        game_map: map::make_map(&mut objects),
        messages: panel::Messages::new(),
        inventory: vec![]
    };

    // populate FOV map, according to the generated map
    for y in 0..map::MAP_HEIGHT {
        for x in 0..map::MAP_WIDTH {
            tcod.fov.set (
                x,
                y,
                !game.game_map[x as usize][y as usize].block_sight,
                !game.game_map[x as usize][y as usize].blocked
            );
        }
    }

    // force FOV "recompute" first time through the game loop
    let mut previous_player_position = (-1,-1);

    game.messages.add("Welcom stranger! Prepare to parish in the Tobs of the Ancient Kinds.", RED);

    while !tcod.root.window_closed() {
        // clear the screen of the previous frame
        tcod.con.clear();

        for object in &objects {
            object.draw(&mut tcod.con);
        }

        let fov_recompute = previous_player_position != (objects[game_object::PLAYER].pos());

        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => tcod.key = k,
            _ => tcod.key = Default::default()
        }

        render_all(&mut tcod, &mut game, &objects, fov_recompute);

        tcod.root.flush();

        let player = &mut objects[game_object::PLAYER];
        previous_player_position = (player.x, player.y);

        let player_action = handle_keys(&mut tcod, &mut game, &mut objects);
        if player_action == game_object::PlayerAction::Exit { break; }

        // let monsters take their turn
        if objects[game_object::PLAYER].alive && player_action != game_object::PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &tcod, &mut game, &mut objects);
                }
            }
        }
    }
}

fn handle_keys(tcod: &mut game::Tcod, game: &mut game::Game, objects: &mut Vec<game_object::GameObject>) -> game_object::PlayerAction {
    use tcod::input::KeyCode::*;
    use game_object::PlayerAction::*;

    let player_alive = objects[game_object::PLAYER].alive;

    match (tcod.key, tcod.key.text(), player_alive) {
        (Key { code: Enter, alt: true, .. }, _, _) => {
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        },
        (Key { code: Escape, .. }, _, _) => Exit,
        (Key { code: Up, .. }, _, true) => {
            game_object::player_move_or_attack(0, -1, game, objects);
            TookTurn
        }
        (Key { code: Down, .. }, _, true) => {
            game_object::player_move_or_attack(0, 1, game, objects);
            TookTurn
        }
        (Key { code: Left, .. }, _, true) => {
            game_object::player_move_or_attack(-1, 0, game, objects);
            TookTurn
        }
        (Key { code: Right, .. }, _, true) => {
            game_object::player_move_or_attack(1, 0, game, objects);
            TookTurn
        }
        (Key { code: Text, .. }, "g", true) => {
            let item_id = objects
                            .iter()
                            .position(|o| o.pos() == objects[game_object::PLAYER].pos() && o.item.is_some());
            if let Some(item_id) = item_id {
                game_object::pick_item_up(item_id, game, objects);
            }
            DidntTakeTurn
        }
        (Key { code: Text, ..}, "i", true) => {
            let inventory_index = menu::inventory_menu(
                &game.inventory, 
                "Press the key next to an item to use it, or any other to cancel.\n", 
                &mut tcod.root
            );
            if let Some(inventory_index) = inventory_index {
                game_object::use_item(inventory_index, tcod, game, objects);
            };
            DidntTakeTurn
        }
        _ => DidntTakeTurn
    }
}

fn render_all(tcod: &mut game::Tcod, game: &mut game::Game, objects: &[game_object::GameObject], fov_recompute: bool) {
    if fov_recompute  {
        // recompute FOV if needed (the player moved or something)
        let player = &objects[game_object::PLAYER];
        tcod.fov.compute_fov(
            player.x, 
            player.y, 
            map::TORCH_RADIUS,
            map::FOV_LIGHT_WALLS,
            map::FOV_ALGO
        );
    }    

    for y in 0..map::MAP_HEIGHT {
        for x in 0..map::MAP_WIDTH {
            let visable = tcod.fov.is_in_fov(x, y);
            let wall = game.game_map[x as usize][y as usize].block_sight;
            let color = match(visable, wall) {
                (false, true) => map::COLOR_DARK_WALL,
                (false, false) => map::COLOR_DARK_GROUND,
                (true, true) => map::COLOR_LIGHT_WALL,
                (true, false) => map::COLOR_LIGHT_GROUND,
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
        &tcod.con,                      //  The offscreen console
        (0, 0),                         //  Starting coordinates
        (map::MAP_WIDTH, map::MAP_HEIGHT),  // size to blit
        &mut tcod.root,                 // blit destination
        (0, 0),                         // Coordinates to blit to
        1.0,                            // Forground opaque
        1.0,                            // Background opaque
    );

    // prepare to render the GUI panel
    tcod.panel.set_default_background(BLACK);
    tcod.panel.clear();

    // print game messages, one line at a time
    let mut y = panel::MSG_HEIGHT as i32;
    for &(ref msg, color) in game.messages.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(panel::MSG_X, y, panel::MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 { break; }
        tcod.panel.set_default_background(color);
        tcod.panel.print_rect(panel::MSG_X, y, panel::MSG_WIDTH, 0, msg);
    }

    // show player's stats
    let hp = objects[game_object::PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[game_object::PLAYER].fighter.map_or(0, |f| f.max_hp);
    panel::render_bar(
        &mut tcod.panel,
        1, 1,
        panel::BAR_WIDTH,
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
        (game::SCREEN_WIDTH, panel::PANEL_HEIGHT),  // size to blit
        &mut tcod.root,                 // blit destination
        (0, panel::PANEL_Y),                         // Coordinates to blit to
        1.0,                            // Forground opaque
        1.0,                            // Background opaque
    );
}

fn ai_take_turn(monster_id: usize, tcod: &game::Tcod, game: &mut game::Game, objects: &mut[game_object::GameObject]) {
    use game_object::Ai::*;
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Basic => ai_basic(monster_id, tcod, game, objects),
            Confused {
                previous_ai,
                num_turns
            } => ai_confused(monster_id, tcod, game, objects, previous_ai, num_turns)
        };
        objects[monster_id].ai = Some(new_ai);
    }
}

fn ai_basic(monster_id: usize, tcod: &game::Tcod, game: &mut game::Game, objects: &mut[game_object::GameObject]) -> game_object::Ai {
    // a basic monster takes its turn.  If you can see it, it can see you
    let(monster_x, monster_y) = objects[monster_id].pos();
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[game_object::PLAYER]) > 2.0 {
            // move towards player if far away
            let (player_x, player_y) = objects[game_object::PLAYER].pos();
            game_object::move_towards(monster_id, player_x, player_y, &game.game_map, objects);
        } else if objects[game_object::PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            // close enough, attack!
            let (monster, player) = game_object::mut_two(monster_id, game_object::PLAYER, objects);
            monster.attack(player, game);
       }
    }
    game_object::Ai::Basic
}

fn ai_confused(monster_id: usize, tcod: &game::Tcod, game: &mut game::Game, objects: &mut[game_object::GameObject], previous_ai: Box<game_object::Ai>, num_turns: i32) -> game_object::Ai {
    if num_turns >= 0 {
        // still confused
        // move a random direction and decrease confused turn count
        game_object::move_by(monster_id, 
            rand::thread_rng().gen_range(-1, 2), 
            rand::thread_rng().gen_range(-1, 2), 
            &game.game_map, 
            objects);
        game_object::Ai::Confused {previous_ai: previous_ai, num_turns: num_turns - 1}
    } else {
        *previous_ai
    }
}

fn get_names_under_mouse(mouse: Mouse, objects: &[game_object::GameObject], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);
    let names = objects
        .iter()
        .filter(|obj| obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}


