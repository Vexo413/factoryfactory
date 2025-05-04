use std::collections::HashSet;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::{components::*, constants::*, resources::*, tiles::*, types::*, utils::*};
use bevy::color::palettes::css;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub fn manage_tiles(
    windows: Query<&mut Window, With<PrimaryWindow>>,
    mut camera_query: Query<(&Camera, &mut Transform)>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut placer: ResMut<Placer>,
    mut world: ResMut<WorldRes>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    hotkeys: Res<Hotkeys>,
    core_menu_query: Query<(), With<CoreMenu>>,
    inventory_query: Query<Entity, With<Inventory>>,
) {
    if inventory_query.is_empty() && core_menu_query.is_empty() {
        if keyboard_input.just_pressed(KeyCode::Digit0) {
            if let Some(&tile_type) = hotkeys.mappings.get(&0) {
                placer.tile_type = tile_type;
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit1) {
            if let Some(&tile_type) = hotkeys.mappings.get(&1) {
                placer.tile_type = tile_type;
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit2) {
            if let Some(&tile_type) = hotkeys.mappings.get(&2) {
                placer.tile_type = tile_type;
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit3) {
            if let Some(&tile_type) = hotkeys.mappings.get(&3) {
                placer.tile_type = tile_type;
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit4) {
            if let Some(&tile_type) = hotkeys.mappings.get(&4) {
                placer.tile_type = tile_type;
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit5) {
            if let Some(&tile_type) = hotkeys.mappings.get(&5) {
                placer.tile_type = tile_type;
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit6) {
            if let Some(&tile_type) = hotkeys.mappings.get(&6) {
                placer.tile_type = tile_type;
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit7) {
            if let Some(&tile_type) = hotkeys.mappings.get(&7) {
                placer.tile_type = tile_type;
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit8) {
            if let Some(&tile_type) = hotkeys.mappings.get(&8) {
                placer.tile_type = tile_type;
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit9) {
            if let Some(&tile_type) = hotkeys.mappings.get(&9) {
                placer.tile_type = tile_type;
            }
        }
    }

    for event in mouse_wheel_events.read() {
        if placer.tile_type == (0, 1) && inventory_query.is_empty() && core_menu_query.is_empty() {
            let zoom_delta = event.y * ZOOM_SPEED;
            placer.zoom_level = (placer.zoom_level + zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);

            if let Ok((_, mut transform)) = camera_query.single_mut() {
                transform.scale = Vec3::splat(1.0 / placer.zoom_level);
            }
        } else {
            placer.direction = match (placer.direction, event.y.partial_cmp(&0.0)) {
                (Direction::Up, Some(std::cmp::Ordering::Less)) => Direction::Right,
                (Direction::Right, Some(std::cmp::Ordering::Less)) => Direction::Down,
                (Direction::Down, Some(std::cmp::Ordering::Less)) => Direction::Left,
                (Direction::Left, Some(std::cmp::Ordering::Less)) => Direction::Up,

                (Direction::Up, Some(std::cmp::Ordering::Greater)) => Direction::Left,
                (Direction::Left, Some(std::cmp::Ordering::Greater)) => Direction::Down,
                (Direction::Down, Some(std::cmp::Ordering::Greater)) => Direction::Right,
                (Direction::Right, Some(std::cmp::Ordering::Greater)) => Direction::Up,
                (current, _) => current,
            };
        }
    }

    let Ok(window) = windows.single() else {
        return;
    };
    if let Some(screen_pos) = window.cursor_position() {
        if let Ok((camera, camera_transform)) = camera_query.single() {
            if let Some(preview_entity) = placer.preview_entity {
                commands.entity(preview_entity).despawn();
            }
            if inventory_query.is_empty() && core_menu_query.is_empty() {
                let window_size = Vec2::new(window.width(), window.height());

                let mut ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
                ndc.y *= -1.0;
                let ndc_to_world =
                    camera_transform.compute_matrix() * camera.clip_from_view().inverse();
                let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
                let world_pos: Vec2 = world_pos.truncate();

                let grid_x = (world_pos.x / TILE_SIZE).round() as i32;
                let grid_y = (world_pos.y / TILE_SIZE).round() as i32;
                let pos = Position::new(grid_x, grid_y);

                let texture_path = get_tile_texture(placer.tile_type);

                let preview_entity = commands
                    .spawn((
                        Sprite {
                            image: asset_server.load(texture_path),
                            color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                            ..Default::default()
                        },
                        Transform {
                            translation: Vec3::new(
                                pos.x as f32 * TILE_SIZE,
                                pos.y as f32 * TILE_SIZE,
                                5.0,
                            ),
                            scale: Vec3::splat(TILE_SIZE / IMAGE_SIZE),
                            rotation: match placer.direction {
                                Direction::Up => Quat::IDENTITY,
                                Direction::Down => Quat::from_rotation_z(PI),
                                Direction::Left => Quat::from_rotation_z(FRAC_PI_2),
                                Direction::Right => Quat::from_rotation_z(-FRAC_PI_2),
                            },
                        },
                    ))
                    .id();

                placer.preview_entity = Some(preview_entity);
            } else {
                placer.preview_entity = None;
            }
        }
    }

    if mouse_button_input.pressed(MouseButton::Left)
        && inventory_query.is_empty()
        && core_menu_query.is_empty()
    {
        if let Ok(window) = windows.single() {
            if let Some(screen_pos) = window.cursor_position() {
                if let Ok((camera, camera_transform)) = camera_query.single() {
                    let window_size = Vec2::new(window.width(), window.height());

                    let mut ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
                    ndc.y *= -1.0;
                    let ndc_to_world =
                        camera_transform.compute_matrix() * camera.clip_from_view().inverse();
                    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
                    let world_pos: Vec2 = world_pos.truncate();

                    let grid_x = (world_pos.x / TILE_SIZE).round() as i32;
                    let grid_y = (world_pos.y / TILE_SIZE).round() as i32;
                    let pos = Position::new(grid_x, grid_y);
                    let tile_type = placer.tile_type;
                    let direction = placer.direction;
                    if pos != Position::new(0, 0) {
                        if world.tiles.contains_key(&pos) {
                            let current_tile_id =
                                world.tiles.get(&pos).map(|(_, id)| *id).unwrap_or((0, 1));

                            if *world.resources.get(&tile_type).unwrap_or(&0) >= 1
                                || placer.tile_type == current_tile_id
                            {
                                *world.resources.entry(current_tile_id).or_insert(0) += 1;
                                *world.resources.entry(tile_type).or_insert(0) -= 1;

                                let new_tile = get_new_tile(tile_type, pos, direction);

                                if let Some(entry) = world.tiles.get_mut(&pos) {
                                    *entry = new_tile;
                                    let new = world
                                        .actions
                                        .clone()
                                        .into_iter()
                                        .filter(|action| match action {
                                            Action::Move(position, _, _) => *position != pos,
                                            Action::Produce(position) => *position != pos,
                                            Action::MoveRouter(position, _, _, _) => {
                                                *position != pos
                                            }
                                            Action::Teleport(position, _) => *position != pos,
                                            Action::IncreaseTicks(position) => *position != pos,
                                        })
                                        .collect();

                                    world.actions = new;
                                }
                            }
                        } else {
                            if *world.resources.get(&tile_type).unwrap_or(&0) >= 1 {
                                *world.resources.entry(tile_type).or_insert(0) -= 1;

                                let new_tile = get_new_tile(tile_type, pos, direction);

                                world.tiles.insert(pos, new_tile);

                                let new = world
                                    .actions
                                    .clone()
                                    .into_iter()
                                    .filter(|action| match action {
                                        Action::Move(position, _, _) => *position != pos,
                                        Action::Produce(position) => *position != pos,
                                        Action::MoveRouter(position, _, _, _) => *position != pos,
                                        Action::Teleport(position, _) => *position != pos,
                                        Action::IncreaseTicks(position) => *position != pos,
                                    })
                                    .collect();

                                world.actions = new;

                                commands
                                    .spawn((
                                        Sprite::from_image(
                                            asset_server.load(get_tile_texture(tile_type)),
                                        ),
                                        Transform {
                                            translation: Vec3::new(
                                                pos.x as f32 * TILE_SIZE,
                                                pos.y as f32 * TILE_SIZE,
                                                0.0,
                                            ),
                                            scale: Vec3::splat(TILE_SIZE / IMAGE_SIZE),
                                            ..Default::default()
                                        },
                                        TileSprite { pos },
                                    ))
                                    .with_children(|parent| {
                                        parent.spawn((
                                            Sprite::from_image(
                                                asset_server
                                                    .load("embedded://textures/items/none.png"),
                                            ),
                                            Transform::from_scale(Vec3::splat(0.5)),
                                        ));
                                    });
                            }
                        }
                    } else {
                        if core_menu_query.is_empty() {
                            if let Some(tile) = world.tiles.get(&pos) {
                                if let Some(core) = tile.0.as_any().downcast_ref::<Core>() {
                                    commands.spawn((
                                        Node {
                                            width: Val::Vw(80.0),
                                            height: Val::Vh(80.0),
                                            position_type: PositionType::Absolute,
                                            left: Val::Vw(10.0),
                                            top: Val::Vh(10.0),
                                            display: Display::Flex,
                                            flex_direction: FlexDirection::Column,
                                            padding: UiRect::all(Val::Px(20.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.18, 0.2, 0.23)),
                                        BorderRadius::all(Val::Px(10.0)),
                                        CoreMenu {
                                            position: pos,
                                            selected_category: 1,
                                        },
                                        children![
                                            (
                                                Node {
                                                    width: Val::Percent(100.0),
                                                    height: Val::Px(40.0),
                                                    margin: UiRect::bottom(Val::Px(20.0)),
                                                    align_items: AlignItems::Center,
                                                    justify_content: JustifyContent::Center,
                                                    ..default()
                                                },
                                                children![(
                                                    Text::new("Core Configuration"),
                                                    TextFont {
                                                        font_size: 24.0,
                                                        ..Default::default()
                                                    },
                                                    TextColor(Color::WHITE)
                                                )],
                                            ),
                                            (
                                                Node {
                                                    width: Val::Percent(100.0),
                                                    height: Val::Px(60.0),
                                                    margin: UiRect::bottom(Val::Px(20.0)),
                                                    display: Display::Flex,
                                                    flex_direction: FlexDirection::Column,
                                                    ..default()
                                                },
                                                children![
                                                    (
                                                        Text::new(format!(
                                                            "Current production: {} ({})",
                                                            get_tile_name(core.tile_id),
                                                            format_tile_id(core.tile_id)
                                                        )),
                                                        TextFont {
                                                            font_size: 16.0,
                                                            ..Default::default()
                                                        },
                                                        TextColor(Color::WHITE),
                                                        Node {
                                                            margin: UiRect::bottom(Val::Px(10.0)),
                                                            ..Default::default()
                                                        }
                                                    ),
                                                    (
                                                        Text::new(format!(
                                                            "Progress: {}/{} seconds",
                                                            core.ticks, core.interval
                                                        )),
                                                        TextFont {
                                                            font_size: 16.0,
                                                            ..Default::default()
                                                        },
                                                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                                        Name::new("core_progress")
                                                    )
                                                ],
                                            ),
                                            (
                                                Node {
                                                    width: Val::Percent(100.0),
                                                    height: Val::Percent(100.0),
                                                    display: Display::Flex,
                                                    flex_direction: FlexDirection::Row,
                                                    ..Default::default()
                                                },
                                                children![
                                                    (
                                                        Node {
                                                            width: Val::Percent(25.0),
                                                            height: Val::Percent(100.0),
                                                            display: Display::Flex,
                                                            flex_direction: FlexDirection::Column,
                                                            padding: UiRect::all(Val::Px(10.0)),
                                                            row_gap: Val::Px(10.0),
                                                            ..Default::default()
                                                        },
                                                        BackgroundColor(Color::srgb(
                                                            0.14, 0.16, 0.19
                                                        )),
                                                        BorderRadius::all(Val::Px(10.0)),
                                                        children![
                                                            (
                                                                Button,
                                                                Node {
                                                                    width: Val::Percent(100.0),
                                                                    height: Val::Px(50.0),
                                                                    align_items: AlignItems::Center,
                                                                    justify_content:
                                                                        JustifyContent::Center,
                                                                    ..Default::default()
                                                                },
                                                                BackgroundColor(Color::srgb(
                                                                    0.3, 0.5, 0.7
                                                                )),
                                                                CoreCategory { category: 1 },
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
                                                                        justify:
                                                                            JustifyText::Center,
                                                                        ..Default::default()
                                                                    }
                                                                )],
                                                            ),
                                                            (
                                                                Button,
                                                                Node {
                                                                    width: Val::Percent(100.0),
                                                                    height: Val::Px(50.0),
                                                                    align_items: AlignItems::Center,
                                                                    justify_content:
                                                                        JustifyContent::Center,
                                                                    ..Default::default()
                                                                },
                                                                BackgroundColor(Color::srgb(
                                                                    0.2, 0.22, 0.25
                                                                )),
                                                                CoreCategory { category: 2 },
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
                                                                        justify:
                                                                            JustifyText::Center,
                                                                        ..Default::default()
                                                                    }
                                                                )],
                                                            ),
                                                            (
                                                                Button,
                                                                Node {
                                                                    width: Val::Percent(100.0),
                                                                    height: Val::Px(50.0),
                                                                    align_items: AlignItems::Center,
                                                                    justify_content:
                                                                        JustifyContent::Center,
                                                                    ..Default::default()
                                                                },
                                                                BackgroundColor(Color::srgb(
                                                                    0.2, 0.22, 0.25
                                                                )),
                                                                CoreCategory { category: 3 },
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
                                                                        justify:
                                                                            JustifyText::Center,
                                                                        ..Default::default()
                                                                    }
                                                                )],
                                                            ),
                                                            (
                                                                Button,
                                                                Node {
                                                                    width: Val::Percent(100.0),
                                                                    height: Val::Px(50.0),
                                                                    align_items: AlignItems::Center,
                                                                    justify_content:
                                                                        JustifyContent::Center,
                                                                    ..Default::default()
                                                                },
                                                                BackgroundColor(Color::srgb(
                                                                    0.2, 0.22, 0.25
                                                                )),
                                                                CoreCategory { category: 4 },
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
                                                                        justify:
                                                                            JustifyText::Center,
                                                                        ..Default::default()
                                                                    }
                                                                )],
                                                            ),
                                                            (
                                                                Button,
                                                                Node {
                                                                    width: Val::Percent(100.0),
                                                                    height: Val::Px(50.0),
                                                                    align_items: AlignItems::Center,
                                                                    justify_content:
                                                                        JustifyContent::Center,
                                                                    ..Default::default()
                                                                },
                                                                BackgroundColor(Color::srgb(
                                                                    0.2, 0.22, 0.25
                                                                )),
                                                                CoreCategory { category: 5 },
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
                                                                        justify:
                                                                            JustifyText::Center,
                                                                        ..Default::default()
                                                                    }
                                                                )],
                                                            ),
                                                        ],
                                                    ),
                                                    (
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
                                                        BackgroundColor(Color::srgb(
                                                            0.18, 0.2, 0.23
                                                        )),
                                                        CoreItemsPanel,
                                                    ),
                                                ],
                                            ),
                                            (
                                                Node {
                                                    width: Val::Percent(100.0),
                                                    height: Val::Px(40.0),
                                                    display: Display::Flex,
                                                    justify_content: JustifyContent::Center,
                                                    margin: UiRect::top(Val::Px(20.0)),
                                                    ..default()
                                                },
                                                children![(
                                                    Button,
                                                    Node {
                                                        width: Val::Px(120.0),
                                                        height: Val::Px(40.0),
                                                        align_content: AlignContent::Center,
                                                        justify_content: JustifyContent::Center,
                                                        display: Display::Grid,
                                                        ..default()
                                                    },
                                                    BackgroundColor(Color::srgb(0.6, 0.3, 0.3)),
                                                    BorderRadius::all(Val::Px(5.0)),
                                                    Interaction::default(),
                                                    Name::new("close_button"),
                                                    children![(
                                                        Text::new("Close"),
                                                        TextFont {
                                                            font_size: 16.0,
                                                            ..default()
                                                        },
                                                        TextColor(Color::WHITE),
                                                    )]
                                                )],
                                            )
                                        ],
                                    ));
                                } else {
                                    commands.spawn((Node::default(), Text::default()));
                                }
                            } else {
                                commands.spawn((Node::default(), Text::default()));
                            }
                        }
                    }
                }
            }
        }
    }
    if mouse_button_input.pressed(MouseButton::Right)
        && inventory_query.is_empty()
        && core_menu_query.is_empty()
    {
        placer.tile_type = (0, 1);
        if let Ok(window) = windows.single() {
            if let Some(screen_pos) = window.cursor_position() {
                if let Ok((camera, camera_transform)) = camera_query.single() {
                    let window_size = Vec2::new(window.width(), window.height());

                    let mut ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
                    ndc.y *= -1.0;
                    let ndc_to_world =
                        camera_transform.compute_matrix() * camera.clip_from_view().inverse();
                    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
                    let world_pos: Vec2 = world_pos.truncate();

                    let grid_x = (world_pos.x / TILE_SIZE).round() as i32;
                    let grid_y = (world_pos.y / TILE_SIZE).round() as i32;
                    let pos = Position::new(grid_x, grid_y);
                    if pos != Position::new(0, 0) {
                        if let Some(entry) = world.tiles.remove_entry(&pos) {
                            *world.resources.entry(entry.1.1).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }
}

pub fn update_tile_visuals(
    world: Res<WorldRes>,
    mut parent_query: Query<(Entity, &TileSprite, &mut Transform, &mut Sprite)>,
    children_query: Query<&Children, With<TileSprite>>,
    mut child_sprite_query: Query<(&mut Sprite, &mut Transform), Without<TileSprite>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    item_animation_query: Query<&ItemAnimation>,
) {
    let mut existing_positions = HashSet::new();
    let mut animated_positions = HashSet::new();

    for animation in item_animation_query.iter() {
        let start_pos = Position::new(
            (animation.start_pos.x / TILE_SIZE).round() as i32,
            (animation.start_pos.y / TILE_SIZE).round() as i32,
        );
        let end_pos = Position::new(
            (animation.end_pos.x / TILE_SIZE).round() as i32,
            (animation.end_pos.y / TILE_SIZE).round() as i32,
        );
        animated_positions.insert(start_pos);
        animated_positions.insert(end_pos);
    }

    for (entity, tile_sprite, mut transform, mut sprite) in parent_query.iter_mut() {
        transform.translation = Vec3::new(
            tile_sprite.pos.x as f32 * TILE_SIZE,
            tile_sprite.pos.y as f32 * TILE_SIZE,
            0.0,
        );

        if let Some(tile) = world.tiles.get(&tile_sprite.pos) {
            existing_positions.insert(tile_sprite.pos);
            if let Some(conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
                transform.translation = Vec3::new(
                    tile_sprite.pos.x as f32 * TILE_SIZE,
                    tile_sprite.pos.y as f32 * TILE_SIZE,
                    0.0,
                );

                let texture_path = determine_conveyor_texture(&world, conveyor);
                sprite.image = asset_server.load(texture_path);

                transform.rotation = match conveyor.direction {
                    Direction::Up => Quat::IDENTITY,
                    Direction::Down => Quat::from_rotation_z(PI),
                    Direction::Left => Quat::from_rotation_z(FRAC_PI_2),
                    Direction::Right => Quat::from_rotation_z(-FRAC_PI_2),
                };

                if let Ok(children) = children_query.get(entity) {
                    for child in children.iter() {
                        if let Ok((mut child_sprite, mut child_transform)) =
                            child_sprite_query.get_mut(child)
                        {
                            if animated_positions.contains(&tile_sprite.pos) {
                                child_sprite.color = Color::NONE;
                            } else {
                                child_sprite.color = Color::WHITE;
                            }
                            child_transform.translation = Vec3::new(0.0, 0.0, 1.0);
                            child_transform.rotation = match conveyor.direction {
                                Direction::Up => Quat::IDENTITY,
                                Direction::Down => Quat::from_rotation_z(PI),
                                Direction::Left => Quat::from_rotation_z(-FRAC_PI_2),
                                Direction::Right => Quat::from_rotation_z(FRAC_PI_2),
                            };

                            child_sprite.image = if let Some(unwraped_item) = conveyor.item {
                                asset_server.load(unwraped_item.sprite())
                            } else {
                                asset_server.load("embedded://textures/items/none.png")
                            }
                        }
                    }
                }
            } else if let Some(router) = tile.0.as_any().downcast_ref::<Router>() {
                transform.translation = Vec3::new(
                    tile_sprite.pos.x as f32 * TILE_SIZE,
                    tile_sprite.pos.y as f32 * TILE_SIZE,
                    2.0,
                );

                sprite.image = asset_server.load("embedded://textures/tiles/conveyors/router.png");

                transform.rotation = match router.direction {
                    Direction::Up => Quat::IDENTITY,
                    Direction::Down => Quat::from_rotation_z(PI),
                    Direction::Left => Quat::from_rotation_z(FRAC_PI_2),
                    Direction::Right => Quat::from_rotation_z(-FRAC_PI_2),
                };

                if let Ok(children) = children_query.get(entity) {
                    for child in children.iter() {
                        if let Ok((mut child_sprite, _)) = child_sprite_query.get_mut(child) {
                            child_sprite.color = Color::NONE;
                        }
                    }
                }
            } else if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
                transform.translation = Vec3::new(
                    tile_sprite.pos.x as f32 * TILE_SIZE,
                    tile_sprite.pos.y as f32 * TILE_SIZE,
                    2.0,
                );
                sprite.image = asset_server.load(
                    tile.0
                        .as_any()
                        .downcast_ref::<Factory>()
                        .unwrap()
                        .factory_type
                        .sprite(),
                );
                transform.rotation = match factory.direction {
                    Direction::Up => Quat::IDENTITY,
                    Direction::Down => Quat::from_rotation_z(PI),
                    Direction::Left => Quat::from_rotation_z(FRAC_PI_2),
                    Direction::Right => Quat::from_rotation_z(-FRAC_PI_2),
                };

                if let Ok(children) = children_query.get(entity) {
                    for child in children.iter() {
                        if let Ok((mut child_sprite, _)) = child_sprite_query.get_mut(child) {
                            child_sprite.color = Color::NONE;
                        }
                    }
                }
            } else if let Some(extractor) = tile.0.as_any().downcast_ref::<Extractor>() {
                transform.translation = Vec3::new(
                    tile_sprite.pos.x as f32 * TILE_SIZE,
                    tile_sprite.pos.y as f32 * TILE_SIZE,
                    2.0,
                );
                sprite.image = asset_server.load(extractor.extractor_type.sprite());

                transform.rotation = match extractor.direction {
                    Direction::Up => Quat::IDENTITY,
                    Direction::Down => Quat::from_rotation_z(PI),
                    Direction::Left => Quat::from_rotation_z(FRAC_PI_2),
                    Direction::Right => Quat::from_rotation_z(-FRAC_PI_2),
                };
            } else if tile.0.as_any().is::<Portal>() {
                transform.translation = Vec3::new(
                    tile_sprite.pos.x as f32 * TILE_SIZE,
                    tile_sprite.pos.y as f32 * TILE_SIZE,
                    2.0,
                );
                sprite.image = asset_server.load("embedded://textures/tiles/portal.png");
            } else if tile.0.as_any().is::<Junction>() {
                transform.translation = Vec3::new(
                    tile_sprite.pos.x as f32 * TILE_SIZE,
                    tile_sprite.pos.y as f32 * TILE_SIZE,
                    2.0,
                );
                sprite.image =
                    asset_server.load("embedded://textures/tiles/conveyors/junction.png");

                transform.rotation = Quat::IDENTITY;

                if let Ok(children) = children_query.get(entity) {
                    for child in children.iter() {
                        if let Ok((mut child_sprite, _)) = child_sprite_query.get_mut(child) {
                            child_sprite.color = Color::NONE;
                        }
                    }
                }
            } else if tile.0.as_any().is::<Core>() {
                transform.translation = Vec3::new(
                    tile_sprite.pos.x as f32 * TILE_SIZE,
                    tile_sprite.pos.y as f32 * TILE_SIZE,
                    2.0,
                );
                sprite.image = asset_server.load("embedded://textures/tiles/core.png");

                transform.rotation = Quat::IDENTITY;

                if let Ok(children) = children_query.get(entity) {
                    for child in children.iter() {
                        if let Ok((mut child_sprite, _)) = child_sprite_query.get_mut(child) {
                            child_sprite.color = Color::NONE;
                        }
                    }
                }
            } else {
                sprite.color = css::GRAY.into();
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
}
