use crate::{components::*, resources::*, utils::*};
use bevy::prelude::*;

pub fn spawn_inventory(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    inventory_query: Query<(Entity, &Inventory)>,
    core_menu_query: Query<(), With<CoreMenu>>,
    asset_server: Res<AssetServer>,
    world: Res<WorldRes>,
    placer: Res<Placer>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyE) && core_menu_query.is_empty() {
        if let Ok((entity, _)) = inventory_query.single() {
            commands.entity(entity).despawn();
        } else {
            let inventory_entity = commands
                .spawn((
                    Node {
                        width: Val::Vw(80.0),
                        height: Val::Vh(80.0),
                        position_type: PositionType::Absolute,
                        left: Val::Vw(10.0),
                        top: Val::Vh(10.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        padding: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },
                    Inventory {
                        selected_category: 1,
                    },
                    BorderRadius::all(Val::Px(10.0)),
                    BackgroundColor(Color::srgb(0.18, 0.2, 0.23)),
                ))
                .id();

            let categories_panel = commands
                .spawn((
                    Node {
                        width: Val::Percent(25.0),
                        height: Val::Percent(100.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(10.0),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.14, 0.16, 0.19)),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .id();

            commands.entity(categories_panel).with_children(|parent| {
                parent.spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.5, 0.7)),
                    InventoryCategory { category: 1 },
                    Interaction::default(),
                    BorderRadius::all(Val::Px(10.0)),
                    children![(
                        Text::new("1: Portals"),
                        TextFont {
                            font_size: 18.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout {
                            justify: JustifyText::Center,
                            ..Default::default()
                        }
                    )],
                ));

                parent.spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.22, 0.25)),
                    InventoryCategory { category: 2 },
                    Interaction::default(),
                    BorderRadius::all(Val::Px(10.0)),
                    children![(
                        Text::new("2: Conveyors"),
                        TextFont {
                            font_size: 18.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout {
                            justify: JustifyText::Center,
                            ..Default::default()
                        }
                    )],
                ));

                parent.spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.22, 0.25)),
                    InventoryCategory { category: 3 },
                    Interaction::default(),
                    BorderRadius::all(Val::Px(10.0)),
                    children![(
                        Text::new("3: Extractors"),
                        TextFont {
                            font_size: 18.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout {
                            justify: JustifyText::Center,
                            ..Default::default()
                        }
                    )],
                ));

                parent.spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.22, 0.25)),
                    InventoryCategory { category: 4 },
                    Interaction::default(),
                    BorderRadius::all(Val::Px(10.0)),
                    children![(
                        Text::new("4: Factories"),
                        TextFont {
                            font_size: 18.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout {
                            justify: JustifyText::Center,
                            ..Default::default()
                        }
                    )],
                ));
                parent.spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.22, 0.25)),
                    InventoryCategory { category: 5 },
                    Interaction::default(),
                    BorderRadius::all(Val::Px(10.0)),
                    children![(
                        Text::new("5: Storage"),
                        TextFont {
                            font_size: 18.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout {
                            justify: JustifyText::Center,
                            ..Default::default()
                        }
                    )],
                ));
            });

            let items_panel = commands
                .spawn((
                    Node {
                        width: Val::Percent(75.0),
                        height: Val::Percent(100.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        align_content: AlignContent::FlexStart,
                        padding: UiRect::all(Val::Px(15.0)),
                        row_gap: Val::Px(15.0),
                        column_gap: Val::Px(15.0),
                        ..Default::default()
                    },
                    InventoryItemsPanel,
                ))
                .id();

            for ((type_a, type_b), count) in world.resources.iter() {
                if *count > 0 && *type_a == 1 {
                    let texture_path = get_tile_texture((*type_a, *type_b));
                    let is_selected = placer.tile_type == (*type_a, *type_b);

                    let item_entity = commands
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(80.0),
                                height: Val::Px(80.0),
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..Default::default()
                            },
                            BackgroundColor(if is_selected {
                                Color::srgb(0.45, 0.67, 0.9)
                            } else {
                                Color::srgb(0.2, 0.22, 0.25)
                            }),
                            Interaction::default(),
                            InventoryItem {
                                tile_type: (*type_a, *type_b),
                            },
                            BorderRadius::all(Val::Px(10.0)),
                            children![
                                (
                                    Node {
                                        width: Val::Px(48.0),
                                        height: Val::Px(48.0),
                                        ..Default::default()
                                    },
                                    ImageNode::new(asset_server.load(texture_path))
                                ),
                                (
                                    Text::new(format!("x{}", count)),
                                    TextFont {
                                        font_size: 16.0,
                                        ..Default::default()
                                    },
                                    TextColor(Color::WHITE),
                                    TextLayout {
                                        justify: JustifyText::Center,
                                        ..Default::default()
                                    }
                                ),
                            ],
                        ))
                        .id();

                    commands.entity(items_panel).add_child(item_entity);
                }
            }

            commands
                .entity(inventory_entity)
                .add_child(categories_panel);
            commands.entity(inventory_entity).add_child(items_panel);
        }
    }
}

