use bevy::{
  prelude::*,
};

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

mod splash {
  use bevy::prelude::*;

  use super::{despawn_screen, GameState};

  // This plugin will display a splash screen with Bevy logo for 1 second before switching to the menu
  pub struct SplashPlugin;

  impl Plugin for SplashPlugin {
      fn build(&self, app: &mut App) {
          // As this plugin is managing the splash screen, it will focus on the state `GameState::Splash`
          app
              // When entering the state, spawn everything needed for this screen
              .add_system(splash_setup.in_schedule(OnEnter(GameState::Splash)))
              // While in this state, run the `countdown` system
              .add_system(countdown.in_set(OnUpdate(GameState::Splash)))
              // When exiting the state, despawn everything that was spawned for this screen
              .add_system(
                  despawn_screen::<OnSplashScreen>.in_schedule(OnExit(GameState::Splash)),
              );
      }
  }

  // Tag component used to tag entities added on the splash screen
  #[derive(Component)]
  struct OnSplashScreen;

  // Newtype to use a `Timer` for this screen as a resource
  #[derive(Resource, Deref, DerefMut)]
  struct SplashTimer(Timer);

  fn splash_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
      let icon = asset_server.load("bevy_icon.png");
      // Display the logo
      commands
          .spawn((
              NodeBundle {
                  style: Style {
                      align_items: AlignItems::Center,
                      justify_content: JustifyContent::Center,
                      size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                      ..default()
                  },
                  ..default()
              },
              OnSplashScreen,
          ))
          .with_children(|parent| {
              parent.spawn(ImageBundle {
                  style: Style {
                      // This will set the logo to be 200px wide, and auto adjust its height
                      size: Size::new(Val::Px(200.0), Val::Auto),
                      ..default()
                  },
                  image: UiImage::new(icon),
                  ..default()
              });
          });
      // Insert the timer as a resource
      commands.insert_resource(SplashTimer(Timer::from_seconds(1.0, TimerMode::Once)));
  }

  // Tick the timer, and change state when finished
  fn countdown(
      mut game_state: ResMut<NextState<GameState>>,
      time: Res<Time>,
      mut timer: ResMut<SplashTimer>,
  ) {
      if timer.tick(time.delta()).finished() {
          game_state.set(GameState::Menu);
      }
  }
}


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


