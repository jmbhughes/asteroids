use bevy::{
  prelude::*,
};

mod splash;
mod menu;
mod game;


#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Splash,
    Menu,
    Game,
}

// One of the two settings that can be set through the menu. It will be a resource in the app
#[derive(Resource, Debug, Component, PartialEq, Eq, Clone, Copy)]
enum DisplayQuality {
    Low,
    Medium,
    High,
}

// One of the two settings that can be set through the menu. It will be a resource in the app
#[derive(Resource, Debug, Component, PartialEq, Eq, Clone, Copy)]
struct Volume(u32);


const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);


fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .insert_resource(DisplayQuality::Medium)
    .insert_resource(Volume(7))
    .add_startup_system(setup)
    .add_state::<GameState>()
    .add_plugin(splash::SplashPlugin)
    .add_plugin(menu::MenuPlugin)
    .add_plugin(game::GamePlugin)
    .run();
}

fn setup(mut commands: Commands) {
  commands.spawn(Camera2dBundle::default());
}


// Generic system that takes a component as a parameter, and will despawn all entities with that component
fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
  for entity in &to_despawn {
      commands.entity(entity).despawn_recursive();
  }
}