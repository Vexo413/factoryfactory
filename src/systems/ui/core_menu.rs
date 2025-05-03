use crate::{components::*, resources::*, tiles::Core, utils::*};
use bevy::prelude::*;

pub fn handle_core_context_menu(
    mut commands: Commands,
    category_query: Query<(&Interaction, &CoreCategory), Changed<Interaction>>,
    interaction_tile_query: Query<(&Interaction, &TileTypeOption), Changed<Interaction>>,
    mut bg_color_query: Query<(&mut BackgroundColor, &TileTypeOption)>,
    mut core_menu_query: Query<(Entity, &mut CoreContextMenu)>,
    tile_panel_query: Query<Entity, With<CoreItemsPanel>>,
    tile_option_query: Query<Entity, With<TileTypeOption>>,
    mut world: ResMut<WorldRes>,
    close_button_query: Query<
        (&Interaction, &Name),
        (Changed<Interaction>, Without<TileTypeOption>),
    >,
    asset_server: Res<AssetServer>,
) {
    // Handle category changes
    for (interaction, category) in category_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, mut core_menu)) = core_menu_query.single_mut() {
                core_menu.selected_category = category.category;

                // Get current core tile_id for highlighting
                let current_tile_id = if let Some((tile, _)) = world.tiles.get(&core_menu.position)
                {
                    if let Some(core) = tile.as_any().downcast_ref::<Core>() {
                        core.tile_id
                    } else {
                        (0, 1) // default
                    }
                } else {
                    (0, 1) // default
                };

                // Refresh tile options with the current selection
                update_core_tiles(
                    &mut commands,
                    &world,
                    core_menu.selected_category,
                    &asset_server,
                    &tile_panel_query,
                    &tile_option_query,
                    current_tile_id,
                );
            }
        }
    }

    // Handle tile type selection
    for (interaction, tile_option) in interaction_tile_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, core_context)) = core_menu_query.single() {
                if let Some((tile, _)) = world.tiles.get_mut(&core_context.position) {
                    if let Some(core) = tile.as_any_mut().downcast_mut::<Core>() {
                        // Set new tile type and reset ticks
                        core.tile_id = tile_option.tile_type;
                        core.interval = get_tile_core_interval(tile_option.tile_type);
                        core.ticks = 0;

                        // Update background colors
                        for (mut bg_color, option) in bg_color_query.iter_mut() {
                            *bg_color = if option.tile_type == tile_option.tile_type {
                                BackgroundColor(Color::srgb(0.45, 0.67, 0.9))
                            } else {
                                BackgroundColor(Color::srgb(0.2, 0.22, 0.25))
                            };
                        }
                    }
                }
            }
        }
    }

    // Handle close button
    for (interaction, name) in close_button_query.iter() {
        if matches!(interaction, Interaction::Pressed) && name.as_str() == "close_button" {
            if let Ok((entity, _)) = core_menu_query.single() {
                commands.entity(entity).despawn();
            }
        }
    }

    // Initial population of the menu when it's created - get the core's current tile_id
    if let Ok((_, core_menu)) = core_menu_query.single() {
        if tile_panel_query.iter().len() > 0 && tile_option_query.iter().len() == 0 {
            let current_tile_id = if let Some((tile, _)) = world.tiles.get(&core_menu.position) {
                if let Some(core) = tile.as_any().downcast_ref::<Core>() {
                    core.tile_id
                } else {
                    (0, 1) // default
                }
            } else {
                (0, 1) // default
            };

            update_core_tiles(
                &mut commands,
                &world,
                core_menu.selected_category,
                &asset_server,
                &tile_panel_query,
                &tile_option_query,
                current_tile_id,
            );
        }
    }
}
pub fn update_core_menu_ui(
    core_menu_query: Query<&CoreContextMenu>,
    mut category_query: Query<(&CoreCategory, &mut BackgroundColor)>,
) {
    if let Ok(core_menu) = core_menu_query.single() {
        // Update the category colors based on the selection
        for (cat, mut bg_color) in category_query.iter_mut() {
            *bg_color = if cat.category == core_menu.selected_category {
                BackgroundColor(Color::srgb(0.45, 0.67, 0.9))
            } else {
                BackgroundColor(Color::srgb(0.2, 0.22, 0.25))
            };
        }
    }
}
pub fn update_core_progress_text(
    world: Res<WorldRes>,
    core_menu_query: Query<&CoreContextMenu>,
    mut text_query: Query<(&mut Text, &Name)>,
) {
    if let Ok(core_menu) = core_menu_query.single() {
        if let Some((tile, _)) = world.tiles.get(&core_menu.position) {
            if let Some(core) = tile.as_any().downcast_ref::<Core>() {
                for (mut text, name) in text_query.iter_mut() {
                    println!("main");
                    if name.as_str() == "core_progress" {
                        println!("a");
                        text.0 = format!("Progress: {}/{} seconds", core.ticks, core.interval);
                        println!("b");
                    }
                }
            }
        }
    }
}
fn update_core_tiles(
    commands: &mut Commands,
    world: &WorldRes,
    category: u8,
    asset_server: &AssetServer,
    panel_query: &Query<Entity, With<CoreItemsPanel>>,
    tile_query: &Query<Entity, With<TileTypeOption>>,
    selected_tile_id: (u8, u8), // Add current core selected tile to highlight it
) {
    // Remove existing tile options
    for entity in tile_query.iter() {
        commands.entity(entity).despawn();
    }

    // If we can get the panel, populate it with new tiles
    if let Ok(panel_entity) = panel_query.single() {
        let tile_types = match category {
            1 => vec![(1, 1), (1, 2), (1, 3)],         // Conveyors
            2 => vec![(2, 1), (2, 2), (2, 3), (2, 4)], // Factories
            3 => vec![(3, 1), (3, 2), (3, 3)],         // Extractors
            4 => vec![(4, 1), (5, 1)],                 // Special
            _ => vec![],
        };

        for tile_type in tile_types {
            let count = *world.resources.get(&tile_type).unwrap_or(&0);
            let interval = get_tile_core_interval(tile_type);
            let is_selected = tile_type == selected_tile_id;

            let tile_entity = commands
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(110.0),
                        min_height: Val::Px(140.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(if is_selected {
                        Color::srgb(0.45, 0.67, 0.9)
                    } else {
                        Color::srgb(0.2, 0.22, 0.25)
                    }),
                    BorderRadius::all(Val::Px(5.0)),
                    TileTypeOption { tile_type },
                    Interaction::default(),
                ))
                .with_children(|parent| {
                    // Tile image
                    parent.spawn((
                        Node {
                            width: Val::Px(48.0),
                            height: Val::Px(48.0),
                            margin: UiRect::bottom(Val::Px(5.0)),
                            ..default()
                        },
                        ImageNode::new(asset_server.load(get_tile_texture(tile_type))),
                    ));

                    // Tile name
                    parent.spawn((
                        Text::new(get_tile_name(tile_type)),
                        TextFont {
                            font_size: 12.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout {
                            justify: JustifyText::Center,
                            ..Default::default()
                        },
                    ));

                    // Tile interval
                    parent.spawn((
                        Text::new(format!("Takes {} seconds", interval)),
                        TextFont {
                            font_size: 12.0,
                            ..Default::default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        TextLayout {
                            justify: JustifyText::Center,
                            ..Default::default()
                        },
                    ));

                    // Available count
                    parent.spawn((
                        Text::new(format!("Available: {}", count)),
                        TextFont {
                            font_size: 12.0,
                            ..Default::default()
                        },
                        TextColor(if count > 0 {
                            Color::WHITE
                        } else {
                            Color::srgb(1.0, 0.5, 0.5)
                        }),
                        TextLayout {
                            justify: JustifyText::Center,
                            ..Default::default()
                        },
                    ));
                })
                .id();

            commands.entity(panel_entity).add_child(tile_entity);
        }
    }
}