mod game {
  use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    sprite::MaterialMesh2dBundle,
  };
  
  use std::f32::consts::PI;

  use super::{despawn_screen, DisplayQuality, GameState, Volume, TEXT_COLOR};

  // This plugin will contain the game. In this case, it's just be a screen that will
  // display the current settings for 5 seconds before returning to the menu
  pub struct GamePlugin;

  impl Plugin for GamePlugin {
      fn build(&self, app: &mut App) {
        app.add_system(update_position)
        .add_system(remove_bullet)
        .add_system(decelerate_starship)
        .add_system(sync_translate_transform.after(update_position))
        .add_system(sync_asteroid_scale_transform)
        .add_system(sync_starship_rotation_transform)
        .add_system(keyboard_events)
        .add_system(detect_starship_asteroid_collision)
        .add_system(detect_bullet_asteroid_collision)
        .add_systems((
              setup_game.in_schedule(OnEnter(GameState::Game)),
              // game.in_set(OnUpdate(GameState::Game)),
              // despawn_screen::<OnGameScreen>.in_schedule(OnExit(GameState::Game)),
          ));
      }
  }
  const VIEWPORT_WIDTH: usize = 1280;
  const VIEWPORT_HEIGHT: usize = 720;
  const VIEWPORT_MAX_X: f32 = VIEWPORT_WIDTH as f32 / 2.0;
  const VIEWPORT_MIN_X: f32 = -VIEWPORT_MAX_X;
  const VIEWPORT_MAX_Y: f32 = VIEWPORT_HEIGHT as f32 / 2.0;
  const VIEWPORT_MIN_Y: f32 = -VIEWPORT_MAX_Y;
  const ASTEROID_VELOCITY: f32 = 2.0;
  const BULLET_VELOCITY: f32 = 6.0;
  const BULLET_DISTANCE: f32 = VIEWPORT_HEIGHT as f32 * 0.8;
  const STARSHIP_ROTATION_SPEED: f32 = 5.0 * 2.0 * PI / 360.0;
  const STARSHIP_ACCELERATION: f32 = 0.2;
  const STARSHIP_DECELERATION: f32 = 0.01;
  const STARSHIP_MAX_VELOCITY: f32 = 10.0;

  // Tag component used to tag entities added on the game screen
  #[derive(Component)]
  struct OnGameScreen;

  #[derive(Resource, Deref, DerefMut)]
  struct GameTimer(Timer);

  fn game_setup(
      mut commands: Commands,
      asset_server: Res<AssetServer>,
      display_quality: Res<DisplayQuality>,
      volume: Res<Volume>,
  ) {
      let font = asset_server.load("fonts/FiraSans-Bold.ttf");

      commands
          .spawn((
              NodeBundle {
                  style: Style {
                      size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                      // center children
                      align_items: AlignItems::Center,
                      justify_content: JustifyContent::Center,
                      ..default()
                  },
                  ..default()
              },
              OnGameScreen,
          ))
          .with_children(|parent| {
              // First create a `NodeBundle` for centering what we want to display
              parent
                  .spawn(NodeBundle {
                      style: Style {
                          // This will display its children in a column, from top to bottom
                          flex_direction: FlexDirection::Column,
                          // `align_items` will align children on the cross axis. Here the main axis is
                          // vertical (column), so the cross axis is horizontal. This will center the
                          // children
                          align_items: AlignItems::Center,
                          ..default()
                      },
                      background_color: Color::BLACK.into(),
                      ..default()
                  })
                  .with_children(|parent| {
                      // Display two lines of text, the second one with the current settings
                      parent.spawn(
                          TextBundle::from_section(
                              "Will be back to the menu shortly...",
                              TextStyle {
                                  font: font.clone(),
                                  font_size: 80.0,
                                  color: TEXT_COLOR,
                              },
                          )
                          .with_style(Style {
                              margin: UiRect::all(Val::Px(50.0)),
                              ..default()
                          }),
                      );
                      parent.spawn(
                          TextBundle::from_sections([
                              TextSection::new(
                                  format!("quality: {:?}", *display_quality),
                                  TextStyle {
                                      font: font.clone(),
                                      font_size: 60.0,
                                      color: Color::BLUE,
                                  },
                              ),
                              TextSection::new(
                                  " - ",
                                  TextStyle {
                                      font: font.clone(),
                                      font_size: 60.0,
                                      color: TEXT_COLOR,
                                  },
                              ),
                              TextSection::new(
                                  format!("volume: {:?}", *volume),
                                  TextStyle {
                                      font: font.clone(),
                                      font_size: 60.0,
                                      color: Color::GREEN,
                                  },
                              ),
                          ])
                          .with_style(Style {
                              margin: UiRect::all(Val::Px(50.0)),
                              ..default()
                          }),
                      );
                  });
          });
      // Spawn a 5 seconds timer to trigger going back to the menu
      commands.insert_resource(GameTimer(Timer::from_seconds(5.0, TimerMode::Once)));
  }

  // Tick the timer, and change state when finished
  fn game(
      time: Res<Time>,
      mut game_state: ResMut<NextState<GameState>>,
      mut timer: ResMut<GameTimer>,
  ) {
      if timer.tick(time.delta()).finished() {
          game_state.set(GameState::Menu);
      }
  }

  #[derive(Debug, Clone, Copy)]
enum AsteroidSize {
  Big,
  Medium,
  Small,
}

impl AsteroidSize {
  fn scale(&self) -> f32 {
    match self {
      AsteroidSize::Big => 100.0,
      AsteroidSize::Medium => 65.0,
      AsteroidSize::Small => 30.0,
    }
  }
}

#[derive(Component)]
struct Starship {
  rotation_angle: f32,
}

impl Starship {
  fn direction(&self) -> Vec2 {
    let (y, x) = (self.rotation_angle + PI / 2.0).sin_cos();

    Vec2::new(x, y)
  }
}

#[derive(Component)]
struct Bullet {
  start: Vec2,
}

#[derive(Component)]
struct Asteroid {
  size: AsteroidSize,
}

#[derive(Component)]
struct Position(Vec2);

#[derive(Component)]
struct Velocity(Vec2);

