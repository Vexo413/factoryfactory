use crate::{components::*, resources::*, tiles::Core, utils::*};
use bevy::prelude::*;

pub fn handle_core_menu_interaction(
    mut commands: Commands,
    category_query: Query<(&Interaction, &CoreCategory), Changed<Interaction>>,
    item_query: Query<(&Interaction, &CoreMenuItem)>,
    mut item_bg_query: Query<(&mut BackgroundColor, &CoreMenuItem)>,
    mut core_menu_query: Query<(Entity, &mut CoreMenu)>,
    panel_query: Query<Entity, With<CoreItemsPanel>>,
    existing_items_query: Query<Entity, With<CoreMenuItem>>,
    mut world: ResMut<WorldRes>,
    close_button_query: Query<(&Interaction, &Name), (Changed<Interaction>, Without<CoreMenuItem>)>,
    asset_server: Res<AssetServer>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    context_menu_query: Query<Entity, With<CoreContextMenu>>,
) {
    for (interaction, category) in category_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, mut core_menu)) = core_menu_query.single_mut() {
                if core_menu.selected_category != category.category {
                    core_menu.selected_category = category.category;

                    for entity in existing_items_query.iter() {
                        commands.entity(entity).despawn();
                    }

                    let current_tile_id =
                        if let Some((tile, _)) = world.tiles.get(&core_menu.position) {
                            if let Some(core) = tile.as_any().downcast_ref::<Core>() {
                                core.tile_id
                            } else {
                                (0, 1)
                            }
                        } else {
                            (0, 1)
                        };

                    if let Ok(panel_entity) = panel_query.single() {
                        spawn_category_items(
                            &mut commands,
                            &world,
                            category.category,
                            &asset_server,
                            panel_entity,
                            current_tile_id,
                        );
                    }
                }
            }
        }
    }

    for (interaction, item) in item_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, core_menu)) = core_menu_query.single() {
                if let Some((tile, _)) = world.tiles.get_mut(&core_menu.position) {
                    if let Some(core) = tile.as_any_mut().downcast_mut::<Core>() {
                        core.tile_id = item.tile_type;
                        core.interval = get_tile_core_interval(item.tile_type);
                        core.ticks = 0;

                        for (mut bg_color, option) in item_bg_query.iter_mut() {
                            *bg_color = if option.tile_type == item.tile_type {
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

    for (interaction, name) in close_button_query.iter() {
        if matches!(interaction, Interaction::Pressed) && name.as_str() == "close_button" {
            if let Ok((entity, _)) = core_menu_query.single() {
                commands.entity(entity).despawn();
            }
        }
    }

    if mouse_button_input.just_pressed(MouseButton::Right) {
        for (interaction, item) in item_query.iter() {
            if matches!(interaction, Interaction::Hovered) {
                for entity in context_menu_query.iter() {
                    commands.entity(entity).despawn();
                }

                commands.spawn((
                    Node {
                        width: Val::Px(150.0),
                        height: Val::Auto,
                        position_type: PositionType::Absolute,
                        right: Val::Px(10.0),
                        top: Val::Px(10.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    BorderRadius::all(Val::Px(5.0)),
                    CoreContextMenu,
                    ZIndex(100),
                    children![(
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(30.0),
                            display: Display::Flex,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            margin: UiRect::bottom(Val::Px(5.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
                        BorderRadius::all(Val::Px(3.0)),
                        BuyOption {
                            tile_type: item.tile_type
                        },
                        Interaction::default(),
                        children![(
                            Text::new(format!("Buy (${})", get_tile_price(item.tile_type))),
                            TextFont {
                                font_size: 16.0,
                                ..Default::default()
                            },
                            TextColor(Color::WHITE),
                        )]
                    )],
                ));
                break;
            }
        }
    }

    if let Ok((_, core_menu)) = core_menu_query.single() {
        if !panel_query.is_empty() && existing_items_query.is_empty() {
            let current_tile_id = if let Some((tile, _)) = world.tiles.get(&core_menu.position) {
                if let Some(core) = tile.as_any().downcast_ref::<Core>() {
                    core.tile_id
                } else {
                    (0, 1)
                }
            } else {
                (0, 1)
            };

            if let Ok(panel_entity) = panel_query.single() {
                spawn_category_items(
                    &mut commands,
                    &world,
                    core_menu.selected_category,
                    &asset_server,
                    panel_entity,
                    current_tile_id,
                );
            }
        }
    }
}
fn spawn_category_items(
    commands: &mut Commands,
    world: &WorldRes,
    category: u8,
    asset_server: &AssetServer,
    panel_entity: Entity,
    selected_tile_id: (u8, u8),
) {
    let tile_types = match category {
        1 => vec![(1, 2)],
        2 => vec![(2, 1), (2, 2), (2, 3)],
        3 => vec![(3, 1), (3, 2), (3, 3)],
        4 => vec![(4, 1), (4, 2), (4, 3), (4, 4), (4, 5)],
        5 => vec![(5, 1), (5, 2), (5, 3)],
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
                CoreMenuItem { tile_type },
                Interaction::default(),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Node {
                        width: Val::Px(48.0),
                        height: Val::Px(48.0),
                        margin: UiRect::bottom(Val::Px(5.0)),
                        ..default()
                    },
                    ImageNode::new(asset_server.load(get_tile_texture(tile_type))),
                ));

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

pub fn update_core_menu(
    core_menu_query: Query<&CoreMenu>,
    mut category_query: Query<(&CoreCategory, &mut BackgroundColor)>,
    world: Res<WorldRes>,
    mut text_query: Query<(&mut Text, &Name)>,
) {
    if let Ok(core_menu) = core_menu_query.single() {
        for (cat, mut bg_color) in category_query.iter_mut() {
            *bg_color = if cat.category == core_menu.selected_category {
                BackgroundColor(Color::srgb(0.45, 0.67, 0.9))
            } else {
                BackgroundColor(Color::srgb(0.2, 0.22, 0.25))
            };
        }
    }
    if let Ok(core_menu) = core_menu_query.single() {
        if let Some((tile, _)) = world.tiles.get(&core_menu.position) {
            if let Some(core) = tile.as_any().downcast_ref::<Core>() {
                for (mut text, name) in text_query.iter_mut() {
                    if name.as_str() == "core_progress" {
                        text.0 = format!("Progress: {}/{} seconds", core.ticks, core.interval);
                    }
                }
            }
        }
    }
}
