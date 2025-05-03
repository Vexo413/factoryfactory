use crate::{components::*, resources::*};
use bevy::prelude::*;

pub fn handle_context_menu(
    mut commands: Commands,
    interaction_query: Query<
        (&Interaction, &HotkeyOption),
        (Changed<Interaction>, With<HotkeyOption>),
    >,
    context_menu_query: Query<Entity, With<ContextMenu>>,
) {
    for (interaction, hotkey_option) in interaction_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Some(menu_entity) = context_menu_query.iter().next() {
                commands.entity(menu_entity).despawn();

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
                        ContextMenu,
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
                                    Button,
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
                                    Button,
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
}

pub fn handle_hotkey_assignment(
    mut commands: Commands,
    interaction_query: Query<(&Interaction, &HotkeyButton), Changed<Interaction>>,
    context_menu_query: Query<Entity, With<ContextMenu>>,
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