fn create_starship_mesh() -> Mesh {
  let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

  mesh.insert_attribute(
    Mesh::ATTRIBUTE_POSITION,
    vec![[0.0, 0.5, 0.0], [-0.25, -0.5, 0.0], [0.25, -0.5, 0.0]],
  );
  mesh.set_indices(Some(Indices::U32(vec![0, 1, 2])));
  mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 3]);
  mesh.insert_attribute(
    Mesh::ATTRIBUTE_UV_0,
    vec![[0.5, 0.0], [0.0, 1.0], [1.0, 1.0]],
  );

  mesh
}

fn get_random_point() -> Vec2 {
  Vec2::new(
    (rand::random::<f32>() * 2.0 - 1.0) * (VIEWPORT_WIDTH as f32) / 2.0,
    (rand::random::<f32>() * 2.0 - 1.0) * (VIEWPORT_HEIGHT as f32) / 2.0,
  )
}

fn setup_game(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
) {

  commands.spawn(Camera2dBundle::default());

  commands
    .spawn(Starship {
      rotation_angle: 0.0,
    })
    .insert(Position(Vec2::splat(0.0)))
    .insert(Velocity(Vec2::splat(0.0)))
    .insert(MaterialMesh2dBundle {
      mesh: meshes.add(create_starship_mesh()).into(),
      transform: Transform::default()
        .with_scale(Vec3::splat(50.0))
        .with_translation(Vec3::new(0.0, 0.0, 1.0)),
      material: materials
        .add(ColorMaterial::from(Color::rgba(1.0, 0.0, 0.0, 1.0))),
      ..default()
    });

  for _ in 0..6 {
    commands
      .spawn(Asteroid {
        size: AsteroidSize::Big,
      })
      .insert(Position(get_random_point()))
      .insert(Velocity(get_random_point().normalize() * ASTEROID_VELOCITY))
      .insert(MaterialMesh2dBundle {
        mesh: meshes.add(Mesh::from(shape::Circle::default())).into(),
        transform: Transform::default()
          .with_translation(Vec3::new(0.0, 0.0, 2.0)),
        material: materials
          .add(ColorMaterial::from(Color::rgba(0.8, 0.8, 0.8, 1.0))),
        ..default()
      });
  }
}

fn sync_translate_transform(mut query: Query<(&Position, &mut Transform)>) {
  for (position, mut transform) in &mut query {
    transform.translation =
      Vec3::new(position.0.x, position.0.y, transform.translation.z);
  }
}

fn sync_asteroid_scale_transform(
  mut query: Query<(&Asteroid, &mut Transform)>,
) {
  for (asteroid, mut transform) in &mut query {
    transform.scale = Vec3::splat(asteroid.size.scale())
  }
}

fn sync_starship_rotation_transform(
  mut query: Query<(&Starship, &mut Transform)>,
) {
  for (starship, mut transform) in &mut query {
    transform.rotation = Quat::from_rotation_z(starship.rotation_angle);
  }
}

fn update_position(mut query: Query<(&Velocity, &Transform, &mut Position)>) {
  for (velocity, transform, mut position) in &mut query {
    let mut new_position = position.0 + velocity.0;
    let half_scale = transform.scale.max_element() / 2.0;

    if new_position.x > VIEWPORT_MAX_X + half_scale {
      new_position.x = VIEWPORT_MIN_X - half_scale;
    } else if new_position.x < VIEWPORT_MIN_X - half_scale {
      new_position.x = VIEWPORT_MAX_X + half_scale;
    }

    if new_position.y > VIEWPORT_MAX_Y + half_scale {
      new_position.y = VIEWPORT_MIN_Y - half_scale;
    } else if new_position.y < VIEWPORT_MIN_Y - half_scale {
      new_position.y = VIEWPORT_MAX_Y + half_scale;
    }

    position.0 = new_position;
  }
}

