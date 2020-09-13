use std::cmp;
use serde::{Deserialize, Serialize};
use tcod::colors::*;
use tcod::console::*;
use crate::game::*;
use crate::map::*;
use crate::panel::Messages;
use crate::menu::level_up_menu;
use crate::equipment::*;

pub const PLAYER: usize = 0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fighter {
    pub hp: i32,
    pub base_max_hp: i32,
    pub base_defense: i32,
    pub base_power: i32,
    pub xp: i32,
    pub on_death: DeathCallback
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ai {
    Basic,
    Confused {
        previous_ai: Box<Ai>,
        num_turns: i32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball,
    Sword,
    Shield
}

const HEAL_AMOUNT: i32 = 40;
const LIGHTNING_DAMAGE: i32 = 40;
const LIGHTNING_RANGE: i32 = 5;
const CONFUSE_NUM_TURNS: i32 = 10;
const CONFUSE_RANGE: i32 = 8;
const FIREBALL_RADIUS: i32 = 3;
const FIREBALL_DAMAGE: i32 = 25;

pub const LEVEL_UP_BASE: i32 = 200;
pub const LEVEL_UP_FACTOR: i32 = 150;

#[derive(Debug, Serialize, Deserialize)]
pub struct GameObject {
    pub x: i32,
    pub y: i32,
    pub glyph: char,
    pub name: String,
    pub color: Color,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
    pub item: Option<Item>,
    pub equipment: Option<Equipment>,
    pub always_visible: bool,
    pub level: i32,
}

impl GameObject {
    pub fn new(x: i32, y: i32, glyph: char, name: &str, color: Color, blocks: bool) -> Self {
        GameObject { 
            x: x, 
            y: y, 
            glyph: glyph, 
            name: name.into(),
            color: color,  
            blocks: blocks, 
            alive: false,
            fighter: None,
            ai: None,
            item: None,
            equipment: None,
            always_visible: false,
            level: 1,
        }
    }

    // set the color and then draw the character that represents this object at its position
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.glyph, BackgroundFlag::None);
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance(&self, x: i32, y: i32) -> f32 {
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }

    pub fn distance_to(&self, other: &GameObject) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32, game: &mut Game) -> Option<i32> {
        // apply damage if possible
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
        // check for death, call the death function
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
                return Some(fighter.xp);
            }
        }
        None
    }

    pub fn attack(&mut self, target: &mut GameObject, game: &mut Game) {
        // a simple formulat for attack damage
        let damage = self.power(game) - target.defense(game);
        if damage > 0 {
            game.messages.add(format!("{} attacks {} for {} hit points.", self.name, target.name, damage), WHITE);
            if let Some(xp) = target.take_damage(damage, game) {
                self.fighter.as_mut().unwrap().xp += xp;
            }
        } else {
            game.messages.add(format!("{} attacks {} but it has no effect!", self.name, target.name), WHITE);
        }
    }

    // heal by the given amount, without going over the maximum
    pub fn heal(&mut self, amount: i32, game: &Game) {
        let max_hp = self.max_hp(game);
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > max_hp {
                fighter.hp = max_hp;
            }
        }
    }

    pub fn equip(&mut self, messages: &mut Messages) {
        if self.item.is_none() {
            messages.add(
                format!("Can't equip {:?} because it's not an Item.", self),
                RED
            );
            return;
        };
        if let Some(ref mut equipment) = self.equipment {
            if !equipment.equipped {
                equipment.equipped = true;
                messages.add(
                    format!("Equipped {} on {}.", self.name, equipment.slot),
                    LIGHT_GREEN
                );  
            }
        } else {
            messages.add(
                format!("Can't equip {:?} because it's not an Equipment.", self),
                RED
            );
        };
    }

    pub fn dequip(&mut self, messages: &mut Messages) {
        if self.item.is_none() {
            messages.add(
                format!("Can't dequip {:?} because it's not an Item.", self),
                RED
            );
            return;
        };
        if let Some(ref mut equipment) = self.equipment {
            if !equipment.equipped {
                equipment.equipped = false;
                messages.add(
                    format!("Dequipped {} on {}.", self.name, equipment.slot),
                    LIGHT_YELLOW
                );  
            }
        } else {
            messages.add(
                format!("Can't dequip {:?} because it's not an Equipment.", self),
                RED
            );
        };
    }

    pub fn get_all_equipped(&self, game: &Game) -> Vec<Equipment> {
        if self.name == "player" {
            game.inventory
                .iter()
                .filter(|item| item.equipment.map_or(false, |e| e.equipped))
                .map(|item| item.equipment.unwrap())
                .collect()
        } else {
            vec![]  // other objecsts have no equipment
        }
    }

    pub fn power(&self, game: &Game) -> i32 {
        let base_power = self.fighter.map_or(0, |f| f.base_power);
        let bonus: i32 = self.get_all_equipped(game)
            .iter()
            .map(|e| e.power_bonus)
            .sum();
        base_power + bonus
    }

    pub fn defense(&self, game: &Game) -> i32 {
        let base_defense = self.fighter.map_or(0, |f| f.base_defense);
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.defense_bonus)
            .sum();
        base_defense + bonus
    }

    pub fn max_hp(&self, game: &Game) -> i32 {
        let base_max_hp = self.fighter.map_or(0,|f| f.base_max_hp);
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.max_hp_bonus)
            .sum();
            base_max_hp + bonus
    }
}


