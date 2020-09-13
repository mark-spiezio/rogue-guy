

# Rogue Guy

A simple [Roguelike](https://en.wikipedia.org/wiki/Roguelike) game.  Used as my own play area for learning the Rust programming language.  This is my take on the [Roguelike Tutorial in Rust + tcod](https://tomassedovic.github.io/roguelike-tutorial/index.html).

Code reader be warned.  This project contains rough and unsightly code.  It is subject to my own whim and musings as I journey through the learning process of the Rust language.  It is not to be taken seriously.

---

# How To Play
You're the Guy.  '@' - That's you, that's your Guy.  Rome through the randomly generated dungeon attacking monsters and gaining experience.  Difficulty increases and new items appear as you traverse to deeper and deeper levels of the dungeon!

## Movement
You can move by using the arrow keys or number pad.  The arrow keys will only move you up, down, left and right.  But the number pad gives you all 8 degrees of motion.  The number 5 key allows you to 'wait' a turn and do nothing.  Attack monsters by walking in to them or use scrolls within their vicinity.


## Actions
* **alt-enter** - Toggle back and forth between full screen and window modes.
* **c** - Character information.  View stats about your character.  
* **i** - Inventory.  View your inventory where you can use the the items you find within the dungeon.
* **g** - Get. Get an item you've found.
* **d** - Drop.  Drop an item from your inventory.
* **<** - Take Stairs.
* **Esc** - Save and leave the game.


## Items
* **@** - The Guy.  That's you,  You're the guy!
* **%** - A dead player or monster.  Hopefully not The Guy.
* **!** - Heal potion.
* **/** - Sword.  (Attack bonus + 3)
* **]** - Shield.  (Defense bonus + 1)
* **#** - Scroll.
  * Scroll of Confusion - Single monster attack.  Targeted monster moves randomly for 5 turns.
  * Scroll of Lightning - Single monster attack.  Closest monster takes major damage.
  * Scroll of Fireball - Area range attack, can damage The Guy too.

## Monsters
* **o** - Orc
* **T** - Troll