fn keyboard_events(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
  keys: Res<Input<KeyCode>>,
  mut key_evr: EventReader<KeyboardInput>,
  mut query: Query<(&mut Starship, &Position, &mut Velocity)>,
) {
  for (mut starship, starship_position, mut velocity) in &mut query {
    if keys.pressed(KeyCode::Left) {
      starship.rotation_angle += STARSHIP_ROTATION_SPEED;
    } else if keys.pressed(KeyCode::Right) {
      starship.rotation_angle -= STARSHIP_ROTATION_SPEED;
    }

    if keys.pressed(KeyCode::Up) {
      velocity.0 += starship.direction() * STARSHIP_ACCELERATION;

      if velocity.0.length() > STARSHIP_MAX_VELOCITY {
        velocity.0 = velocity.0.normalize_or_zero() * STARSHIP_MAX_VELOCITY;
      }
    }

    for evt in key_evr.iter() {
      if let (ButtonState::Pressed, Some(KeyCode::Space)) =
        (evt.state, evt.key_code)
      {
        commands
          .spawn(Bullet {
            start: starship_position.0.clone(),
          })
          .insert(Position(starship_position.0.clone()))
          .insert(Velocity(
            starship.direction().normalize() * BULLET_VELOCITY,
          ))
          .insert(MaterialMesh2dBundle {
            mesh: meshes.add(Mesh::from(shape::Circle::default())).into(),
            transform: Transform::default()
              .with_scale(Vec3::splat(5.0))
              .with_translation(starship_position.0.clone().extend(0.0)),
            material: materials
              .add(ColorMaterial::from(Color::rgba(1.0, 1.0, 1.0, 1.0))),
            ..default()
          });
      }
    }
  }
}

fn remove_bullet(
  mut commands: Commands,
  query: Query<(Entity, &Bullet, &Position)>,
) {
  for (entity, bullet, position) in &query {
    if (bullet.start - position.0).length() > BULLET_DISTANCE {
      commands.entity(entity).despawn();
    }
  }
}

fn decelerate_starship(
  keys: Res<Input<KeyCode>>,
  mut query: Query<&mut Velocity, With<Starship>>,
) {
  // Only decelerate when not accelerating
  if !keys.pressed(KeyCode::Up) {
    for mut velocity in &mut query {
      velocity.0 *= 1.0 - STARSHIP_DECELERATION;
    }
  }
}

fn detect_starship_asteroid_collision(
  mut commands: Commands,
  starship_query: Query<(Entity, &Transform, &Position), With<Starship>>,
  asteroids_query: Query<(&Transform, &Position), With<Asteroid>>,
) {
  for (starship_entity, starship_transform, starship_position) in
    &starship_query
  {
    for (asteroid_transform, asteroid_position) in &asteroids_query {
      let starship_size = starship_transform.scale.max_element();
      let asteroid_size = asteroid_transform.scale.max_element();
      let distance = (starship_position.0 - asteroid_position.0).length();

      if distance < starship_size / 4.0 + asteroid_size / 2.0 {
        commands.entity(starship_entity).despawn();
      }
    }
  }
}

fn detect_bullet_asteroid_collision(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
  bullets_query: Query<(Entity, &Transform, &Position), With<Bullet>>,
  asteroids_query: Query<(Entity, &Asteroid, &Transform, &Position)>,
) {
  for (bullet_entity, bullet_transform, bullet_position) in &bullets_query {
    for (asteroid_entity, asteroid, asteroid_transform, asteroid_position) in
      &asteroids_query
    {
      let bullet_size = bullet_transform.scale.max_element();
      let asteroid_size = asteroid_transform.scale.max_element();
      let distance = (bullet_position.0 - asteroid_position.0).length();

      if distance < bullet_size / 2.0 + asteroid_size / 2.0 {
        commands.entity(bullet_entity).despawn();
        commands.entity(asteroid_entity).despawn();

        let asteroid_new_size = match asteroid.size {
          AsteroidSize::Big => Some(AsteroidSize::Medium),
          AsteroidSize::Medium => Some(AsteroidSize::Small),
          AsteroidSize::Small => None,
        };

        if let Some(asteroid_new_size) = asteroid_new_size {
          for _ in 0..2 {
            commands
              .spawn(Asteroid {
                size: asteroid_new_size,
              })
              .insert(Position(asteroid_position.0.clone()))
              .insert(Velocity(
                get_random_point().normalize() * ASTEROID_VELOCITY,
              ))
              .insert(MaterialMesh2dBundle {
                mesh: meshes.add(Mesh::from(shape::Circle::default())).into(),
                transform: Transform::default()
                  .with_translation(Vec3::new(0.0, 0.0, 2.0)),
                material: materials
                  .add(ColorMaterial::from(Color::rgba(0.8, 0.8, 0.8, 1.0))),
                ..default()
              });
          }
        }
      }
    }
  }
}
}

mod menu {
  use bevy::{app::AppExit, prelude::*};

