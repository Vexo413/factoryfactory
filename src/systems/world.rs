use crate::{Hotkeys, tiles::*, utils::*};
use bevy::prelude::*;
use noise::*;
use std::collections::HashSet;

use crate::{
    Action, CHUNK_SIZE, ChunkPosition, Direction, ELECTRINE_DENSITY, ELECTRINE_NOISE_SCALE,
    FLEXTORIUM_DENSITY, FLEXTORIUM_NOISE_SCALE, IMAGE_SIZE, Placer, Position, RIGTORIUM_DENSITY,
    RIGTORIUM_NOISE_SCALE, TERRAIN_BASE_THRESHOLD, TILE_SIZE, TerrainChunk, TerrainTileType,
    WorldRes,
};

pub fn manage_terrain_chunks(
    mut commands: Commands,
    mut world: ResMut<WorldRes>,
    placer: Res<Placer>,
    camera_query: Query<&Transform, With<Camera2d>>,
    chunk_query: Query<(Entity, &TerrainChunk)>,
    asset_server: Res<AssetServer>,
) {
    if let Ok(camera_transform) = camera_query.single() {
        let camera_pos = camera_transform.translation.truncate();

        let base_chunk_radius = 2;
        let zoom_factor = 1.0 / placer.zoom_level;

        let chunks_radius = (base_chunk_radius as f32 * zoom_factor).ceil() as i32;

        let camera_chunk = ChunkPosition::from_world_position(camera_pos);

        let mut visible_chunks = HashSet::new();
        for x in (camera_chunk.x - chunks_radius)..(camera_chunk.x + chunks_radius + 1) {
            for y in (camera_chunk.y - chunks_radius)..(camera_chunk.y + chunks_radius + 1) {
                visible_chunks.insert(ChunkPosition::new(x, y));
            }
        }

        let mut chunks_to_unload = HashSet::new();
        for &loaded_chunk in &world.loaded_chunks {
            if !visible_chunks.contains(&loaded_chunk) {
                chunks_to_unload.insert(loaded_chunk);
            }
        }

        for chunk_pos in &chunks_to_unload {
            for (entity, chunk) in &chunk_query {
                if chunk.position == *chunk_pos {
                    commands.entity(entity).despawn();
                    break;
                }
            }
            world.loaded_chunks.remove(chunk_pos);
        }

        for chunk_pos in &visible_chunks {
            if !world.loaded_chunks.contains(chunk_pos) {
                generate_chunk(&mut commands, &mut world, *chunk_pos, &asset_server);
                world.loaded_chunks.insert(*chunk_pos);
            }
        }
    }
}

fn generate_chunk(
    commands: &mut Commands,
    world: &mut WorldRes,
    chunk_pos: ChunkPosition,
    asset_server: &AssetServer,
) {
    let chunk_entity = commands
        .spawn((
            TerrainChunk {
                position: chunk_pos,
            },
            Visibility::Visible,
            Transform::from_translation(Vec3::new(
                chunk_pos.x as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
                chunk_pos.y as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
                0.0,
            )),
        ))
        .id();

    let seed = world.world_seed;
    let rigtorium_noise = Perlin::new(seed);
    let flextorium_noise = Perlin::new(seed.wrapping_add(1));
    let electrine_noise = Perlin::new(seed.wrapping_add(2));

    commands.entity(chunk_entity).with_children(|parent| {
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let world_x = chunk_pos.x * CHUNK_SIZE + x;
                let world_y = chunk_pos.y * CHUNK_SIZE + y;
                let pos = Position::new(world_x, world_y);

                let rigtorium_val = rigtorium_noise.get([
                    world_x as f64 * RIGTORIUM_NOISE_SCALE,
                    world_y as f64 * RIGTORIUM_NOISE_SCALE,
                ]) + RIGTORIUM_DENSITY;

                let flextorium_val = flextorium_noise.get([
                    world_x as f64 * FLEXTORIUM_NOISE_SCALE,
                    world_y as f64 * FLEXTORIUM_NOISE_SCALE,
                ]) + FLEXTORIUM_DENSITY;

                let electrine_val = electrine_noise.get([
                    world_x as f64 * ELECTRINE_NOISE_SCALE,
                    world_y as f64 * ELECTRINE_NOISE_SCALE,
                ]) + ELECTRINE_DENSITY;

                let terrain_type = if rigtorium_val > TERRAIN_BASE_THRESHOLD
                    && rigtorium_val > flextorium_val
                    && rigtorium_val > electrine_val
                {
                    TerrainTileType::RawRigtoriumDeposit
                } else if flextorium_val > TERRAIN_BASE_THRESHOLD
                    && flextorium_val > rigtorium_val
                    && flextorium_val > electrine_val
                {
                    TerrainTileType::RawFlextoriumDeposit
                } else if electrine_val > TERRAIN_BASE_THRESHOLD
                    && electrine_val > rigtorium_val
                    && electrine_val > flextorium_val
                {
                    TerrainTileType::ElectrineDeposit
                } else {
                    TerrainTileType::Stone
                };

                world.terrain.insert(pos, terrain_type);

                let texture_path = match terrain_type {
                    TerrainTileType::Stone => "embedded://textures/terrain/stone.png",
                    TerrainTileType::RawFlextoriumDeposit => {
                        "embedded://textures/terrain/flextorium.png"
                    }
                    TerrainTileType::RawRigtoriumDeposit => {
                        "embedded://textures/terrain/rigtorium.png"
                    }
                    TerrainTileType::ElectrineDeposit => {
                        "embedded://textures/terrain/electrine.png"
                    }
                };

                parent.spawn((
                    Sprite::from_image(asset_server.load(texture_path)),
                    Transform {
                        translation: Vec3::new(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE, -1.0),
                        scale: Vec3::splat(TILE_SIZE / IMAGE_SIZE),
                        ..Default::default()
                    },
                ));
            }
        }
    });
}

