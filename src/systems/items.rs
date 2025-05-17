use std::collections::HashSet;

use crate::{
    Action, Conveyor, Direction, Extractor, Factory, IMAGE_SIZE, ITEM_SIZE, Junction, Portal,
    Position, Router, TICK_LENGTH, TILE_SIZE, WorldRes, components::*,
};
use bevy::prelude::*;

pub fn animate_items(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut ItemAnimation, &mut Transform)>,
) {
    for (entity, mut animation, mut transform) in query.iter_mut() {
        animation.timer.tick(time.delta());
        let t = animation.timer.fraction();
        transform.translation = animation.start_pos.lerp(animation.end_pos, t);

        if animation.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn spawn_animations(
    mut commands: Commands,
    world: Res<WorldRes>,
    asset_server: Res<AssetServer>,
) {
    if world.tick_timer.finished() {
        let mut filled_positions: HashSet<Position> = HashSet::new();
        let mut empty_positions: HashSet<Position> = HashSet::new();

        for (pos, tile) in world.tiles.iter() {
            if let Some(conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
                if conveyor.item.is_none() {
                    empty_positions.insert(*pos);
                }
            } else if tile.0.as_any().is::<Factory>() {
                empty_positions.insert(*pos);
            } else if let Some(router) = tile.0.as_any().downcast_ref::<Router>() {
                if router.item.is_none() {
                    empty_positions.insert(*pos);
                }
            }
        }

        for action in &world.actions {
            match action {
                Action::Move(start, end, item) => {
                    if let Some(tile) = world.tiles.get(end) {
                        if tile.0.as_any().is::<Conveyor>() {
                            if !filled_positions.contains(end) && empty_positions.contains(end) {
                                filled_positions.insert(*end);
                                empty_positions.remove(end);

                                filled_positions.remove(start);
                                empty_positions.insert(*start);

                                let start_pos = Vec3::new(
                                    start.x as f32 * TILE_SIZE,
                                    start.y as f32 * TILE_SIZE,
                                    1.0,
                                );
                                let end_pos = Vec3::new(
                                    end.x as f32 * TILE_SIZE,
                                    end.y as f32 * TILE_SIZE,
                                    1.0,
                                );

                                commands.spawn((
                                    ItemAnimation {
                                        start_pos,
                                        end_pos,
                                        timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Once),
                                    },
                                    Sprite::from_image(asset_server.load(item.sprite())),
                                    Transform {
                                        translation: start_pos,
                                        scale: Vec3::splat(ITEM_SIZE / IMAGE_SIZE),
                                        ..Default::default()
                                    },
                                ));
                            }
                        } else if tile.0.as_any().is::<Router>() {
                            if !filled_positions.contains(end) && empty_positions.contains(end) {
                                filled_positions.insert(*end);
                                empty_positions.remove(end);

                                filled_positions.remove(start);
                                empty_positions.insert(*start);

                                let start_pos = Vec3::new(
                                    start.x as f32 * TILE_SIZE,
                                    start.y as f32 * TILE_SIZE,
                                    1.0,
                                );
                                let end_pos = Vec3::new(
                                    end.x as f32 * TILE_SIZE,
                                    end.y as f32 * TILE_SIZE,
                                    1.0,
                                );
                                commands.spawn((
                                    ItemAnimation {
                                        start_pos,
                                        end_pos,
                                        timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Once),
                                    },
                                    Sprite::from_image(asset_server.load(item.sprite())),
                                    Transform {
                                        translation: start_pos,
                                        scale: Vec3::splat(ITEM_SIZE / IMAGE_SIZE),
                                        ..Default::default()
                                    },
                                ));
                            }
                        } else if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
                            if factory.factory_type.capacity().get(item).unwrap_or(&0_u32)
                                > factory.inventory.get(item).unwrap_or(&0_u32)
                            {
                                filled_positions.remove(start);
                                empty_positions.insert(*start);

                                let start_pos = Vec3::new(
                                    start.x as f32 * TILE_SIZE,
                                    start.y as f32 * TILE_SIZE,
                                    1.0,
                                );
                                let end_pos = Vec3::new(
                                    end.x as f32 * TILE_SIZE,
                                    end.y as f32 * TILE_SIZE,
                                    1.0,
                                );
                                commands.spawn((
                                    ItemAnimation {
                                        start_pos,
                                        end_pos,
                                        timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Once),
                                    },
                                    Sprite::from_image(asset_server.load(item.sprite())),
                                    Transform {
                                        translation: start_pos,
                                        scale: Vec3::splat(ITEM_SIZE / IMAGE_SIZE),
                                        ..Default::default()
                                    },
                                ));
                            }
                        } else if let Some(end_portal) = tile.0.as_any().downcast_ref::<Portal>() {
                            if end_portal.item.is_none() {
                                filled_positions.insert(*end);
                                empty_positions.remove(end);

                                filled_positions.remove(start);
                                empty_positions.insert(*start);

                                let start_pos = Vec3::new(
                                    start.x as f32 * TILE_SIZE,
                                    start.y as f32 * TILE_SIZE,
                                    1.0,
                                );
                                let end_pos = Vec3::new(
                                    end.x as f32 * TILE_SIZE,
                                    end.y as f32 * TILE_SIZE,
                                    1.0,
                                );
                                commands.spawn((
                                    ItemAnimation {
                                        start_pos,
                                        end_pos,
                                        timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Once),
                                    },
                                    Sprite::from_image(asset_server.load(item.sprite())),
                                    Transform {
                                        translation: start_pos,
                                        scale: Vec3::splat(ITEM_SIZE / IMAGE_SIZE),
                                        ..Default::default()
                                    },
                                ));
                            }
                        } else if let Some(junction) = tile.0.as_any().downcast_ref::<Junction>() {
                            let is_horizontal_movement = start.y == end.y;
                            let can_accept = if is_horizontal_movement {
                                junction.horizontal_item.is_none()
                            } else {
                                junction.vertical_item.is_none()
                            };

                            if can_accept {
                                filled_positions.insert(*end);

                                if is_horizontal_movement {
                                    filled_positions.remove(start);
                                } else {
                                    filled_positions.remove(start);
                                }

                                let start_pos = Vec3::new(
                                    start.x as f32 * TILE_SIZE,
                                    start.y as f32 * TILE_SIZE,
                                    1.0,
                                );
                                let end_pos = Vec3::new(
                                    end.x as f32 * TILE_SIZE,
                                    end.y as f32 * TILE_SIZE,
                                    1.0,
                                );
                                commands.spawn((
                                    ItemAnimation {
                                        start_pos,
                                        end_pos,
                                        timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Once),
                                    },
                                    Sprite::from_image(asset_server.load(item.sprite())),
                                    Transform {
                                        translation: start_pos,
                                        scale: Vec3::splat(ITEM_SIZE / IMAGE_SIZE),
                                        ..Default::default()
                                    },
                                ));
                            }
                        }
                    }
                }
                Action::MoveRouter(start, end, item, _last_output) => {
                    if let Some(tile) = world.tiles.get(end) {
                        let can_accept = if tile.0.as_any().is::<Conveyor>() {
                            !filled_positions.contains(end) && empty_positions.contains(end)
                        } else if tile.0.as_any().is::<Router>() {
                            !filled_positions.contains(end) && empty_positions.contains(end)
                        } else if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
                            factory.factory_type.capacity().get(item).unwrap_or(&0_u32)
                                > factory.inventory.get(item).unwrap_or(&0_u32)
                        } else if let Some(end_portal) = tile.0.as_any().downcast_ref::<Portal>() {
                            end_portal.item.is_none()
                        } else {
                            false
                        };

                        if can_accept {
                            filled_positions.insert(*end);
                            empty_positions.remove(end);

                            filled_positions.remove(start);
                            empty_positions.insert(*start);
                            let start_pos = Vec3::new(
                                start.x as f32 * TILE_SIZE,
                                start.y as f32 * TILE_SIZE,
                                1.0,
                            );
                            let end_pos =
                                Vec3::new(end.x as f32 * TILE_SIZE, end.y as f32 * TILE_SIZE, 1.0);
                            commands.spawn((
                                ItemAnimation {
                                    start_pos,
                                    end_pos,
                                    timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Once),
                                },
                                Sprite::from_image(asset_server.load(item.sprite())),
                                Transform {
                                    translation: start_pos,
                                    scale: Vec3::splat(ITEM_SIZE / IMAGE_SIZE),
                                    ..Default::default()
                                },
                            ));
                        }
                    }
                }
                Action::Produce(position) => {
                    let can_produce_and_move = {
                        if let Some(tile) = world.tiles.get(position) {
                            if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
                                if factory.can_produce()
                                    && factory.item.is_none()
                                    && factory.ticks >= factory.interval
                                {
                                    let mut dest_pos = *position;
                                    match factory.direction {
                                        Direction::Up => dest_pos.y += 1,
                                        Direction::Down => dest_pos.y -= 1,
                                        Direction::Left => dest_pos.x -= 1,
                                        Direction::Right => dest_pos.x += 1,
                                    }

                                    if let Some(dest_tile) = world.tiles.get(&dest_pos) {
                                        let output_item = factory.factory_type.recipe().output;
                                        let can_accept = if dest_tile.0.as_any().is::<Conveyor>() {
                                            empty_positions.contains(&dest_pos)
                                                && !filled_positions.contains(&dest_pos)
                                        } else if dest_tile.0.as_any().is::<Router>() {
                                            empty_positions.contains(&dest_pos)
                                                && !filled_positions.contains(&dest_pos)
                                        } else if let Some(dest_factory) =
                                            dest_tile.0.as_any().downcast_ref::<Factory>()
                                        {
                                            dest_factory
                                                .factory_type
                                                .capacity()
                                                .get(&output_item)
                                                .unwrap_or(&0)
                                                > dest_factory
                                                    .inventory
                                                    .get(&output_item)
                                                    .unwrap_or(&0)
                                        } else if let Some(portal) =
                                            dest_tile.0.as_any().downcast_ref::<Portal>()
                                        {
                                            portal.item.is_none()
                                        } else {
                                            false
                                        };

                                        if can_accept {
                                            Some((
                                                factory.factory_type.recipe().output,
                                                *position,
                                                dest_pos,
                                            ))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else if let Some(extractor) =
                                tile.0.as_any().downcast_ref::<Extractor>()
                            {
                                if extractor.item.is_none()
                                    && world.tick_count % extractor.extractor_type.interval() == 0
                                    && world.terrain.get(position)
                                        == Some(&extractor.extractor_type.terrain())
                                {
                                    let mut dest_pos = *position;
                                    match extractor.direction {
                                        Direction::Up => dest_pos.y += 1,
                                        Direction::Down => dest_pos.y -= 1,
                                        Direction::Left => dest_pos.x -= 1,
                                        Direction::Right => dest_pos.x += 1,
                                    }

                                    if let Some(dest_tile) = world.tiles.get(&dest_pos) {
                                        let output_item = extractor.extractor_type.spawn_item();
                                        let can_accept = if dest_tile.0.as_any().is::<Conveyor>() {
                                            empty_positions.contains(&dest_pos)
                                                && !filled_positions.contains(&dest_pos)
                                        } else if dest_tile.0.as_any().is::<Router>() {
                                            empty_positions.contains(&dest_pos)
                                                && !filled_positions.contains(&dest_pos)
                                        } else if let Some(factory) =
                                            dest_tile.0.as_any().downcast_ref::<Factory>()
                                        {
                                            factory
                                                .factory_type
                                                .capacity()
                                                .get(&output_item)
                                                .unwrap_or(&0)
                                                > factory.inventory.get(&output_item).unwrap_or(&0)
                                        } else if let Some(portal) =
                                            dest_tile.0.as_any().downcast_ref::<Portal>()
                                        {
                                            portal.item.is_none()
                                        } else {
                                            false
                                        };

                                        if can_accept {
                                            Some((
                                                extractor.extractor_type.spawn_item(),
                                                *position,
                                                dest_pos,
                                            ))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some((item, source_pos, dest_pos)) = can_produce_and_move {
                        filled_positions.insert(dest_pos);
                        empty_positions.remove(&dest_pos);

                        let start_pos = Vec3::new(
                            source_pos.x as f32 * TILE_SIZE,
                            source_pos.y as f32 * TILE_SIZE,
                            1.0,
                        );
                        let end_pos = Vec3::new(
                            dest_pos.x as f32 * TILE_SIZE,
                            dest_pos.y as f32 * TILE_SIZE,
                            1.0,
                        );

                        commands.spawn((
                            ItemAnimation {
                                start_pos,
                                end_pos,
                                timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Once),
                            },
                            Sprite::from_image(asset_server.load(item.sprite())),
                            Transform {
                                translation: start_pos,
                                scale: Vec3::splat(ITEM_SIZE / IMAGE_SIZE),
                                ..Default::default()
                            },
                        ));
                    }
                }

                _ => {}
            }
        }
    }
}