  use super::{despawn_screen, DisplayQuality, GameState, Volume, TEXT_COLOR};

  // This plugin manages the menu, with 5 different screens:
  // - a main menu with "New Game", "Settings", "Quit"
  // - a settings menu with two submenus and a back button
  // - two settings screen with a setting that can be set and a back button
  pub struct MenuPlugin;

  impl Plugin for MenuPlugin {
      fn build(&self, app: &mut App) {
          app
              // At start, the menu is not enabled. This will be changed in `menu_setup` when
              // entering the `GameState::Menu` state.
              // Current screen in the menu is handled by an independent state from `GameState`
              .add_state::<MenuState>()
              .add_system(menu_setup.in_schedule(OnEnter(GameState::Menu)))
              // Systems to handle the main menu screen
              .add_systems((
                  main_menu_setup.in_schedule(OnEnter(MenuState::Main)),
                  despawn_screen::<OnMainMenuScreen>.in_schedule(OnExit(MenuState::Main)),
              ))
              // Systems to handle the settings menu screen
              .add_systems((
                  settings_menu_setup.in_schedule(OnEnter(MenuState::Settings)),
                  despawn_screen::<OnSettingsMenuScreen>.in_schedule(OnExit(MenuState::Settings)),
              ))
              // Systems to handle the display settings screen
              .add_systems((
                  display_settings_menu_setup.in_schedule(OnEnter(MenuState::SettingsDisplay)),
                  setting_button::<DisplayQuality>.in_set(OnUpdate(MenuState::SettingsDisplay)),
                  despawn_screen::<OnDisplaySettingsMenuScreen>
                      .in_schedule(OnExit(MenuState::SettingsDisplay)),
              ))
              // Systems to handle the sound settings screen
              .add_systems((
                  sound_settings_menu_setup.in_schedule(OnEnter(MenuState::SettingsSound)),
                  setting_button::<Volume>.in_set(OnUpdate(MenuState::SettingsSound)),
                  despawn_screen::<OnSoundSettingsMenuScreen>
                      .in_schedule(OnExit(MenuState::SettingsSound)),
              ))
              // Common systems to all screens that handles buttons behaviour
              .add_systems((menu_action, button_system).in_set(OnUpdate(GameState::Menu)));
      }
  }

  // State used for the current menu screen
  #[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
  enum MenuState {
      Main,
      Settings,
      SettingsDisplay,
      SettingsSound,
      #[default]
      Disabled,
  }

  // Tag component used to tag entities added on the main menu screen
  #[derive(Component)]
  struct OnMainMenuScreen;

  // Tag component used to tag entities added on the settings menu screen
  #[derive(Component)]
  struct OnSettingsMenuScreen;

  // Tag component used to tag entities added on the display settings menu screen
  #[derive(Component)]
  struct OnDisplaySettingsMenuScreen;

  // Tag component used to tag entities added on the sound settings menu screen
  #[derive(Component)]
  struct OnSoundSettingsMenuScreen;

  const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
  const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
  const HOVERED_PRESSED_BUTTON: Color = Color::rgb(0.25, 0.65, 0.25);
  const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

  // Tag component used to mark which setting is currently selected
  #[derive(Component)]
  struct SelectedOption;

  // All actions that can be triggered from a button click
  #[derive(Component)]
  enum MenuButtonAction {
      Play,
      Settings,
      SettingsDisplay,
      SettingsSound,
      BackToMainMenu,
      BackToSettings,
      Quit,
  }

  // This system handles changing all buttons color based on mouse interaction
  fn button_system(
      mut interaction_query: Query<
          (&Interaction, &mut BackgroundColor, Option<&SelectedOption>),
          (Changed<Interaction>, With<Button>),
      >,
  ) {
      for (interaction, mut color, selected) in &mut interaction_query {
          *color = match (*interaction, selected) {
              (Interaction::Clicked, _) | (Interaction::None, Some(_)) => PRESSED_BUTTON.into(),
              (Interaction::Hovered, Some(_)) => HOVERED_PRESSED_BUTTON.into(),
              (Interaction::Hovered, None) => HOVERED_BUTTON.into(),
              (Interaction::None, None) => NORMAL_BUTTON.into(),
          }
      }
  }

