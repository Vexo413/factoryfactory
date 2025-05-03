mod components;
mod constants;
mod resources;
mod systems;
mod tiles;
mod types;
mod utils;

use bevy::prelude::*;
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

// Re-export commonly used types
pub use components::*;
pub use constants::*;
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
            EguiPlugin {
                enable_multipass_for_primary_context: true,
            },
            WorldInspectorPlugin::new(),
        ))
        .insert_resource(WorldRes::default())
        .insert_resource(Placer::default())
        .insert_resource(Hotkeys::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                // World systems
                (
                    systems::manage_terrain_chunks,
                    systems::tick_tiles,
                    systems::update_tile_visuals.after(tick_tiles),
                    systems::animate_items.after(update_tile_visuals),
                )
                    .chain(),
                // Player interaction systems
                (systems::manage_tiles, systems::move_camera).chain(),
                // UI systems
                (
                    systems::update_inventory,
                    systems::handle_inventory_interaction,
                    systems::exit_menu,
                    systems::spawn_inventory,
                    systems::handle_context_menu,
                    systems::handle_hotkey_assignment,
                    systems::handle_core_context_menu,
                    systems::update_core_menu_ui,
                    systems::update_core_progress_text,
                )
                    .chain(),
            ),
        )
        .run();
}

// Setup function that initializes the game world
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut world: ResMut<WorldRes>) {
    commands.spawn(Camera2d);

    if world.tiles.is_empty() {
        world.tiles.insert(
            Position::new(-3, -3),
            (
                Box::new(tiles::Extractor {
                    position: Position::new(-3, -3),
                    direction: Direction::Right,
                    extractor_type: ExtractorType::RawRigtorium,
                    item: None,
                }),
                (3, 1),
            ),
        );
        world.tiles.insert(
            Position::new(3, 3),
            (
                Box::new(tiles::Extractor {
                    position: Position::new(3, 3),
                    direction: Direction::Left,
                    extractor_type: ExtractorType::RawFlextorium,
                    item: None,
                }),
                (3, 2),
            ),
        );
    }

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