pub fn tick_tiles(time: Res<Time>, mut world: ResMut<WorldRes>, hotkeys: Res<Hotkeys>) {
    world.tick_timer.tick(time.delta());
    if world.tick_timer.finished() {
        world.tick_count += 1;

        for action in world.actions.clone() {
            match action {
                Action::Move(start, end, item) => {
                    let mut empty = false;
                    let mut special = true;
                    if let Some(tile) = world.tiles.get_mut(&end) {
                        empty = tile.0.get_item().is_none();
                        if empty {
                            special = tile.0.as_any().is::<Factory>()
                                || tile.0.as_any().is::<Junction>()
                                || tile.0.as_any().is::<Extractor>();
                            if !special {
                                tile.0.set_item(Some(item));
                            } else if let Some(factory) =
                                tile.0.as_any_mut().downcast_mut::<Factory>()
                            {
                                if factory.factory_type.capacity().get(&item).unwrap_or(&0_u32)
                                    > factory.inventory.get(&item).unwrap_or(&0_u32)
                                {
                                    *factory.inventory.entry(item).or_insert(0) += 1;
                                    if let Some(start_tile) = world.tiles.get_mut(&start) {
                                        start_tile.0.set_item(None);
                                        if let Some(start_junction) =
                                            start_tile.0.as_any_mut().downcast_mut::<Junction>()
                                        {
                                            if start.x != end.x {
                                                start_junction.horizontal_item = None;
                                            } else if start.y != end.y {
                                                start_junction.vertical_item = None;
                                            }
                                        } else if let Some(start_storage) =
                                            start_tile.0.as_any_mut().downcast_mut::<Storage>()
                                        {
                                            start_storage.inventory -= 1;
                                        }
                                    }
                                }
                            } else if let Some(end_junction) =
                                tile.0.as_any_mut().downcast_mut::<Junction>()
                            {
                                if end.y == start.y {
                                    let input_direction = if end.x > start.x {
                                        Direction::Left
                                    } else {
                                        Direction::Right
                                    };
                                    if end_junction.horizontal_item.is_none() {
                                        end_junction.horizontal_item =
                                            Some((item, input_direction));
                                        if let Some(tile) = world.tiles.get_mut(&start) {
                                            tile.0.set_item(None);
                                            if let Some(start_junction) =
                                                tile.0.as_any_mut().downcast_mut::<Junction>()
                                            {
                                                if start.x != end.x {
                                                    start_junction.horizontal_item = None;
                                                } else if start.y != end.y {
                                                    start_junction.vertical_item = None;
                                                }
                                            } else if let Some(start_storage) =
                                                tile.0.as_any_mut().downcast_mut::<Storage>()
                                            {
                                                start_storage.inventory -= 1;
                                            }
                                        }
                                    }
                                } else {
                                    let input_direction = if end.y > start.y {
                                        Direction::Down
                                    } else {
                                        Direction::Up
                                    };
                                    if end_junction.vertical_item.is_none() {
                                        end_junction.vertical_item = Some((item, input_direction));
                                        if let Some(tile) = world.tiles.get_mut(&start) {
                                            tile.0.set_item(None);
                                            if let Some(start_junction) =
                                                tile.0.as_any_mut().downcast_mut::<Junction>()
                                            {
                                                if start.x != end.x {
                                                    start_junction.horizontal_item = None;
                                                } else if start.y != end.y {
                                                    start_junction.vertical_item = None;
                                                }
                                            } else if let Some(start_storage) =
                                                tile.0.as_any_mut().downcast_mut::<Storage>()
                                            {
                                                start_storage.inventory -= 1;
                                            }
                                        }
                                    }
                                }
                            } else if let Some(end_storage) =
                                tile.0.as_any_mut().downcast_mut::<Storage>()
                            {
                                if end_storage.storage_type.capacity() > end_storage.inventory {
                                    end_storage.inventory += 1;
                                    if let Some(start_tile) = world.tiles.get_mut(&start) {
                                        start_tile.0.set_item(None);
                                        if let Some(start_junction) =
                                            start_tile.0.as_any_mut().downcast_mut::<Junction>()
                                        {
                                            if start.x != end.x {
                                                start_junction.horizontal_item = None;
                                            } else if start.y != end.y {
                                                start_junction.vertical_item = None;
                                            }
                                        } else if let Some(start_storage) =
                                            start_tile.0.as_any_mut().downcast_mut::<Storage>()
                                        {
                                            start_storage.inventory -= 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(start_tile) = world.tiles.get_mut(&start) {
                        if empty && !special {
                            start_tile.0.set_item(None);

                            if let Some(start_junction) =
                                start_tile.0.as_any_mut().downcast_mut::<Junction>()
                            {
                                if start.x != end.x {
                                    start_junction.horizontal_item = None;
                                } else if start.y != end.y {
                                    start_junction.vertical_item = None;
                                }
                            } else if let Some(start_storage) =
                                start_tile.0.as_any_mut().downcast_mut::<Storage>()
                            {
                                start_storage.inventory -= 1;
                            }
                        }
                    }
                }
                Action::MoveRouter(start, end, item, _last_output) => {
                    let mut empty = false;
                    let mut special = true;
                    if let Some(tile) = world.tiles.get_mut(&end) {
                        empty = tile.0.get_item().is_none();
                        if empty {
                            special = tile.0.as_any().is::<Factory>()
                                || tile.0.as_any().is::<Junction>()
                                || tile.0.as_any().is::<Extractor>();
                            if !special {
                                tile.0.set_item(Some(item));
                            } else if let Some(factory) =
                                tile.0.as_any_mut().downcast_mut::<Factory>()
                            {
                                if factory.factory_type.capacity().get(&item).unwrap_or(&0)
                                    > factory.inventory.get(&item).unwrap_or(&0)
                                {
                                    *factory.inventory.entry(item).or_insert(0) += 1;
                                }
                            } else if let Some(end_junction) =
                                tile.0.as_any_mut().downcast_mut::<Junction>()
                            {
                                if end.y == start.y {
                                    let input_direction = if end.x > start.x {
                                        Direction::Left
                                    } else {
                                        Direction::Right
                                    };
                                    if end_junction.horizontal_item.is_none() {
                                        end_junction.horizontal_item =
                                            Some((item, input_direction));
                                    }
                                } else {
                                    let input_direction = if end.y > start.y {
                                        Direction::Down
                                    } else {
                                        Direction::Up
                                    };
                                    if end_junction.vertical_item.is_none() {
                                        end_junction.vertical_item = Some((item, input_direction));
                                    }
                                }
                            }
                        }
                    }
                    if let Some(start_tile) = world.tiles.get_mut(&start) {
                        if empty && !special {
                            if let Some(start_router) =
                                start_tile.0.as_any_mut().downcast_mut::<Router>()
                            {
                                start_router.item = None;
                                start_router.last_output = start_router.last_output.next();
                            }
                        }
                    }
                }
                Action::Produce(position) => {
                    let new_item = if let Some(tile) = world.tiles.get_mut(&position) {
                        if let Some(factory) = tile.0.as_any_mut().downcast_mut::<Factory>() {
                            Some(factory.factory_type.recipe().output)
                        } else if let Some(extractor) =
                            tile.0.as_any_mut().downcast_mut::<Extractor>()
                        {
                            Some(extractor.extractor_type.spawn_item())
                        } else {
                            return;
                        }
                    } else {
                        None
                    };
                    let direction = if let Some(tile) = world.tiles.get_mut(&position) {
                        if let Some(factory) = tile.0.as_any_mut().downcast_mut::<Factory>() {
                            Some(factory.direction)
                        } else if let Some(extractor) =
                            tile.0.as_any_mut().downcast_mut::<Extractor>()
                        {
                            Some(extractor.direction)
                        } else {
                            return;
                        }
                    } else {
                        None
                    };

                    if let Some(unwraped_item) = new_item {
                        let move_item;
                        if let Some(tile) = world.tiles.get_mut(&position) {
                            if let Some(factory) = tile.0.as_any_mut().downcast_mut::<Factory>() {
                                if factory.ticks >= factory.interval {
                                    factory.produce();
                                    factory.ticks = 0;
                                    factory.item = Some(unwraped_item);
                                    move_item = true;
                                } else {
                                    factory.ticks += 1;
                                    move_item = false;
                                }
                            } else if let Some(extractor) =
                                tile.0.as_any_mut().downcast_mut::<Extractor>()
                            {
                                extractor.item = Some(unwraped_item);
                                move_item = true;
                            } else {
                                move_item = false;
                            }
                        } else {
                            move_item = false;
                        }
                        if move_item {
                            let mut dest_pos = position;
                            if let Some(unwraped_direction) = direction {
                                match unwraped_direction {
                                    Direction::Up => dest_pos.y += 1,
                                    Direction::Down => dest_pos.y -= 1,
                                    Direction::Left => dest_pos.x -= 1,
                                    Direction::Right => dest_pos.x += 1,
                                }
                            }

                            let mut empty = false;
                            let mut special = true;
                            if let Some(tile) = world.tiles.get_mut(&dest_pos) {
                                empty = tile.0.get_item().is_none();
                                if empty {
                                    special = tile.0.as_any().is::<Factory>()
                                        || tile.0.as_any().is::<Junction>()
                                        || tile.0.as_any().is::<Extractor>();
                                    if !special {
                                        tile.0.set_item(Some(unwraped_item));
                                    } else if let Some(factory) =
                                        tile.0.as_any_mut().downcast_mut::<Factory>()
                                    {
                                        if factory
                                            .factory_type
                                            .capacity()
                                            .get(&unwraped_item)
                                            .unwrap_or(&0_u32)
                                            > factory
                                                .inventory
                                                .get(&unwraped_item)
                                                .unwrap_or(&0_u32)
                                        {
                                            *factory.inventory.entry(unwraped_item).or_insert(0) +=
                                                1;
                                            if let Some(start_tile) = world.tiles.get_mut(&position)
                                            {
                                                start_tile.0.set_item(None);
                                            }
                                        }
                                    } else if let Some(end_junction) =
                                        tile.0.as_any_mut().downcast_mut::<Junction>()
                                    {
                                        if dest_pos.y == position.y {
                                            let input_direction = if dest_pos.x > position.x {
                                                Direction::Left
                                            } else {
                                                Direction::Right
                                            };
                                            if end_junction.horizontal_item.is_none() {
                                                end_junction.horizontal_item =
                                                    Some((unwraped_item, input_direction));
                                                if let Some(tile) = world.tiles.get_mut(&position) {
                                                    tile.0.set_item(None);
                                                    if let Some(start_junction) = tile
                                                        .0
                                                        .as_any_mut()
                                                        .downcast_mut::<Junction>()
                                                    {
                                                        if position.x != dest_pos.x {
                                                            start_junction.horizontal_item = None;
                                                        } else if position.y != dest_pos.y {
                                                            start_junction.vertical_item = None;
                                                        }
                                                    } else if let Some(start_storage) = tile
                                                        .0
                                                        .as_any_mut()
                                                        .downcast_mut::<Storage>()
                                                    {
                                                        start_storage.inventory -= 1;
                                                    }
                                                }
                                            }
                                        } else {
                                            let input_direction = if dest_pos.y > position.y {
                                                Direction::Down
                                            } else {
                                                Direction::Up
                                            };
                                            if end_junction.vertical_item.is_none() {
                                                end_junction.vertical_item =
                                                    Some((unwraped_item, input_direction));
                                                if let Some(tile) = world.tiles.get_mut(&position) {
                                                    tile.0.set_item(None);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(start_tile) = world.tiles.get_mut(&position) {
                                if empty && !special {
                                    start_tile.0.set_item(None);
                                }
                            }
                        }
                    }
                }
                Action::Teleport(position, tile) => {
                    if let Some(tiles) = world.tiles.get_mut(&position) {
                        if let Some(portal) = tiles.0.as_any_mut().downcast_mut::<Portal>() {
                            portal.item = None;

                            *world.resources.entry(tile).or_insert(0) += 1;
                        } else if let Some(core) = tiles.0.as_any_mut().downcast_mut::<Core>() {
                            core.ticks = 0;

                            *world.resources.entry(tile).or_insert(0) += 1;
                        }
                    }
                }
                Action::IncreaseTicks(position) => {
                    if let Some(tiles) = world.tiles.get_mut(&position) {
                        if let Some(core) = tiles.0.as_any_mut().downcast_mut::<Core>() {
                            core.ticks += 1;
                        }
                    }
                }
            }
        }

        let mut next = Vec::new();

        for tile in world.tiles.values() {
            if let Some(action) = tile.0.tick(&world) {
                next.push(action);
            }
        }

        world.actions = sort_moves_topologically(next, &world);
        world.actions.reverse();

        if let Err(err) = world.save("savegame.ffs", &hotkeys) {
            eprintln!("Error saving game: {}", err);
        }
    }
}