  // This system updates the settings when a new value for a setting is selected, and marks
  // the button as the one currently selected
  fn setting_button<T: Resource + Component + PartialEq + Copy>(
      interaction_query: Query<(&Interaction, &T, Entity), (Changed<Interaction>, With<Button>)>,
      mut selected_query: Query<(Entity, &mut BackgroundColor), With<SelectedOption>>,
      mut commands: Commands,
      mut setting: ResMut<T>,
  ) {
      for (interaction, button_setting, entity) in &interaction_query {
          if *interaction == Interaction::Clicked && *setting != *button_setting {
              let (previous_button, mut previous_color) = selected_query.single_mut();
              *previous_color = NORMAL_BUTTON.into();
              commands.entity(previous_button).remove::<SelectedOption>();
              commands.entity(entity).insert(SelectedOption);
              *setting = *button_setting;
          }
      }
  }

  fn menu_setup(mut menu_state: ResMut<NextState<MenuState>>) {
      menu_state.set(MenuState::Main);
  }

  fn main_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
      let font = asset_server.load("fonts/FiraSans-Bold.ttf");
      // Common style for all buttons on the screen
      let button_style = Style {
          size: Size::new(Val::Px(250.0), Val::Px(65.0)),
          margin: UiRect::all(Val::Px(20.0)),
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
      };
      let button_icon_style = Style {
          size: Size::new(Val::Px(30.0), Val::Auto),
          // This takes the icons out of the flexbox flow, to be positioned exactly
          position_type: PositionType::Absolute,
          // The icon will be close to the left border of the button
          position: UiRect {
              left: Val::Px(10.0),
              right: Val::Auto,
              top: Val::Auto,
              bottom: Val::Auto,
          },
          ..default()
      };
      let button_text_style = TextStyle {
          font: font.clone(),
          font_size: 40.0,
          color: TEXT_COLOR,
      };

