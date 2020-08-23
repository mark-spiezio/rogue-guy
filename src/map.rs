use serde::{Deserialize, Serialize};
use tcod::colors::*;
use tcod::map::{FovAlgorithm};
use std::cmp;
use rand::Rng;
use crate::game_object::*;

pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 43;

pub const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
pub const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
pub const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
pub const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

pub const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
pub const FOV_LIGHT_WALLS: bool = true;
pub const TORCH_RADIUS: i32 = 10;

const MAX_ROOM_MONSTERS: i32 = 3;

const MAX_ROOM_ITEMS: i32 = 2;

// alias Vec<Vec<Tile>> to "Map"
pub type Map = Vec<Vec<Tile>>;

// A tile of the map and it's properties
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tile {
    pub blocked: bool,
    pub block_sight: bool,
    pub explored: bool
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            explored: false
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            explored: false
        }
    }
}

pub fn make_map(objects: &mut Vec<GameObject>) -> Map {
    // fill map with "blocked" tiles
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
    let mut rooms = vec![];

    // for "next levels", remove any existing objects except the player
    objects.retain(|i| i.name == "player");

    for _ in 0..MAX_ROOMS {
        // random width and height of room
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        // random position without going out of the map boundaries
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);
        let new_room = Rect::new(x, y, w, h);

        // for each existing room see if new room intersects with it
        let failed = rooms
            .iter()
            .any(|other_room| new_room.intersects_with(other_room));

        // No intersections, lets create the new room
        if !failed {
            create_room(new_room, &mut map);
            place_objects(new_room, &map, objects);

            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                // This is the first room, set the player here
                objects[PLAYER].set_pos(new_x, new_y);

            } else {
                // All the other rooms, connect to the previous room
                // with a tunnel

                let(prev_x, prev_y) = rooms[rooms.len() -1].center();

                if rand::random() {
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }
            rooms.push(new_room);
        }
    }

    // create stairs at the center of the last room
    let(last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
    let mut stairs = GameObject::new(last_room_x, last_room_y, '<', "stairs", WHITE, false);
    stairs.always_visible = true;
    objects.push(stairs);

    map
}

// A rectangle on the map used to characterise a room.
#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h
        }
    }
    
    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }
    
    pub fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<GameObject>) {
    // choose random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        // choose random spot for this monster
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        // only place it if the tile is not blocked
        if !is_blocked(x, y, map, objects) {
            let mut monster = if rand::random::<f32>() < 0.8 {
                // 80% chance of getting an orc
                let mut orc = GameObject::new(x, y, 'o', "orc", DESATURATED_GREEN, true);
                orc.fighter = Some(Fighter {
                    max_hp: 10,
                    hp: 10,
                    defense: 0,
                    power: 3,
                    xp: 35,
                    on_death: DeathCallback::Monster
                });
                orc.ai = Some(Ai::Basic);
                orc
            } else {
                // 20% chance of getting a troll
                let mut troll = GameObject::new(x, y, 'T', "troll", DARKER_GREEN, true);
                troll.fighter = Some(Fighter {
                    max_hp: 16,
                    hp: 16,
                    defense: 1,
                    power: 4,
                    xp: 100,
                    on_death: DeathCallback::Monster
                });
                troll.ai = Some(Ai::Basic);
                troll
            };
            
            monster.alive = true;
            objects.push(monster);
        }
    }

    // choose random number of items
    let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);
    
    for _ in 0..num_items {
        // choose random spot for this item
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        // only place it if the tle is not blocked
        if !is_blocked(x, y, map, objects) {
            let dice = rand::random::<f32>();
            let mut item = if dice < 0.7 {
                // healing potion (70% chance)
                let mut object = GameObject::new(x, y, '!', "healing potion", VIOLET, false);
                object.item = Some(Item::Heal);
                object
            } else if dice < 0.7 + 0.1 {
                // lightning bolt scroll (30% chance)
                let mut object = GameObject::new(x, y, '#', "scroll of lightning bolt", LIGHT_YELLOW, false);
                object.item = Some(Item::Lightning);
                object
            } else if dice < 0.7 + 0.1 + 0.1 {
                // lightning bolt scroll (30% chance)
                let mut object = GameObject::new(x, y, '#', "scroll of fireball", LIGHT_YELLOW, false);
                object.item = Some(Item::Fireball);
                object
            } else {
                let mut object = GameObject::new(x, y, '#', "scroll of confusion", LIGHT_YELLOW, false);
                object.item = Some(Item::Confuse);
                object
            };

            item.always_visible = true;
            objects.push(item);
        }
    }
}