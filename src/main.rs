mod components;
mod constants;
mod resources;
mod systems;
mod tiles;
mod types;
mod utils;

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};

pub use components::*;
pub use constants::*;
use rand::{Rng, rng};
pub use resources::*;
pub use systems::*;
pub use tiles::*;
pub use types::*;
pub use utils::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Factory Factory".into(),
                    name: Some("factoyfactory.app".into()),
                    resolution: (1280.0, 720.0).into(),
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            }),
            EmbeddedAssetPlugin {
                mode: PluginMode::AutoLoad,
            },
        ))
        .insert_resource(Placer::default())
        .add_systems(Startup, (setup_resources, setup.after(setup_resources)))
        .add_systems(
            Update,
            (
                (
                    systems::manage_terrain_chunks,
                    systems::tick_tiles,
                    systems::spawn_animations.after(tick_tiles),
                    systems::update_tile_visuals.after(spawn_animations),
                    systems::animate_items.after(update_tile_visuals),
                )
                    .chain(),
                (systems::manage_tiles, systems::move_camera).chain(),
                (
                    systems::exit_menu,
                    systems::spawn_inventory,
                    systems::update_inventory,
                    systems::handle_inventory_interaction,
                    systems::handle_inventory_context_menu,
                    systems::handle_hotkey_assignment,
                    systems::update_core_menu,
                    systems::handle_core_menu_interaction,
                    systems::handle_core_context_menu,
                    systems::update_money_widget,
                )
                    .chain(),
            ),
        )
        .run();
}
fn setup_resources(mut commands: Commands) {
    match WorldRes::load_game("savegame.ffs") {
        Ok((world, hotkeys_map)) => {
            commands.insert_resource(world);
            commands.insert_resource(Hotkeys {
                mappings: hotkeys_map,
            });
        }
        Err(_) => {
            let mut resources = HashMap::new();
            resources.insert((2, 1), 20);
            resources.insert((2, 2), 5);
            resources.insert((2, 3), 5);
            resources.insert((3, 1), 1);
            resources.insert((3, 3), 1);
            resources.insert((4, 1), 1);

            let mut tiles: HashMap<Position, (Box<dyn Tile + 'static>, (u8, u8))> = HashMap::new();
            tiles.insert(
                Position::new(0, 0),
                (
                    Box::new(Core {
                        position: Position::new(0, 0),
                        interval: 10,
                        ticks: 0,
                        tile_id: (6, 1),
                    }),
                    (6, 1),
                ),
            );

            commands.insert_resource(WorldRes {
                tiles,
                terrain: HashMap::new(),
                loaded_chunks: HashSet::new(),
                resources,
                world_seed: rng().random_range(u32::MIN..u32::MAX),
                tick_timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Repeating),
                tick_count: 0,
                actions: Vec::new(),
                money: 100,
            });
            commands.insert_resource(Hotkeys::default());
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, world: Res<WorldRes>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(5.0),
            top: Val::Px(5.0),
            min_width: Val::Vw(10.0),
            height: Val::Vh(5.0),
            display: Display::Grid,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BorderRadius::all(Val::Px(10.0)),
        BackgroundColor(Color::srgb(0.18, 0.2, 0.23)),
        children![(
            Text::new(""),
            TextFont {
                font_size: 16.0,
                ..Default::default()
            },
            TextColor(Color::WHITE),
            MoneyWidget,
        )],
    ));

    for (pos, _) in world.tiles.iter() {
        commands
            .spawn((
                Sprite::from_image(
                    asset_server.load("embedded://textures/tiles/conveyors/back.png"),
                ),
                Transform {
                    translation: Vec3::new(pos.x as f32 * TILE_SIZE, pos.y as f32 * TILE_SIZE, 0.0),
                    scale: Vec3::splat(TILE_SIZE / IMAGE_SIZE),
                    ..Default::default()
                },
                TileSprite { pos: *pos },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Sprite::from_image(asset_server.load("embedded://textures/items/none.png")),
                    Transform::from_scale(Vec3::splat(0.5)),
                ));
            });
    }
}
