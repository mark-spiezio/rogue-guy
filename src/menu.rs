use crate::game::*;
use crate::game_object::GameObject;
use tcod::colors::*;
use tcod::console::*;

const INVENTORY_MENU_WIDTH: i32 = 50;
const LEVEL_SCREEN_WIDTH: i32 = 40;
const CHARACTER_SCREEN_WIDTH: i32 = 30;

fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
    assert!(options.len() <= 26,
        "Cannot have a menu with more than 26 options."
    );

    // calculate total height for the header (after auto-wrap) and one line per option
    let header_height = if header.is_empty() {
        0
    } else {
        root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header)
    };
    let height = options.len() as i32 + header_height;

    let mut window = Offscreen::new(width, height);

    // print the header, with auto-wrap
    window.set_default_foreground(WHITE);
    window.print_rect_ex(0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, header);

    // print all the options
    for(index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(
            0,
            header_height + index as i32,
            BackgroundFlag::None,
            TextAlignment::Left,
            text
        );
    }

    // blit the contents of "window" to the root console
    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    blit(&window, (0,0), (width, height), root, (x, y), 1.0, 0.7);
    root.flush();
    let key = root.wait_for_keypress(true);

    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn main_menu(tcod: &mut Tcod) {
    let img = tcod::image::Image::from_file("assets/menu_background.png")
        .ok()
        .expect("Background image not found");

    while !tcod.root.window_closed() {
        tcod::image::blit_2x(&img, (0,0), (-1,-1), &mut tcod.root, (0,0));

        tcod.root.set_default_foreground(LIGHT_YELLOW);
        tcod.root.print_ex(
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT / 2 - 4,
            BackgroundFlag::None,
            TextAlignment::Center,
            "ROGUE GUY - TOMBS OF THE ANCIENT KINGS",
        );
        tcod.root.print_ex(
            SCREEN_WIDTH / 2, 
            SCREEN_HEIGHT -2, 
            BackgroundFlag::None, 
            TextAlignment::Center,
            "By Yours Truely"
        );

        // show options and wait for the player's choice
        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut tcod.root);

        match choice {
            Some(0) => {
                // new game
                let (mut game, mut objects) = new_game(tcod);
                play_game(tcod, &mut game, &mut objects);
            }
            Some(1) => {
                match load_game() {
                    Ok((mut game, mut objects)) => {
                        initialize_fov(tcod, &game.game_map);
                        play_game(tcod, &mut game, &mut objects);
                    }
                    Err(_e) => {
                        msgbox("\nNo saved game to load.\n", 24, &mut tcod.root);
                        continue;
                    }
                }
                

            }
            Some(2) => {
                break; // quit
            }
            _ => {}
        }
    }
}

pub fn inventory_menu(inventory: &[GameObject], header: &str, root: &mut Root) -> Option<usize> {
    // how a menu with each item of the inventory as an option
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| item.name.clone()).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_MENU_WIDTH, root);

    // if an item was chosen, return it
    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

pub fn level_up_menu(player: &mut GameObject, root: &mut Root) -> Option<usize> {
    let fighter = player.fighter.as_mut().unwrap();
    let mut choice = None;
    while choice.is_none() {
        choice = menu(
            "Level up! Choose a stat to raise:\n",
            &[
                format!("Constitution (+20 HP, from {})", fighter.max_hp),
                format!("Strength (+1 attack, from {})", fighter.power),
                format!("Agility (+1 defense, from {})", fighter.defense),
            ],
            LEVEL_SCREEN_WIDTH,
            root
        );
    }
    choice
}

pub fn character_information_msgbox(player: &GameObject, base: i32, factor: i32, root: &mut Root) {
    let level = player.level;
    let level_up_xp = base + player.level * factor;
    if let Some(fighter) = player.fighter.as_ref() {
        let msg = format!(
            "Character information
            
Level: {}
Experience: {}
Experience to level up: {}

Maximum HP: {}
Attack: {}
Defense: {}",
            level, fighter.xp, level_up_xp, fighter.max_hp, fighter.power, fighter.defense
        );
        msgbox(&msg, CHARACTER_SCREEN_WIDTH, root);
    }
}