// move by the given amount
pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [GameObject]) {
    let (x,y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

pub fn is_blocked(x: i32, y: i32, map: &Map, objects: &[GameObject]) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }
    objects
        .iter()
        .any(|object| object.blocks && object.pos() == (x, y))
}

pub fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game, objects: &mut [GameObject]) {
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    let target_id = objects
        .iter()
        .position(|object| object.fighter.is_some() && object.pos() == (x, y));

    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
            player.attack(target, game);
        }
        None => {
            move_by(PLAYER, dx, dy, &game.game_map, objects);
        }
    }
}

pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [GameObject]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

pub fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    // panic at the disco, you can't mutable borrow an object more than once
    assert!(first_index != second_index); 

    let split_at_index= cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeathCallback {
    Player,
    Monster
}

impl DeathCallback {
    fn callback(self, object: &mut GameObject, game: &mut Game) {
        use DeathCallback::*;
        let callback = match self {
            Player => player_death,
            Monster => monster_death
        };
        callback(object, game);
    }
}

fn player_death(player: &mut GameObject, game: &mut Game) {
    game.messages.add("You died!", RED);

    player.glyph = '%';
    player.color = DARK_RED;
}

fn monster_death(monster: &mut GameObject, game: &mut Game) {
    game.messages.add(format!("{} Is dead!", monster.name), ORANGE);

    monster.glyph = '%';
    monster.color = DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

pub fn pick_item_up(object_id: usize, game: &mut Game, objects: &mut Vec<GameObject>) {
    if game.inventory.len() >= 26 {
        game.messages.add(format!("Your inventory is full, cannot pick up {}.", objects[object_id].name), RED);
    } else {
        let item = objects.swap_remove(object_id);
        game.messages
            .add(format!("You picked up a {}!", item.name), GREEN);
        let index = game.inventory.len();
        let slot = item.equipment.map(|e| e.slot);
        game.inventory.push(item);

        // automatically equip if teh corresponding equipment slot is unused
        if let Some(slot) = slot {
            if get_equipped_in_slot(slot, &game.inventory).is_none() {
                game.inventory[index].equip(&mut game.messages);
            }
        }
    }
}

pub fn level_up(tcod: &mut Tcod, game: &mut Game, objects: &mut [GameObject]) {
    let player = &mut objects[PLAYER];
    let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
    if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
        player.level += 1;
        game.messages.add(format!(
            "Your battle skills grow stronger!  You reached level {}!",
            player.level
        ), YELLOW);

        let choice = level_up_menu(player, &mut tcod.root);
        let fighter = player.fighter.as_mut().unwrap();
        fighter.xp -= level_up_xp;
        match choice.unwrap() {
            0 => {
                fighter.base_max_hp += 20;
                fighter.hp += 20;
            }
            1 => {
                fighter.base_power += 1;
            }
            2 => {
                fighter.base_defense += 1;
            }
            _ => unreachable!()
        }
    }
}

enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

pub fn use_item(inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [GameObject]) {
    use Item::*;
    // just call the "use_function" if it is defined
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball,
            Sword => toggle_equipment,
            Shield => toggle_equipment
        };
        match on_use(inventory_id, tcod, game, objects) {
            UseResult::UsedUp => {
                // destroy after use, unless it was cancelled for some reason
                game.inventory.remove(inventory_id);
            }
            UseResult::UsedAndKept => {} // nop
            UseResult::Cancelled => {
                game.messages.add("Cancelled", WHITE);
            }
        }
    } else {
        game.messages.add(
            format!("The {} cannot be used.", game.inventory[inventory_id].name),
            WHITE,
        );
    }
}

pub fn drop_item(inventory_id: usize, game: &mut Game, objects: &mut Vec<GameObject>) {
    let mut item = game.inventory.remove(inventory_id);
    if item.equipment.is_some() {
        item.dequip(&mut game.messages);
    }

    item.set_pos(objects[PLAYER].x, objects[PLAYER].y);
    game.messages
        .add(format!("You dropped a {}.", item.name), YELLOW);
    objects.push(item);
}

