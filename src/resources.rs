use bevy::prelude::*;
use bincode::{Decode, Encode, config};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use noise::{NoiseFn, Perlin};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::extractor::ExtractorType;
use crate::factory::FactoryType;
use crate::router::RouterOutputIndex;
use crate::storage::StorageType;
use crate::tiles::Tile;
use crate::{Conveyor, Extractor, Factory, Junction, Portal, Router, Storage, types::*};
use crate::{Core, constants::*};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPosition {
    pub x: i32,
    pub y: i32,
}

impl ChunkPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn from_world_position(world_pos: Vec2) -> Self {
        Self {
            x: (world_pos.x / (CHUNK_SIZE as f32 * TILE_SIZE)).floor() as i32,
            y: (world_pos.y / (CHUNK_SIZE as f32 * TILE_SIZE)).floor() as i32,
        }
    }
}

#[derive(Resource)]
pub struct Placer {
    pub direction: Direction,
    pub tile_type: (u8, u8),
    pub preview_entity: Option<Entity>,
    pub zoom_level: f32,
}

impl Default for Placer {
    fn default() -> Self {
        Self {
            direction: Direction::Up,
            tile_type: (0, 1),
            preview_entity: None,
            zoom_level: 1.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct Hotkeys {
    pub mappings: HashMap<u8, (u8, u8)>,
}

#[derive(Resource)]
pub struct WorldRes {
    pub tiles: HashMap<Position, (Box<dyn Tile>, (u8, u8))>,
    pub terrain: HashMap<Position, TerrainTileType>,
    pub loaded_chunks: HashSet<ChunkPosition>,
    pub resources: HashMap<(u8, u8), u32>,
    pub world_seed: u32,
    pub tick_timer: Timer,
    pub tick_count: i32,
    pub actions: Vec<Action>,
    pub money: u32,
}

#[derive(Serialize, Deserialize, Encode, Decode)]
pub enum SerializableTile {
    Conveyor {
        position: Position,
        direction: Direction,
        item: Option<Item>,
    },
    Router {
        position: Position,
        direction: Direction,
        item: Option<Item>,
        last_output: RouterOutputIndex,
    },
    Extractor {
        position: Position,
        direction: Direction,
        extractor_type: ExtractorType,
        item: Option<Item>,
    },
    Factory {
        position: Position,
        direction: Direction,
        factory_type: FactoryType,
        inventory: HashMap<Item, u32>,
        item: Option<Item>,
        interval: u32,
        ticks: u32,
    },

    Storage {
        position: Position,
        direction: Direction,
        inventory: u32,
        storage_type: StorageType,
    },
    Portal {
        position: Position,
        item: Option<Item>,
    },
    Junction {
        position: Position,
        horizontal_item: Option<(Item, Direction)>,
        vertical_item: Option<(Item, Direction)>,
    },
    Core {
        position: Position,
        interval: u32,
        ticks: u32,
        tile_id: (u8, u8),
    },
}

#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct SerializableWorld {
    pub tiles: HashMap<u64, (SerializableTile, (u8, u8))>,
    pub resources: HashMap<(u8, u8), u32>,
    pub world_seed: u32,
    pub tick_count: i32,
    pub hotkey_mappings: HashMap<u8, (u8, u8)>,
    pub money: u32,
}

impl WorldRes {
    pub fn save(&self, path: impl AsRef<Path>, hotkeys: &Hotkeys) -> Result<(), io::Error> {
        let serializable_world = SerializableWorld {
            tiles: self
                .tiles
                .iter()
                .map(|(pos, (tile, id))| {
                    let serializable_tile =
                        if let Some(conveyor) = tile.as_any().downcast_ref::<Conveyor>() {
                            SerializableTile::Conveyor {
                                position: conveyor.position,
                                direction: conveyor.direction,
                                item: conveyor.item,
                            }

                        } else if let Some(extractor) = tile.as_any().downcast_ref::<Extractor>() {
                            SerializableTile::Extractor {
                                position: extractor.position,
                                direction: extractor.direction,
                                extractor_type: extractor.extractor_type,
                                item: extractor.item,
                            }
                        } else if let Some(factory) = tile.as_any().downcast_ref::<Factory>() {
                            SerializableTile::Factory {
                                position: factory.position,
                                direction: factory.direction,
                                factory_type: factory.factory_type,
                                inventory: factory.inventory.clone(),
                                item: factory.item,
                                interval: factory.interval,
                                ticks: factory.ticks,
                            }
                        } else if let Some(portal) = tile.as_any().downcast_ref::<Portal>() {
                            SerializableTile::Portal {
                                position: portal.position,
                                item: portal.item,
                            }
                        } else if let Some(junction) = tile.as_any().downcast_ref::<Junction>() {
                            SerializableTile::Junction {
                                position: junction.position,
                                horizontal_item: junction.horizontal_item,
                                vertical_item: junction.vertical_item,
                            }
                        } else if let Some(core) = tile.as_any().downcast_ref::<Core>() {
                            SerializableTile::Core {
                                position: core.position,
                                interval: core.interval,
                                ticks: core.ticks,
                                tile_id: core.tile_id,
                            }
                        } else if let Some(router) = tile.as_any().downcast_ref::<Router>() {
                            SerializableTile::Router {
                                position: router.position,
                                direction: router.direction,
                                item: router.item,
                                last_output: router.last_output,
                            }
                        } else {
                            SerializableTile::Conveyor {
                                position: *pos,
                                direction: Direction::Up,
                                item: None,
                            }
                        };
                    (pos.get_as_key(), (serializable_tile, *id))
                })
                .collect(),
            resources: self.resources.clone(),
            world_seed: self.world_seed,
            tick_count: self.tick_count,
            hotkey_mappings: hotkeys.mappings.clone(),
            money: self.money,
        };

        let config = config::standard().with_fixed_int_encoding().with_no_limit();

        let serialized = bincode::encode_to_vec(&serializable_world, config)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let file = File::create(path)?;
        let mut encoder = DeflateEncoder::new(file, Compression::best());
        encoder.write_all(&serialized)?;
        encoder.finish()?;

        Ok(())
    }

    pub fn load_game(path: impl AsRef<Path>) -> io::Result<(WorldRes, HashMap<u8, (u8, u8)>)> {
        let file = File::open(path)?;

        let mut decoder = DeflateDecoder::new(file);
        let mut buffer = Vec::new();
        decoder.read_to_end(&mut buffer)?;

        let config = config::standard().with_fixed_int_encoding().with_no_limit();

        let (serializable_world, _): (SerializableWorld, _) =
            bincode::decode_from_slice(&buffer, config)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let mut tiles = HashMap::new();
        let mut terrain = HashMap::new();
        let loaded_chunks = HashSet::new();

        for (pos_key, (tile, id)) in serializable_world.tiles {
            let pos = Position::from_key(pos_key);
            let boxed_tile: Box<dyn Tile> = match tile {
                SerializableTile::Conveyor {
                    position,
                    direction,
                    item,
                } => Box::new(Conveyor {
                    position,
                    direction,
                    item,
                }),

                SerializableTile::Extractor {
                    position,
                    direction,
                    extractor_type,
                    item,
                } => Box::new(Extractor {
                    position,
                    direction,
                    extractor_type,
                    item,
                }),
                SerializableTile::Factory {
                    position,
                    direction,
                    factory_type,
                    inventory,
                    item,
                    interval,
                    ticks,
                } => Box::new(Factory {
                    position,
                    direction,
                    factory_type,
                    inventory,
                    item,
                    interval,
                    ticks,
                }),
                SerializableTile::Storage {
                    position,
                    direction,
                    storage_type,
                    inventory,
                } => Box::new(Storage {
                    position,
                    direction,
                    storage_type,
                    inventory,
                }),
                SerializableTile::Portal { position, item } => Box::new(Portal { position, item }),
                SerializableTile::Router {
                    position,
                    direction,
                    item,
                    last_output,
                } => Box::new(Router {
                    position,
                    direction,
                    item,
                    last_output,
                }),
                SerializableTile::Junction {
                    position,
                    horizontal_item,
                    vertical_item,
                } => Box::new(Junction {
                    position,
                    horizontal_item,
                    vertical_item,
                }),
                SerializableTile::Core {
                    position,
                    interval,
                    ticks,
                    tile_id,
                } => Box::new(Core {
                    position,
                    interval,
                    ticks,
                    tile_id,
                }),
            };

            tiles.insert(pos, (boxed_tile, id));
        }

        if terrain.is_empty() {
            let seed = serializable_world.world_seed;
            let rigtorium_noise = Perlin::new(seed);
            let flextorium_noise = Perlin::new(seed.wrapping_add(1));
            let electrine_noise = Perlin::new(seed.wrapping_add(2));

            for x in -TERRAIN_GEN_RANGE..=TERRAIN_GEN_RANGE {
                for y in -TERRAIN_GEN_RANGE..=TERRAIN_GEN_RANGE {
                    let rigtorium_val = rigtorium_noise.get([
                        x as f64 * RIGTORIUM_NOISE_SCALE,
                        y as f64 * RIGTORIUM_NOISE_SCALE,
                    ]) + RIGTORIUM_DENSITY;

                    let flextorium_val = flextorium_noise.get([
                        x as f64 * FLEXTORIUM_NOISE_SCALE,
                        y as f64 * FLEXTORIUM_NOISE_SCALE,
                    ]) + FLEXTORIUM_DENSITY;

                    let electrine_val = electrine_noise.get([
                        x as f64 * ELECTRINE_NOISE_SCALE,
                        y as f64 * ELECTRINE_NOISE_SCALE,
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

                    terrain.insert(Position::new(x, y), terrain_type);
                }
            }
        }

        let world_res = WorldRes {
            tiles,
            terrain,
            loaded_chunks,
            resources: serializable_world.resources,
            world_seed: serializable_world.world_seed,
            tick_timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Repeating),
            tick_count: serializable_world.tick_count,
            actions: Vec::new(),
            money: serializable_world.money,
        };

        Ok((world_res, serializable_world.hotkey_mappings))
    }
}