pub fn update_inventory(
    mut commands: Commands,
    inventory_query: Query<(Entity, &Inventory)>,
    category_query: Query<(&InventoryCategory, Entity)>,
) {
    if let Ok((_, inventory)) = inventory_query.single() {
        for (category, entity) in category_query.iter() {
            let color = if category.category == inventory.selected_category {
                Color::srgb(0.45, 0.67, 0.9)
            } else {
                Color::srgb(0.2, 0.22, 0.25)
            };
            commands.entity(entity).insert(BackgroundColor(color));
        }
    }
}

pub fn handle_inventory_interaction(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    world: Res<WorldRes>,
    category_query: Query<(&Interaction, &InventoryCategory), Changed<Interaction>>,
    item_query: Query<(&Interaction, &InventoryItem)>,
    mut inventory_query: Query<(Entity, &mut Inventory)>,
    mut placer: ResMut<Placer>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    context_menu_query: Query<Entity, With<InventoryContextMenu>>,
    mut bg_color_query: Query<(&mut BackgroundColor, &InventoryItem)>,
    item_panel_query: Query<Entity, With<InventoryItemsPanel>>,
    existing_items_query: Query<Entity, With<InventoryItem>>,
) {
    for (interaction, category) in category_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, mut inventory)) = inventory_query.single_mut() {
                if inventory.selected_category != category.category {
                    inventory.selected_category = category.category;

                    for item_entity in existing_items_query.iter() {
                        commands.entity(item_entity).despawn();
                    }

                    if let Ok(panel_entity) = item_panel_query.single() {
                        for ((type_a, type_b), count) in world.resources.iter() {
                            if *count > 0 && *type_a == category.category {
                                let texture_path = get_tile_texture((*type_a, *type_b));
                                let is_selected = placer.tile_type == (*type_a, *type_b);

                                let item_entity = commands
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(80.0),
                                            height: Val::Px(80.0),
                                            display: Display::Flex,
                                            flex_direction: FlexDirection::Column,
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::Center,
                                            ..Default::default()
                                        },
                                        BackgroundColor(if is_selected {
                                            Color::srgb(0.45, 0.67, 0.9)
                                        } else {
                                            Color::srgb(0.2, 0.22, 0.25)
                                        }),
                                        Interaction::default(),
                                        InventoryItem {
                                            tile_type: (*type_a, *type_b),
                                        },
                                        BorderRadius::all(Val::Px(10.0)),
                                        children![
                                            (
                                                Node {
                                                    width: Val::Px(48.0),
                                                    height: Val::Px(48.0),
                                                    ..Default::default()
                                                },
                                                ImageNode::new(asset_server.load(texture_path))
                                            ),
                                            (
                                                Text::new(format!("x{}", count)),
                                                TextFont {
                                                    font_size: 16.0,
                                                    ..Default::default()
                                                },
                                                TextColor(Color::WHITE),
                                                TextLayout {
                                                    justify: JustifyText::Center,
                                                    ..Default::default()
                                                }
                                            ),
                                        ],
                                    ))
                                    .id();

                                commands.entity(panel_entity).add_child(item_entity);
                            }
                        }
                    }
                }
            }
        }
    }

    for (interaction, item) in item_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            placer.tile_type = item.tile_type;

            for (mut bg_color, option) in bg_color_query.iter_mut() {
                *bg_color = if option.tile_type == placer.tile_type {
                    BackgroundColor(Color::srgb(0.45, 0.67, 0.9))
                } else {
                    BackgroundColor(Color::srgb(0.2, 0.22, 0.25))
                };
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
                    InventoryContextMenu,
                    ZIndex(100),
                    children![
                        (
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
                            HotkeyOption {
                                tile_type: item.tile_type,
                            },
                            Interaction::default(),
                            children![(
                                Text::new("Assign Hotkey"),
                                TextFont {
                                    font_size: 16.0,
                                    ..Default::default()
                                },
                                TextColor(Color::WHITE),
                            )]
                        ),
                        (
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
                            SellOption {
                                tile_type: item.tile_type,
                            },
                            Interaction::default(),
                            children![(
                                Text::new(format!("Sell (${})", get_tile_price(item.tile_type))),
                                TextFont {
                                    font_size: 16.0,
                                    ..Default::default()
                                },
                                TextColor(Color::WHITE),
                            )]
                        )
                    ],
                ));

                break;
            }
        }
    }
}
