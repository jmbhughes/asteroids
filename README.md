# asteroids
asteroids spin-off in Rust

I'm using this project [to learn Rust](https://publish.obsidian.md/arbor/learning+rust). 

Based on [this tutorial](https://github.com/yishn/lets-code/tree/main/asteroids). 

# How to run
1. Clone the repo. 
2. Change into the directory. 
3. Run `cargo run`

## New features added already
- Converted to Bevy 0.10
- Fixed bullet bug where the bullet first shows up in the center of the screen
- Added start menu and settings menus (followed [this example](https://github.com/bevyengine/bevy/blob/release-0.10.0/examples/games/game_menu.rs))

## Roadmap
- Allow lives for the ship
- Add pause menu
- Return to menu on death
- Keep a high score (longest duration plus asteroids shot or something)
- Change shape of asteroids to something more visually interesting
- Add sound effects
- Build with webassembly for online play? 
- Make a more complicated game from it where you can get powerups or upgrade your ship? 
