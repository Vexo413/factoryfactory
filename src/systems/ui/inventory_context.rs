use crate::{components::*, get_tile_price, resources::*};
use bevy::prelude::*;

pub fn handle_inventory_context_menu(
    mut commands: Commands,
    hotkey_interaction_query: Query<(&Interaction, &HotkeyOption), Changed<Interaction>>,
    hotkey_button_interaction_query: Query<(), (Changed<Interaction>, With<HotkeyButton>)>,
    sell_interaction_query: Query<(&Interaction, &SellOption), Changed<Interaction>>,
    context_menu_query: Query<Entity, With<InventoryContextMenu>>,
    mut world: ResMut<WorldRes>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    for (interaction, hotkey_option) in hotkey_interaction_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok(entity) = context_menu_query.single() {
                commands.entity(entity).despawn();

                let new_menu = commands
                    .spawn((
                        Node {
                            width: Val::Px(180.0),
                            height: Val::Auto,
                            position_type: PositionType::Absolute,
                            right: Val::Px(10.0),
                            top: Val::Px(10.0),
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(10.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                        BorderRadius::all(Val::Px(5.0)),
                        InventoryContextMenu,
                        ZIndex(100),
                    ))
                    .id();

                commands.entity(new_menu).with_children(|parent| {
                    parent
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(30.0),
                            margin: UiRect::bottom(Val::Px(10.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .with_children(|title| {
                            title.spawn((
                                Text::new("Select a key (0-9)"),
                                TextFont {
                                    font_size: 16.0,
                                    ..Default::default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                    parent
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(30.0),
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            margin: UiRect::bottom(Val::Px(5.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            for i in 0..5 {
                                row.spawn((
                                    Node {
                                        width: Val::Px(25.0),
                                        height: Val::Px(25.0),
                                        margin: UiRect::horizontal(Val::Px(2.0)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    //BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                                    BorderRadius::all(Val::Px(3.0)),
                                    HotkeyButton {
                                        key: i,
                                        tile_type: hotkey_option.tile_type,
                                    },
                                    Interaction::default(),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new(format!("{}", i)),
                                        TextFont {
                                            font_size: 14.0,
                                            ..Default::default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                            }
                        });

                    parent
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(30.0),
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            for i in 5..10 {
                                row.spawn((
                                    Node {
                                        width: Val::Px(25.0),
                                        height: Val::Px(25.0),
                                        margin: UiRect::horizontal(Val::Px(2.0)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    //BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                                    BorderRadius::all(Val::Px(3.0)),
                                    HotkeyButton {
                                        key: i,
                                        tile_type: hotkey_option.tile_type,
                                    },
                                    Interaction::default(),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new(format!("{}", i)),
                                        TextFont {
                                            font_size: 14.0,
                                            ..Default::default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                            }
                        });
                });
            }
        }
    }
    for (interaction, sell_option) in sell_interaction_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok(entity) = context_menu_query.single() {
                if world.resources.get(&sell_option.tile_type) >= Some(&1) {
                    world.money += get_tile_price(sell_option.tile_type);
                    *world
                        .resources
                        .entry(sell_option.tile_type)
                        .or_insert(0_u32) -= 1;
                    commands.entity(entity).despawn();
                }
            }
        }
    }
    if mouse_input.just_pressed(MouseButton::Left)
        && hotkey_interaction_query.is_empty()
        && sell_interaction_query.is_empty()
        && hotkey_button_interaction_query.is_empty()
    {
        if let Ok(entity) = context_menu_query.single() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn handle_hotkey_assignment(
    mut commands: Commands,
    interaction_query: Query<(&Interaction, &HotkeyButton), Changed<Interaction>>,
    context_menu_query: Query<Entity, With<InventoryContextMenu>>,
    mut hotkeys: ResMut<Hotkeys>,
) {
    for (interaction, hotkey_button) in interaction_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            hotkeys
                .mappings
                .insert(hotkey_button.key, hotkey_button.tile_type);

            for entity in context_menu_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}