fn cast_heal(
    _inventory_id: usize,
    _tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [GameObject],
) -> UseResult {
    // heal the player
    let player = &mut objects[PLAYER];
    if let Some(fighter) = player.fighter {
        if fighter.hp == player.max_hp(game) {
            game.messages.add("You are already at full health.", RED);
            return UseResult::Cancelled;
        }
        game.messages
            .add("Your wounds start to feel better!", LIGHT_VIOLET);
        objects[PLAYER].heal(HEAL_AMOUNT, game);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn cast_lightning(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [GameObject],
) -> UseResult {

    // find closest enemy withing range and damage it
    let monster_id = closest_monster(tcod, objects, LIGHTNING_RANGE);
    if let Some(monster_id) = monster_id {
        game.messages.add(
            format!(
                "A lightning bolt strikes the {} with a loud thunger! \
                 The damage is {} hit points.",
                 objects[monster_id].name, LIGHTNING_DAMAGE
            ),
            LIGHT_BLUE
        );
        if let Some(xp) = objects[monster_id].take_damage(LIGHTNING_DAMAGE, game) {
            objects[PLAYER].fighter.as_mut().unwrap().xp += xp;
        }
        UseResult::UsedUp
    } else {
        game.messages.add("No enemy is close enough to strike.", RED);
        UseResult::Cancelled
    }
}

fn cast_confuse(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [GameObject],
) -> UseResult {
    // ask the player for a target to confuse
    game.messages.add(
        "Left-click an enemy to confuse it, or right-click to cancel.",
        LIGHT_CYAN,
    );
    // find closest enemy and confuse it
    let monster_id = target_monster(tcod, game, objects, Some(CONFUSE_RANGE as f32));
    if let Some(monster_id) = monster_id {
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);

        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS
        });
        game.messages.add(
            format!(
                "The eyes of {} look vacant, as he starts to stumble around!",
                 objects[monster_id].name
            ),
            LIGHT_GREEN
        );
        UseResult::UsedUp
    } else {
        game.messages.add("No enemy is close enough to strike.", RED);
        UseResult::Cancelled
    }
}


fn cast_fireball(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [GameObject],
) -> UseResult {
    game.messages.add(
        "Left-click a target tile for the fireball, or right-click to cancel.", 
        LIGHT_CYAN);
    let (x,y) = match target_tile(tcod, game, objects, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled
    };
    game.messages.add(
        format!(
            "The fireball explodes, burning everything within {} tiles!",
            FIREBALL_RADIUS
        ), ORANGE
    );

    let mut xp_to_gain = 0;
    for (id, obj) in objects.iter_mut().enumerate() {
        if obj.distance(x,y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            game.messages.add(
                format!(
                    "The {} gets burned for {} hit points.", 
                    obj.name, FIREBALL_DAMAGE
                ),
                ORANGE
            );
            if let Some(xp) = obj.take_damage(FIREBALL_DAMAGE, game) {
                if id != PLAYER { 
                    xp_to_gain += xp; 
                }
            }
        }
    }
    objects[PLAYER].fighter.as_mut().unwrap().xp += xp_to_gain;

    UseResult::UsedUp
}

fn closest_monster(tcod: &Tcod, objects: &[GameObject], max_range: i32) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32;

    // loop through all of the objects
    // if they are a fighter and in fov return the closest one
    for(id, object) in objects.iter().enumerate() {
        if (id != PLAYER)
            && object.fighter.is_some()
            && object.ai.is_some()
            && tcod.fov.is_in_fov(object.x, object.y)
        {
            let dist = objects[PLAYER].distance_to(object);
            if dist < closest_dist {
                closest_enemy = Some(id);
                closest_dist = dist;
            }
        }
    }
    closest_enemy
}

pub fn target_tile(
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &[GameObject],
    max_range: Option<f32>
) -> Option<(i32,i32)> {
    use tcod::input::KeyCode::Escape;
    use tcod::input::{self, Event};

    loop {
        tcod.root.flush();
        let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e|e.1);
        match event {
            Some(Event::Mouse(m)) => tcod.mouse = m,
            Some(Event::Key(k)) => tcod.key = k,
            None => tcod.key = Default::default()
        }

        render_all(tcod, game, objects, false);

        let (x,y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);

        // accept the target if the player clicked in FOV, and in case a range
        // is specified, if it's in that range
        let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x,y);
        let in_range = max_range.map_or(true, |range| objects[PLAYER].distance(x,y) <= range);
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x,y));
        }

        if tcod.mouse.rbutton_pressed || tcod.key.code == Escape {
            return None;
        }
    }
}

fn target_monster(tcod: &mut Tcod, game: &mut Game, objects: &[GameObject], max_range: Option<f32>) -> Option<usize> {
    loop {
        match target_tile(tcod, game, objects, max_range) {
            Some((x,y)) => {
                // return the first clicked monster, otherwise continue looping
                for(id, obj) in objects.iter().enumerate() {
                    if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                        return Some(id);
                    }
                }
            }
            None => return None
        }
    }
}

fn toggle_equipment(
    inventory_id: usize,
    _tcod: &mut Tcod,
    game: &mut Game,
    _objects: &mut [GameObject]
) -> UseResult {
    let equipment = match game.inventory[inventory_id].equipment {
        Some(equipment) => equipment,
        None => return UseResult::Cancelled
    };
    if equipment.equipped {
        game.inventory[inventory_id].dequip(&mut game.messages);
    } else {
        if let Some(current) = get_equipped_in_slot(equipment.slot, &game.inventory) {
            game.inventory[current].dequip(&mut game.messages);
        }
        game.inventory[inventory_id].equip(&mut game.messages);
    }
    UseResult::UsedAndKept
}