      commands
          .spawn((
              NodeBundle {
                  style: Style {
                      size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                      align_items: AlignItems::Center,
                      justify_content: JustifyContent::Center,
                      ..default()
                  },
                  ..default()
              },
              OnMainMenuScreen,
          ))
          .with_children(|parent| {
              parent
                  .spawn(NodeBundle {
                      style: Style {
                          flex_direction: FlexDirection::Column,
                          align_items: AlignItems::Center,
                          ..default()
                      },
                      background_color: Color::CRIMSON.into(),
                      ..default()
                  })
                  .with_children(|parent| {
                      // Display the game name
                      parent.spawn(
                          TextBundle::from_section(
                              "Asteroids",
                              TextStyle {
                                  font: font.clone(),
                                  font_size: 80.0,
                                  color: TEXT_COLOR,
                              },
                          )
                          .with_style(Style {
                              margin: UiRect::all(Val::Px(50.0)),
                              ..default()
                          }),
                      );

                      // Display three buttons for each action available from the main menu:
                      // - new game
                      // - settings
                      // - quit
                      parent
                          .spawn((
                              ButtonBundle {
                                  style: button_style.clone(),
                                  background_color: NORMAL_BUTTON.into(),
                                  ..default()
                              },
                              MenuButtonAction::Play,
                          ))
                          .with_children(|parent| {
                              let icon = asset_server.load("textures/Game Icons/right.png");
                              parent.spawn(ImageBundle {
                                  style: button_icon_style.clone(),
                                  image: UiImage::new(icon),
                                  ..default()
                              });
                              parent.spawn(TextBundle::from_section(
                                  "New Game",
                                  button_text_style.clone(),
                              ));
                          });
                      parent
                          .spawn((
                              ButtonBundle {
                                  style: button_style.clone(),
                                  background_color: NORMAL_BUTTON.into(),
                                  ..default()
                              },
                              MenuButtonAction::Settings,
                          ))
                          .with_children(|parent| {
                              let icon = asset_server.load("textures/Game Icons/wrench.png");
                              parent.spawn(ImageBundle {
                                  style: button_icon_style.clone(),
                                  image: UiImage::new(icon),
                                  ..default()
                              });
                              parent.spawn(TextBundle::from_section(
                                  "Settings",
                                  button_text_style.clone(),
                              ));
                          });
                      parent
                          .spawn((
                              ButtonBundle {
                                  style: button_style,
                                  background_color: NORMAL_BUTTON.into(),
                                  ..default()
                              },
                              MenuButtonAction::Quit,
                          ))
                          .with_children(|parent| {
                              let icon = asset_server.load("textures/Game Icons/exitRight.png");
                              parent.spawn(ImageBundle {
                                  style: button_icon_style,
                                  image: UiImage::new(icon),
                                  ..default()
                              });
                              parent.spawn(TextBundle::from_section("Quit", button_text_style));
                          });
                  });
          });
  }

  fn settings_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
      let button_style = Style {
          size: Size::new(Val::Px(200.0), Val::Px(65.0)),
          margin: UiRect::all(Val::Px(20.0)),
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
      };

      let button_text_style = TextStyle {
          font: asset_server.load("fonts/FiraSans-Bold.ttf"),
          font_size: 40.0,
          color: TEXT_COLOR,
      };

      commands
          .spawn((
              NodeBundle {
                  style: Style {
                      size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                      align_items: AlignItems::Center,
                      justify_content: JustifyContent::Center,
                      ..default()
                  },
                  ..default()
              },
              OnSettingsMenuScreen,
          ))
          .with_children(|parent| {
              parent
                  .spawn(NodeBundle {
                      style: Style {
                          flex_direction: FlexDirection::Column,
                          align_items: AlignItems::Center,
                          ..default()
                      },
                      background_color: Color::CRIMSON.into(),
                      ..default()
                  })
                  .with_children(|parent| {
                      for (action, text) in [
                          (MenuButtonAction::SettingsDisplay, "Display"),
                          (MenuButtonAction::SettingsSound, "Sound"),
                          (MenuButtonAction::BackToMainMenu, "Back"),
                      ] {
                          parent
                              .spawn((
                                  ButtonBundle {
                                      style: button_style.clone(),
                                      background_color: NORMAL_BUTTON.into(),
                                      ..default()
                                  },
                                  action,
                              ))
                              .with_children(|parent| {
                                  parent.spawn(TextBundle::from_section(
                                      text,
                                      button_text_style.clone(),
                                  ));
                              });
                      }
                  });
          });
  }

  fn display_settings_menu_setup(
      mut commands: Commands,
      asset_server: Res<AssetServer>,
      display_quality: Res<DisplayQuality>,
  ) {
      let button_style = Style {
          size: Size::new(Val::Px(200.0), Val::Px(65.0)),
          margin: UiRect::all(Val::Px(20.0)),
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
      };
      let button_text_style = TextStyle {
          font: asset_server.load("fonts/FiraSans-Bold.ttf"),
          font_size: 40.0,
          color: TEXT_COLOR,
      };

      commands
          .spawn((
              NodeBundle {
                  style: Style {
                      size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                      align_items: AlignItems::Center,
                      justify_content: JustifyContent::Center,
                      ..default()
                  },
                  ..default()
              },
              OnDisplaySettingsMenuScreen,
          ))
          .with_children(|parent| {
              parent
                  .spawn(NodeBundle {
                      style: Style {
                          flex_direction: FlexDirection::Column,
                          align_items: AlignItems::Center,
                          ..default()
                      },
                      background_color: Color::CRIMSON.into(),
                      ..default()
                  })
                  .with_children(|parent| {
                      // Create a new `NodeBundle`, this time not setting its `flex_direction`. It will
                      // use the default value, `FlexDirection::Row`, from left to right.
                      parent
                          .spawn(NodeBundle {
                              style: Style {
                                  align_items: AlignItems::Center,
                                  ..default()
                              },
                              background_color: Color::CRIMSON.into(),
                              ..default()
                          })
                          .with_children(|parent| {
                              // Display a label for the current setting
                              parent.spawn(TextBundle::from_section(
                                  "Display Quality",
                                  button_text_style.clone(),
                              ));
                              // Display a button for each possible value
                              for quality_setting in [
                                  DisplayQuality::Low,
                                  DisplayQuality::Medium,
                                  DisplayQuality::High,
                              ] {
                                  let mut entity = parent.spawn(ButtonBundle {
                                      style: Style {
                                          size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                                          ..button_style.clone()
                                      },
                                      background_color: NORMAL_BUTTON.into(),
                                      ..default()
                                  });
                                  entity.insert(quality_setting).with_children(|parent| {
                                      parent.spawn(TextBundle::from_section(
                                          format!("{quality_setting:?}"),
                                          button_text_style.clone(),
                                      ));
                                  });
                                  if *display_quality == quality_setting {
                                      entity.insert(SelectedOption);
                                  }
                              }
                          });
                      // Display the back button to return to the settings screen
                      parent
                          .spawn((
                              ButtonBundle {
                                  style: button_style,
                                  background_color: NORMAL_BUTTON.into(),
                                  ..default()
                              },
                              MenuButtonAction::BackToSettings,
                          ))
                          .with_children(|parent| {
                              parent.spawn(TextBundle::from_section("Back", button_text_style));
                          });
                  });
          });
  }

  fn sound_settings_menu_setup(
      mut commands: Commands,
      asset_server: Res<AssetServer>,
      volume: Res<Volume>,
  ) {
      let button_style = Style {
          size: Size::new(Val::Px(200.0), Val::Px(65.0)),
          margin: UiRect::all(Val::Px(20.0)),
          justify_content: JustifyContent::Center,
          align_items: AlignItems::Center,
          ..default()
      };
      let button_text_style = TextStyle {
          font: asset_server.load("fonts/FiraSans-Bold.ttf"),
          font_size: 40.0,
          color: TEXT_COLOR,
      };

      commands
          .spawn((
              NodeBundle {
                  style: Style {
                      size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                      align_items: AlignItems::Center,
                      justify_content: JustifyContent::Center,
                      ..default()
                  },
                  ..default()
              },
              OnSoundSettingsMenuScreen,
          ))
          .with_children(|parent| {
              parent
                  .spawn(NodeBundle {
                      style: Style {
                          flex_direction: FlexDirection::Column,
                          align_items: AlignItems::Center,
                          ..default()
                      },
                      background_color: Color::CRIMSON.into(),
                      ..default()
                  })
                  .with_children(|parent| {
                      parent
                          .spawn(NodeBundle {
                              style: Style {
                                  align_items: AlignItems::Center,
                                  ..default()
                              },
                              background_color: Color::CRIMSON.into(),
                              ..default()
                          })
                          .with_children(|parent| {
                              parent.spawn(TextBundle::from_section(
                                  "Volume",
                                  button_text_style.clone(),
                              ));
                              for volume_setting in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] {
                                  let mut entity = parent.spawn(ButtonBundle {
                                      style: Style {
                                          size: Size::new(Val::Px(30.0), Val::Px(65.0)),
                                          ..button_style.clone()
                                      },
                                      background_color: NORMAL_BUTTON.into(),
                                      ..default()
                                  });
                                  entity.insert(Volume(volume_setting));
                                  if *volume == Volume(volume_setting) {
                                      entity.insert(SelectedOption);
                                  }
                              }
                          });
                      parent
                          .spawn((
                              ButtonBundle {
                                  style: button_style,
                                  background_color: NORMAL_BUTTON.into(),
                                  ..default()
                              },
                              MenuButtonAction::BackToSettings,
                          ))
                          .with_children(|parent| {
                              parent.spawn(TextBundle::from_section("Back", button_text_style));
                          });
                  });
          });
  }

  fn menu_action(
      interaction_query: Query<
          (&Interaction, &MenuButtonAction),
          (Changed<Interaction>, With<Button>),
      >,
      mut app_exit_events: EventWriter<AppExit>,
      mut menu_state: ResMut<NextState<MenuState>>,
      mut game_state: ResMut<NextState<GameState>>,
  ) {
      for (interaction, menu_button_action) in &interaction_query {
          if *interaction == Interaction::Clicked {
              match menu_button_action {
                  MenuButtonAction::Quit => app_exit_events.send(AppExit),
                  MenuButtonAction::Play => {
                      game_state.set(GameState::Game);
                      menu_state.set(MenuState::Disabled);
                  }
                  MenuButtonAction::Settings => menu_state.set(MenuState::Settings),
                  MenuButtonAction::SettingsDisplay => {
                      menu_state.set(MenuState::SettingsDisplay);
                  }
                  MenuButtonAction::SettingsSound => {
                      menu_state.set(MenuState::SettingsSound);
                  }
                  MenuButtonAction::BackToMainMenu => menu_state.set(MenuState::Main),
                  MenuButtonAction::BackToSettings => {
                      menu_state.set(MenuState::Settings);
                  }
              }
          }
      }
  }
}


// Generic system that takes a component as a parameter, and will despawn all entities with that component
fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
  for entity in &to_despawn {
      commands.entity(entity).despawn_recursive();
  }
}