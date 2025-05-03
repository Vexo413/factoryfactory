use bevy::{color::palettes::css, input::mouse::MouseWheel, prelude::*, window::PrimaryWindow};
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bincode::{Decode, Encode, config};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use noise::{NoiseFn, Perlin};
use rand::{Rng, rng};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{HashMap, HashSet},
    f32::consts::{FRAC_PI_2, PI},
    fmt::Debug,
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

const TILE_SIZE: f32 = 64.0;
const ITEM_SIZE: f32 = 32.0;
const IMAGE_SIZE: f32 = 128.0;
const TICK_LENGTH: f32 = 1.0;
const CAMERA_SPEED: f32 = 10.0;

const TERRAIN_GEN_RANGE: i32 = 200;
const TERRAIN_BASE_THRESHOLD: f64 = 0.4;

const RIGTORIUM_NOISE_SCALE: f64 = 0.15;
const FLEXTORIUM_NOISE_SCALE: f64 = 0.15;
const ELECTRINE_NOISE_SCALE: f64 = 0.4;

const RIGTORIUM_DENSITY: f64 = -0.2;
const FLEXTORIUM_DENSITY: f64 = -0.3;
const ELECTRINE_DENSITY: f64 = -0.4;

const CHUNK_SIZE: i32 = 16;

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 2.0;
const ZOOM_SPEED: f32 = 0.0001;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ChunkPosition {
    x: i32,
    y: i32,
}

impl ChunkPosition {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    fn from_world_position(world_pos: Vec2) -> Self {
        Self {
            x: (world_pos.x / (CHUNK_SIZE as f32 * TILE_SIZE)).floor() as i32,
            y: (world_pos.y / (CHUNK_SIZE as f32 * TILE_SIZE)).floor() as i32,
        }
    }
}

#[derive(Component)]
struct TerrainChunk {
    position: ChunkPosition,
}

#[derive(Serialize, Deserialize, Encode, Decode)]
enum SerializableTile {
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
        inventory: HashMap<Item, u32>,
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
struct SerializableWorld {
    tiles: HashMap<u64, (SerializableTile, (u8, u8))>,
    resources: HashMap<(u8, u8), u32>,
    world_seed: u32,
    tick_count: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
enum TerrainTileType {
    RawFlextoriumDeposit,
    RawRigtoriumDeposit,
    ElectrineDeposit,
    Stone,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn shift(&self, i: i32) -> Direction {
        let current_index = match self {
            Direction::Up => 0,
            Direction::Right => 1,
            Direction::Down => 2,
            Direction::Left => 3,
        };

        let new_index = (current_index + i).rem_euclid(4);

        match new_index {
            0 => Direction::Up,
            1 => Direction::Right,
            2 => Direction::Down,
            3 => Direction::Left,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
enum FactoryType {
    RigtoriumSmelter,
    FlextoriumFabricator,
    RigtoriumRodMolder,
    ConveyorConstructor,
}

impl FactoryType {
    fn capacity(&self) -> HashMap<Item, u32> {
        match self {
            FactoryType::RigtoriumSmelter => {
                let mut hashmap = HashMap::new();
                hashmap.insert(Item::RawRigtorium, 2);
                hashmap.insert(Item::Electrine, 2);
                hashmap
            }
            FactoryType::FlextoriumFabricator => {
                let mut hashmap = HashMap::new();
                hashmap.insert(Item::RawFlextorium, 2);
                hashmap.insert(Item::Electrine, 2);
                hashmap
            }
            FactoryType::RigtoriumRodMolder => {
                let mut hashmap = HashMap::new();
                hashmap.insert(Item::Rigtorium, 4);
                hashmap.insert(Item::Electrine, 2);
                hashmap
            }
            FactoryType::ConveyorConstructor => {
                let mut hashmap = HashMap::new();
                hashmap.insert(Item::Flextorium, 8);
                hashmap.insert(Item::RigtoriumRod, 4);
                hashmap.insert(Item::Electrine, 2);
                hashmap
            }
        }
    }
    fn recipe(&self) -> Recipe {
        match self {
            FactoryType::RigtoriumSmelter => {
                let mut inputs = HashMap::new();
                inputs.insert(Item::RawRigtorium, 1);
                inputs.insert(Item::Electrine, 1);
                Recipe {
                    inputs,
                    output: Item::Rigtorium,
                }
            }
            FactoryType::FlextoriumFabricator => {
                let mut inputs = HashMap::new();
                inputs.insert(Item::RawFlextorium, 1);
                inputs.insert(Item::Electrine, 1);
                Recipe {
                    inputs,
                    output: Item::Flextorium,
                }
            }
            FactoryType::RigtoriumRodMolder => {
                let mut inputs = HashMap::new();
                inputs.insert(Item::Rigtorium, 2);
                inputs.insert(Item::Electrine, 1);
                Recipe {
                    inputs,
                    output: Item::RigtoriumRod,
                }
            }
            FactoryType::ConveyorConstructor => {
                let mut inputs = HashMap::new();
                inputs.insert(Item::Flextorium, 4);
                inputs.insert(Item::RigtoriumRod, 2);
                inputs.insert(Item::Electrine, 1);
                Recipe {
                    inputs,
                    output: Item::Conveyor,
                }
            }
        }
    }
    fn sprite(&self) -> &'static str {
        match self {
            FactoryType::RigtoriumSmelter => {
                "embedded://textures/tiles/factories/rigtorium_smelter.png"
            }
            FactoryType::FlextoriumFabricator => {
                "embedded://textures/tiles/factories/flextorium_fabricator.png"
            }
            FactoryType::RigtoriumRodMolder => {
                "embedded://textures/tiles/factories/rigtorium_rod_molder.png"
            }
            FactoryType::ConveyorConstructor => {
                "embedded://textures/tiles/factories/conveyor_constructor.png"
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Move(Position, Position, Item),
    MoveRouter(Position, Position, Item, RouterOutputIndex),
    Produce(Position),
    Teleport(Position, (u8, u8)),
    IncreaseTicks(Position),
}

#[derive(PartialEq, Eq, Clone, Hash, Debug, Copy, Deserialize, Serialize, Encode, Decode)]
enum Item {
    RawFlextorium,
    RawRigtorium,
    Flextorium,
    Rigtorium,
    Electrine,
    RigtoriumRod,
    Conveyor,
}

impl Item {
    fn sprite(&self) -> &'static str {
        match self {
            Item::RawFlextorium => "embedded://textures/items/raw_flextorium.png",
            Item::RawRigtorium => "embedded://textures/items/raw_rigtorium.png",
            Item::Flextorium => "embedded://textures/items/flextorium.png",
            Item::Rigtorium => "embedded://textures/items/rigtorium.png",
            Item::Electrine => "embedded://textures/items/electrine.png",
            Item::RigtoriumRod => "embedded://textures/items/rigtorium_rod.png",
            Item::Conveyor => "embedded://textures/items/conveyor.png",
        }
    }
    fn to_tile(&self) -> Option<(u8, u8)> {
        match self {
            Item::Conveyor => Some((1, 1)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct Recipe {
    inputs: HashMap<Item, u32>,
    output: Item,
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Serialize, Deserialize, Encode, Decode,
)]
struct Position {
    x: i32,
    y: i32,
}

impl Position {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    fn shift(&self, direction: Direction) -> Position {
        let mut pos = *self;
        match direction {
            Direction::Up => pos.y += 1,
            Direction::Down => pos.y -= 1,
            Direction::Left => pos.x -= 1,
            Direction::Right => pos.x += 1,
        }
        pos
    }

    fn get_as_key(&self) -> u64 {
        ((self.x as u64) & 0xFFFFFFFF) | (((self.y as u64) & 0xFFFFFFFF) << 32)
    }

    fn from_key(key: u64) -> Self {
        let x = (key & 0xFFFFFFFF) as i32;
        let y = ((key >> 32) & 0xFFFFFFFF) as i32;
        Position::new(x, y)
    }
}

#[derive(Resource)]
struct Placer {
    direction: Direction,
    tile_type: (u8, u8),
    preview_entity: Option<Entity>,
    zoom_level: f32,
}

impl Default for Placer {
    fn default() -> Self {
        Self {
            direction: Direction::Up,
            tile_type: (1, 1),
            preview_entity: None,
            zoom_level: 1.0,
        }
    }
}

#[derive(Resource)]
struct WorldRes {
    tiles: HashMap<Position, (Box<dyn Tile>, (u8, u8))>,
    terrain: HashMap<Position, TerrainTileType>,
    loaded_chunks: HashSet<ChunkPosition>,
    resources: HashMap<(u8, u8), u32>,
    world_seed: u32,
    tick_timer: Timer,
    tick_count: i32,
    actions: Vec<Action>,
}

#[derive(Component)]
struct Inventory {
    selected_category: u8,
}

#[derive(Component)]
struct InventoryCategory {
    category: u8,
}

#[derive(Component)]
struct InventoryItemsPanel;

#[derive(Component)]
struct InventoryItem {
    tile_type: (u8, u8),
}
#[derive(Component)]
struct ContextMenu;

#[derive(Component)]
struct HotkeyOption {
    tile_type: (u8, u8),
}

#[derive(Component)]
struct HotkeyButton {
    key: u8,
    tile_type: (u8, u8),
}

#[derive(Resource, Default)]
struct Hotkeys {
    mappings: HashMap<u8, (u8, u8)>,
}

impl WorldRes {
    fn save(&self, path: impl AsRef<Path>) -> Result<(), io::Error> {
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
                        } else if let Some(router) = tile.as_any().downcast_ref::<Router>() {
                            SerializableTile::Router {
                                position: router.position,
                                direction: router.direction,
                                item: router.item,
                                last_output: router.last_output,
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

    fn load(path: impl AsRef<Path>) -> io::Result<Self> {
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

        Ok(WorldRes {
            tiles,
            terrain,
            loaded_chunks,
            resources: serializable_world.resources,
            world_seed: serializable_world.world_seed,
            tick_timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Repeating),
            tick_count: serializable_world.tick_count,
            actions: Vec::new(),
        })
    }
}

impl Default for WorldRes {
    fn default() -> Self {
        let world = WorldRes::load("savegame.ff");
        let mut resources = HashMap::new();
        resources.insert((1, 1), 40);
        resources.insert((1, 2), 10);
        resources.insert((1, 3), 10);
        resources.insert((2, 1), 10);
        resources.insert((2, 2), 10);
        resources.insert((2, 3), 10);
        resources.insert((2, 4), 10);
        resources.insert((3, 1), 10);
        resources.insert((3, 2), 10);
        resources.insert((3, 3), 10);
        resources.insert((4, 1), 10);
        resources.insert((5, 1), 10);

        let mut tiles: HashMap<Position, (Box<dyn Tile + 'static>, (u8, u8))> = HashMap::new();
        tiles.insert(
            Position::new(0, 0),
            (
                Box::new(Core {
                    position: Position::new(0, 0),
                    interval: 10,
                    ticks: 0,
                    tile_id: (0, 1),
                }),
                (6, 1),
            ),
        );

        world.unwrap_or(WorldRes {
            tiles,
            terrain: HashMap::new(),
            loaded_chunks: HashSet::new(),
            resources,
            world_seed: rng().random_range(u32::MIN..u32::MAX),
            tick_timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Repeating),
            tick_count: 0,
            actions: Vec::new(),
        })
    }
}

#[derive(Component)]
struct TileSprite {
    pos: Position,
}

#[derive(Component)]
struct ItemAnimation {
    start_pos: Vec3,
    end_pos: Vec3,
    timer: Timer,
}

trait Tile: Send + Sync + Debug {
    fn tick(&self, tiles: &WorldRes) -> Option<Action>;
    fn set_item(&mut self, item: Option<Item>);
    fn get_item(&self) -> Option<Item>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Debug)]
struct Conveyor {
    position: Position,
    direction: Direction,
    item: Option<Item>,
}

impl Tile for Conveyor {
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        let start_position = self.position;
        let mut end_position = self.position;

        match self.direction {
            Direction::Up => end_position.y += 1,
            Direction::Down => end_position.y -= 1,
            Direction::Left => end_position.x -= 1,
            Direction::Right => end_position.x += 1,
        }

        if world.tiles.contains_key(&end_position) {
            if let Some(item) = self.item {
                return Some(Action::Move(start_position, end_position, item));
            }
        }

        None
    }

    fn set_item(&mut self, item: Option<Item>) {
        self.item = item;
    }

    fn get_item(&self) -> Option<Item> {
        self.item
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
enum RouterOutputIndex {
    Forward = 0,
    Right = 1,
    Left = 2,
}
impl RouterOutputIndex {
    fn next(&self) -> Self {
        match self {
            RouterOutputIndex::Forward => RouterOutputIndex::Right,
            RouterOutputIndex::Right => RouterOutputIndex::Left,
            RouterOutputIndex::Left => RouterOutputIndex::Forward,
        }
    }

    fn to_direction(&self, base_direction: Direction) -> Direction {
        match self {
            RouterOutputIndex::Forward => base_direction,
            RouterOutputIndex::Right => rotate_direction_clockwise(base_direction),
            RouterOutputIndex::Left => rotate_direction_counterclockwise(base_direction),
        }
    }
}

#[derive(Debug)]
struct Router {
    position: Position,
    direction: Direction,
    item: Option<Item>,
    last_output: RouterOutputIndex,
}

impl Tile for Router {
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        if let Some(item) = self.item {
            let mut next_output = self.last_output.next();
            let start_position = self.position;

            for _ in 0..3 {
                let dir = next_output.to_direction(self.direction);
                let mut end_pos = self.position;

                match dir {
                    Direction::Up => end_pos.y += 1,
                    Direction::Down => end_pos.y -= 1,
                    Direction::Left => end_pos.x -= 1,
                    Direction::Right => end_pos.x += 1,
                }

                if let Some(tile) = world.tiles.get(&end_pos) {
                    let can_accept =
                        if let Some(conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
                            conveyor.item.is_none()
                        } else if let Some(router) = tile.0.as_any().downcast_ref::<Router>() {
                            router.item.is_none()
                        } else if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
                            factory.factory_type.capacity().get(&item).unwrap_or(&0)
                                > factory.inventory.get(&item).unwrap_or(&0)
                        } else {
                            false
                        };

                    if can_accept {
                        return Some(Action::MoveRouter(
                            start_position,
                            end_pos,
                            item,
                            next_output,
                        ));
                    }
                }
                next_output = next_output.next();
            }
        }
        None
    }

    fn set_item(&mut self, item: Option<Item>) {
        self.item = item;
    }

    fn get_item(&self) -> Option<Item> {
        self.item
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
enum ExtractorType {
    RawFlextorium,
    RawRigtorium,
    Electrine,
}
impl ExtractorType {
    fn interval(&self) -> i32 {
        match self {
            ExtractorType::RawRigtorium => 5,
            ExtractorType::RawFlextorium => 5,
            ExtractorType::Electrine => 2,
        }
    }
    fn terrain(&self) -> TerrainTileType {
        match self {
            ExtractorType::RawRigtorium => TerrainTileType::RawRigtoriumDeposit,
            ExtractorType::RawFlextorium => TerrainTileType::RawFlextoriumDeposit,
            ExtractorType::Electrine => TerrainTileType::ElectrineDeposit,
        }
    }
    fn spawn_item(&self) -> Item {
        match self {
            ExtractorType::RawRigtorium => Item::RawRigtorium,
            ExtractorType::RawFlextorium => Item::RawFlextorium,
            ExtractorType::Electrine => Item::Electrine,
        }
    }
    fn sprite(&self) -> String {
        match self {
            ExtractorType::RawRigtorium => "embedded://textures/tiles/extractors/raw_rigtorium.png",
            ExtractorType::RawFlextorium => {
                "embedded://textures/tiles/extractors/raw_flextorium.png"
            }
            ExtractorType::Electrine => "embedded://textures/tiles/extractors/electrine.png",
        }
        .to_string()
    }
}

#[derive(Debug)]
struct Extractor {
    position: Position,
    direction: Direction,
    extractor_type: ExtractorType,
    item: Option<Item>,
}

impl Tile for Extractor {
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        if world.tick_count % self.extractor_type.interval() == 0
            && world.terrain.get(&self.position) == Some(&self.extractor_type.terrain())
        {
            return Some(Action::Produce(self.position));
        }
        let start_position = self.position;
        let mut end_position = self.position;

        match self.direction {
            Direction::Up => end_position.y += 1,
            Direction::Down => end_position.y -= 1,
            Direction::Left => end_position.x -= 1,
            Direction::Right => end_position.x += 1,
        }

        if world.tiles.contains_key(&end_position) {
            if let Some(item) = self.item {
                return Some(Action::Move(start_position, end_position, item));
            }
        }

        None
    }

    fn set_item(&mut self, item: Option<Item>) {
        self.item = item;
    }

    fn get_item(&self) -> Option<Item> {
        self.item
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug)]
struct Factory {
    position: Position,
    direction: Direction,
    factory_type: FactoryType,
    inventory: HashMap<Item, u32>,
    item: Option<Item>,
    interval: u32,
    ticks: u32,
}

impl Factory {
    fn can_produce(&self) -> bool {
        let recipe = self.factory_type.recipe();
        recipe
            .inputs
            .iter()
            .all(|(item, &qty_required)| self.inventory.get(item).unwrap_or(&0) >= &qty_required)
            && self.item.is_none()
    }

    fn produce(&mut self) -> Option<Item> {
        let recipe = self.factory_type.recipe();
        if self.can_produce() {
            for (item, &qty_required) in recipe.inputs.iter() {
                if let Some(qty) = self.inventory.get_mut(item) {
                    *qty = qty.saturating_sub(qty_required);
                }
            }

            Some(recipe.output)
        } else {
            None
        }
    }
}

impl Tile for Factory {
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        if self.can_produce() {
            return Some(Action::Produce(self.position));
        }

        let mut end_position = self.position;

        match self.direction {
            Direction::Up => end_position.y += 1,
            Direction::Down => end_position.y -= 1,
            Direction::Left => end_position.x -= 1,
            Direction::Right => end_position.x += 1,
        }

        if world.tiles.contains_key(&end_position) {
            if let Some(item) = self.item {
                return Some(Action::Move(self.position, end_position, item));
            }
        }

        None
    }

    fn set_item(&mut self, item: Option<Item>) {
        self.item = item;
    }

    fn get_item(&self) -> Option<Item> {
        self.item
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
enum StorageType {
    SmallVault,
    MediumVault,
    LargeVault,
}

impl StorageType {
    fn capacity(&self) -> HashMap<Item, u32> {
        match self {
            StorageType::SmallVault => {
                let mut hashmap = HashMap::new();
                hashmap.insert(Item::RawRigtorium, 5);
                hashmap.insert(Item::RawFlextorium, 5);
                hashmap
            }
            StorageType::MediumVault => {
                let mut hashmap = HashMap::new();
                hashmap.insert(Item::RawRigtorium, 10);
                hashmap.insert(Item::RawFlextorium, 10);
                hashmap
            }

            StorageType::LargeVault => {
                let mut hashmap = HashMap::new();
                hashmap.insert(Item::RawRigtorium, 20);
                hashmap.insert(Item::RawFlextorium, 20);
                hashmap
            }
        }
    }
}

#[derive(Debug)]
struct Storage {
    position: Position,
    direction: Direction,
    inventory: HashMap<Item, u32>,
    storage_type: StorageType,
}

impl Tile for Storage {
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        let mut end_position = self.position;

        match self.direction {
            Direction::Up => end_position.y += 1,
            Direction::Down => end_position.y -= 1,
            Direction::Left => end_position.x -= 1,
            Direction::Right => end_position.x += 1,
        }
        if let Some(tile) = world.tiles.get(&end_position) {
            if let Some(conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
                if conveyor.item.is_none() {
                    return Some(Action::Produce(self.position));
                }
            }
        }

        None
    }

    fn set_item(&mut self, _: Option<Item>) {}

    fn get_item(&self) -> Option<Item> {
        return None;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug)]
struct Portal {
    position: Position,
    item: Option<Item>,
}
impl Tile for Portal {
    fn tick(&self, _world: &WorldRes) -> Option<Action> {
        if let Some(item) = self.item {
            if let Some(tile) = item.to_tile() {
                return Some(Action::Teleport(self.position, tile));
            }
        }

        None
    }

    fn set_item(&mut self, item: Option<Item>) {
        self.item = item;
    }

    fn get_item(&self) -> Option<Item> {
        return None;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn can_tile_accept_item(tile: &(Box<dyn Tile>, (u8, u8)), item: Item) -> bool {
    if let Some(conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
        conveyor.item.is_none()
    } else if let Some(router) = tile.0.as_any().downcast_ref::<Router>() {
        router.item.is_none()
    } else if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
        factory.factory_type.capacity().get(&item).unwrap_or(&0)
            > factory.inventory.get(&item).unwrap_or(&0)
    } else if let Some(junction) = tile.0.as_any().downcast_ref::<Junction>() {
        junction.horizontal_item.is_none()
    } else if let Some(portal) = tile.0.as_any().downcast_ref::<Portal>() {
        portal.item.is_none()
    } else {
        false
    }
}

#[derive(Debug)]
struct Junction {
    position: Position,

    horizontal_item: Option<(Item, Direction)>,

    vertical_item: Option<(Item, Direction)>,
}
impl Tile for Junction {
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        if let Some((item, input_dir)) = self.horizontal_item {
            let output = match input_dir {
                Direction::Left => Direction::Right,
                Direction::Right => Direction::Left,
                _ => return None,
            };
            let end_pos = self.position.shift(output);
            if world.tiles.get(&end_pos).is_some()
                && can_tile_accept_item(world.tiles.get(&end_pos).unwrap(), item)
            {
                return Some(Action::Move(self.position, end_pos, item));
            }
        }

        if let Some((item, input_dir)) = self.vertical_item {
            let output = match input_dir {
                Direction::Down => Direction::Up,
                Direction::Up => Direction::Down,
                _ => return None,
            };
            let end_pos = self.position.shift(output);
            if world.tiles.get(&end_pos).is_some()
                && can_tile_accept_item(world.tiles.get(&end_pos).unwrap(), item)
            {
                return Some(Action::Move(self.position, end_pos, item));
            }
        }
        None
    }

    fn set_item(&mut self, _item: Option<Item>) {}

    fn get_item(&self) -> Option<Item> {
        return None;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug)]
struct Core {
    position: Position,
    interval: u32,
    ticks: u32,
    tile_id: (u8, u8),
}

impl Tile for Core {
    fn tick(&self, _world: &WorldRes) -> Option<Action> {
        dbg!(self.ticks);
        if self.ticks >= self.interval {
            println!("created: {:?}", self.tile_id);
            return Some(Action::Teleport(self.position, self.tile_id));
        } else {
            return Some(Action::IncreaseTicks(self.position));
        }
    }
    fn set_item(&mut self, _item: Option<Item>) {}

    fn get_item(&self) -> Option<Item> {
        return None;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

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
                manage_terrain_chunks,
                tick_tiles,
                update_tile_visuals.after(tick_tiles),
                animate_items.after(update_tile_visuals),
                manage_tiles,
                move_camera,
                update_inventory_view,
                handle_inventory_interaction,
                handle_context_menu,
                handle_hotkey_assignment,
                handle_core_context_menu,
                update_core_menu_ui,
                update_core_progress_text,
            )
                .chain(),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut world: ResMut<WorldRes>) {
    commands.spawn(Camera2d);

    /*for (pos, terrain) in world.terrain.iter() {
        let texture_path = match terrain {
            TerrainTileType::Stone => "embedded://textures/terrain/stone.png",
            TerrainTileType::RawFlextoriumDeposit => "embedded://textures/terrain/flextorium.png",
            TerrainTileType::RawRigtoriumDeposit => "embedded://textures/terrain/rigtorium.png",
            TerrainTileType::ElectrineDeposit => "embedded://textures/terrain/electrine.png",
        };
        commands.spawn((
            Sprite::from_image(asset_server.load(texture_path)),
            Transform {
                translation: Vec3::new(pos.x as f32 * TILE_SIZE, pos.y as f32 * TILE_SIZE, -1.0),
                scale: Vec3::splat(TILE_SIZE / IMAGE_SIZE),
                ..Default::default()
            },
        ));
    }*/

    if world.tiles.is_empty() {
        world.tiles.insert(
            Position::new(-3, -3),
            (
                Box::new(Extractor {
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
                Box::new(Extractor {
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

fn manage_terrain_chunks(
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

fn tick_tiles(
    time: Res<Time>,
    mut commands: Commands,
    mut world: ResMut<WorldRes>,
    asset_server: Res<AssetServer>,
) {
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
                                            }
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
                    /*let item_info: Option<(Item, Position)> = {
                        if let Some(tile) = world.tiles.get(&position) {
                            if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
                                if factory.can_produce() {
                                    let recipe = factory.factory_type.recipe();
                                    let mut end_position = position;
                                    match factory.direction {
                                        Direction::Up => end_position.y += 1,
                                        Direction::Down => end_position.y -= 1,
                                        Direction::Left => end_position.x -= 1,
                                        Direction::Right => end_position.x += 1,
                                    }
                                    Some((recipe.output, end_position))
                                } else {
                                    None
                                }
                            } else if let Some(extractor) =
                                tile.0.as_any().downcast_ref::<Extractor>()
                            {
                                if extractor.item.is_none() {
                                    let item = extractor.extractor_type.spawn_item();
                                    let mut end_position = position;
                                    match extractor.direction {
                                        Direction::Up => end_position.y += 1,
                                        Direction::Down => end_position.y -= 1,
                                        Direction::Left => end_position.x -= 1,
                                        Direction::Right => end_position.x += 1,
                                    }
                                    Some((item, end_position))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };*/

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
                                dbg!(!filled_positions.contains(end));
                                dbg!(empty_positions.contains(end));
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
        if let Err(err) = world.save("savegame.ff") {
            eprintln!("Error saving game: {}", err);
        }
    }
}

fn sort_moves_topologically(actions: Vec<Action>, world: &WorldRes) -> Vec<Action> {
    let mut position_to_output_action: HashMap<Position, Vec<usize>> = HashMap::new();

    let mut position_to_input_action: HashMap<Position, Vec<usize>> = HashMap::new();

    for (i, action) in actions.iter().enumerate() {
        match action {
            Action::Move(from, to, _) => {
                position_to_output_action.entry(*from).or_default().push(i);
                position_to_input_action.entry(*to).or_default().push(i);
            }
            Action::MoveRouter(from, to, _, _) => {
                position_to_output_action.entry(*from).or_default().push(i);
                position_to_input_action.entry(*to).or_default().push(i);
            }
            Action::Produce(pos) => {
                if let Some((_, destination)) = get_produce_destination(*pos, world) {
                    position_to_output_action.entry(*pos).or_default().push(i);
                    position_to_input_action
                        .entry(destination)
                        .or_default()
                        .push(i);
                }
            }
            Action::Teleport(pos, _) => {
                position_to_output_action.entry(*pos).or_default().push(i);
            }
            Action::IncreaseTicks(_) => {}
        }
    }

    let mut graph: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut in_degree: HashMap<usize, usize> = HashMap::new();

    for (pos, output_actions) in &position_to_output_action {
        if let Some(input_actions) = position_to_input_action.get(pos) {
            for &output_action in output_actions {
                for &input_action in input_actions {
                    if output_action != input_action {
                        graph.entry(input_action).or_default().push(output_action);
                        *in_degree.entry(output_action).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let mut queue: Vec<usize> = (0..actions.len())
        .filter(|i| !in_degree.contains_key(i))
        .collect();
    let mut sorted = Vec::new();
    let mut visited = HashSet::new();

    while let Some(i) = queue.pop() {
        sorted.push(actions[i].clone());
        visited.insert(i);

        if let Some(dependents) = graph.get(&i) {
            for &dep in dependents {
                if let Some(entry) = in_degree.get_mut(&dep) {
                    *entry -= 1;
                    if *entry == 0 {
                        queue.push(dep);
                    }
                }
            }
        }
    }

    for (i, action) in actions.iter().enumerate() {
        if !visited.contains(&i) {
            sorted.push(action.clone());
        }
    }

    sorted
}

fn get_produce_destination(pos: Position, world: &WorldRes) -> Option<(Position, Position)> {
    if let Some((tile, _)) = world.tiles.get(&pos) {
        let mut end_position = pos;

        if let Some(factory) = tile.as_any().downcast_ref::<Factory>() {
            match factory.direction {
                Direction::Up => end_position.y += 1,
                Direction::Down => end_position.y -= 1,
                Direction::Left => end_position.x -= 1,
                Direction::Right => end_position.x += 1,
            }
            return Some((pos, end_position));
        } else if let Some(extractor) = tile.as_any().downcast_ref::<Extractor>() {
            match extractor.direction {
                Direction::Up => end_position.y += 1,
                Direction::Down => end_position.y -= 1,
                Direction::Left => end_position.x -= 1,
                Direction::Right => end_position.x += 1,
            }
            return Some((pos, end_position));
        }
    }
    None
}
fn update_tile_visuals(
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
                                asset_server.load("textures/items/none.png")
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
fn determine_conveyor_texture(world: &WorldRes, conveyor: &Conveyor) -> &'static str {
    let pos = conveyor.position;
    let dir = conveyor.direction;

    let (behind_pos, left_pos, right_pos) = match dir {
        Direction::Up => (
            Position::new(pos.x, pos.y - 1),
            Position::new(pos.x - 1, pos.y),
            Position::new(pos.x + 1, pos.y),
        ),
        Direction::Down => (
            Position::new(pos.x, pos.y + 1),
            Position::new(pos.x + 1, pos.y),
            Position::new(pos.x - 1, pos.y),
        ),
        Direction::Left => (
            Position::new(pos.x + 1, pos.y),
            Position::new(pos.x, pos.y - 1),
            Position::new(pos.x, pos.y + 1),
        ),
        Direction::Right => (
            Position::new(pos.x - 1, pos.y),
            Position::new(pos.x, pos.y + 1),
            Position::new(pos.x, pos.y - 1),
        ),
    };

    let has_behind = is_conveyor_pointing_to(world, behind_pos, dir);
    let has_left = is_conveyor_pointing_to(world, left_pos, rotate_direction_clockwise(dir));
    let has_right =
        is_conveyor_pointing_to(world, right_pos, rotate_direction_counterclockwise(dir));

    match (has_behind, has_left, has_right) {
        (true, true, false) => "embedded://textures/tiles/conveyors/left_back.png",
        (true, false, true) => "embedded://textures/tiles/conveyors/right_back.png",
        (false, true, true) => "embedded://textures/tiles/conveyors/sides.png",
        (true, false, false) => "embedded://textures/tiles/conveyors/back.png",
        (false, true, false) => "embedded://textures/tiles/conveyors/left.png",
        (false, false, true) => "embedded://textures/tiles/conveyors/right.png",
        (true, true, true) => "embedded://textures/tiles/conveyors/all.png",
        _ => "embedded://textures/tiles/conveyors/back.png",
    }
}

fn is_conveyor_pointing_to(
    world: &WorldRes,
    from_pos: Position,
    pointing_direction: Direction,
) -> bool {
    if let Some(tile) = world.tiles.get(&from_pos) {
        if let Some(conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
            return conveyor.direction == pointing_direction;
        } else if let Some(router) = tile.0.as_any().downcast_ref::<Router>() {
            return router.direction == pointing_direction
                || router.direction.shift(1) == pointing_direction
                || router.direction.shift(-1) == pointing_direction;
        } else if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
            return factory.direction == pointing_direction;
        } else if let Some(extractor) = tile.0.as_any().downcast_ref::<Extractor>() {
            return extractor.direction == pointing_direction;
        } else if let Some(_junction) = tile.0.as_any().downcast_ref::<Junction>() {
            return pointing_direction == Direction::Up
                || pointing_direction == Direction::Down
                || pointing_direction == Direction::Left
                || pointing_direction == Direction::Right;
        }
    }
    false
}

fn get_tile_texture(tile_type: (u8, u8)) -> &'static str {
    match tile_type {
        (0, 1) => "embedded://textures/tiles/none.png",
        (1, 1) => "embedded://textures/tiles/conveyors/back.png",
        (1, 2) => "embedded://textures/tiles/conveyors/router.png",
        (1, 3) => "embedded://textures/tiles/conveyors/junction.png",
        (2, 1) => "embedded://textures/tiles/factories/rigtorium_smelter.png",
        (2, 2) => "embedded://textures/tiles/factories/flextorium_fabricator.png",
        (2, 3) => "embedded://textures/tiles/factories/conveyor_constructor.png",
        (2, 4) => "embedded://textures/tiles/factories/rigtorium_rod_molder.png",
        (3, 1) => "embedded://textures/tiles/extractors/raw_rigtorium.png",
        (3, 2) => "embedded://textures/tiles/extractors/raw_flextorium.png",
        (3, 3) => "embedded://textures/tiles/extractors/electrine.png",
        (4, 1) => "embedded://textures/tiles/portal.png",
        (5, 1) => "embedded://textures/tiles/storage.png",
        (6, 1) => "embedded://textures/tiles/core.png",
        _ => "embedded://textures/tiles/conveyors/back.png",
    }
}
fn get_tile_core_interval(tile_type: (u8, u8)) -> u32 {
    match tile_type {
        (1, 1) => 60,  // Conveyor - 1 minute
        (1, 2) => 80,  // Router - 1 minute 20 seconds
        (1, 3) => 90,  // Junction - 1 minute 30 seconds
        (2, 1) => 150, // Rigtorium Smelter - 2 minutes 30 seconds
        (2, 2) => 150, // Flextorium Fabricator - 2 minutes 30 seconds
        (2, 3) => 300, // Conveyor Constructor - 5 minutes
        (2, 4) => 180, // Rigtorium Rod Molder - 3 minutes
        (3, 1) => 240, // Raw Rigtorium Extractor - 4 minutes
        (3, 2) => 240, // Raw Flextorium Extractor - 4 minutes
        (3, 3) => 180, // Electrine Extractor - 3 minutes
        (4, 1) => 100, // Portal - 1 minute 40 seconds
        _ => 60,       // Default - 1 minute
    }
}

fn format_tile_id(tile_type: (u8, u8)) -> String {
    format!("{}, {}", tile_type.0, tile_type.1)
}

fn get_tile_name(tile_type: (u8, u8)) -> String {
    match tile_type {
        (1, 1) => "Conveyor",
        (1, 2) => "Router",
        (1, 3) => "Junction",
        (2, 1) => "Rigtorium Smelter",
        (2, 2) => "Flextorium Fabricator",
        (2, 3) => "Conveyor Constructor",
        (2, 4) => "Rigtorium Rod Molder",
        (3, 1) => "Raw Rigtorium Extractor",
        (3, 2) => "Raw Flextorium Extractor",
        (3, 3) => "Electrine Extractor",
        (4, 1) => "Portal",
        _ => "Unknown Tile",
    }
    .to_string()
}

fn rotate_direction_clockwise(dir: Direction) -> Direction {
    match dir {
        Direction::Up => Direction::Right,
        Direction::Right => Direction::Down,
        Direction::Down => Direction::Left,
        Direction::Left => Direction::Up,
    }
}

fn rotate_direction_counterclockwise(dir: Direction) -> Direction {
    match dir {
        Direction::Up => Direction::Left,
        Direction::Left => Direction::Down,
        Direction::Down => Direction::Right,
        Direction::Right => Direction::Up,
    }
}

fn animate_items(
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

fn manage_tiles(
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
    core_context_query: Query<&CoreContextMenu>,
    inventory_query: Query<Entity, With<Inventory>>,
) {
    if inventory_query.is_empty() {
        if keyboard_input.just_pressed(KeyCode::Digit0) {
            if let Some(&tile_type) = hotkeys.mappings.get(&0) {
                placer.tile_type = tile_type;
            } else {
                placer.tile_type = (2, 2);
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit1) {
            if let Some(&tile_type) = hotkeys.mappings.get(&1) {
                placer.tile_type = tile_type;
            } else {
                placer.tile_type = (1, 1);
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit2) {
            if let Some(&tile_type) = hotkeys.mappings.get(&2) {
                placer.tile_type = tile_type;
            } else {
                placer.tile_type = (2, 1);
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit3) {
            if let Some(&tile_type) = hotkeys.mappings.get(&3) {
                placer.tile_type = tile_type;
            } else {
                placer.tile_type = (3, 1);
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit4) {
            if let Some(&tile_type) = hotkeys.mappings.get(&4) {
                placer.tile_type = tile_type;
            } else {
                placer.tile_type = (3, 2);
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit5) {
            if let Some(&tile_type) = hotkeys.mappings.get(&5) {
                placer.tile_type = tile_type;
            } else {
                placer.tile_type = (3, 3);
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit6) {
            if let Some(&tile_type) = hotkeys.mappings.get(&6) {
                placer.tile_type = tile_type;
            } else {
                placer.tile_type = (4, 3);
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit7) {
            if let Some(&tile_type) = hotkeys.mappings.get(&7) {
                placer.tile_type = tile_type;
            } else {
                placer.tile_type = (2, 3);
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit8) {
            if let Some(&tile_type) = hotkeys.mappings.get(&8) {
                placer.tile_type = tile_type;
            } else {
                placer.tile_type = (4, 1);
            }
        } else if keyboard_input.just_pressed(KeyCode::Digit9) {
            if let Some(&tile_type) = hotkeys.mappings.get(&9) {
                placer.tile_type = tile_type;
            } else {
                placer.tile_type = (1, 2);
            }
        }
    }

    for event in mouse_wheel_events.read() {
        if placer.tile_type == (0, 1) {
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
            if inventory_query.is_empty() {
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

    if mouse_button_input.pressed(MouseButton::Left) && inventory_query.is_empty() {
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
                        if core_context_query.is_empty() {
                            if let Some(tile) = world.tiles.get(&pos) {
                                if let Some(core) = tile.0.as_any().downcast_ref::<Core>() {
                                    // Show Core context menu when clicking on a Core tile
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
                                        CoreContextMenu {
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
                                                    // Categories panel
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
                                                                    Text::new("1: Conveyors"),
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
                                                                    Text::new("2: Factories"),
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
                                                                    Text::new("4: Special"),
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
                                                    // Items grid panel
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
    if mouse_button_input.pressed(MouseButton::Right) && inventory_query.is_empty() {
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

    if keyboard_input.just_pressed(KeyCode::KeyE) {
        if let Ok(entity) = inventory_query.single() {
            commands.entity(entity).despawn();
        } else {
            commands.spawn((
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
                        BackgroundColor(Color::srgb(0.14, 0.16, 0.19)),
                        BorderRadius::all(Val::Px(10.0)),
                        children![
                            (
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
                                    Text::new("1: Conveyors"),
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
                            ),
                            (
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
                                    Text::new("2: Factories"),
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
                            ),
                            (
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
                            ),
                            (
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
                                    Text::new("4: Special"),
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
                        InventoryItemsPanel,
                    ),
                ],
            ));
        }
        return;
    }
}

#[derive(Component)]
struct CoreContextMenu {
    position: Position,
    selected_category: u8,
}

#[derive(Component)]
struct CoreCategory {
    category: u8,
}

#[derive(Component)]
struct CoreItemsPanel;

#[derive(Component)]
struct TileTypeOption {
    tile_type: (u8, u8),
}

fn get_new_tile(
    tile_type: (u8, u8),
    position: Position,
    direction: Direction,
) -> (Box<dyn Tile>, (u8, u8)) {
    match tile_type {
        (1, 1) => (
            Box::new(Conveyor {
                position,
                direction,
                item: None,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (1, 2) => (
            Box::new(Router {
                position,
                direction,
                item: None,
                last_output: RouterOutputIndex::Forward,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (1, 3) => (
            Box::new(Junction {
                position,
                horizontal_item: None,
                vertical_item: None,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (2, 1) => (
            Box::new(Factory {
                factory_type: FactoryType::RigtoriumSmelter,
                position,
                direction,
                inventory: HashMap::new(),
                item: None,
                interval: 2,
                ticks: 0,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (2, 2) => (
            Box::new(Factory {
                factory_type: FactoryType::FlextoriumFabricator,
                position,
                direction,
                inventory: HashMap::new(),
                item: None,
                interval: 2,
                ticks: 0,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (2, 3) => (
            Box::new(Factory {
                factory_type: FactoryType::ConveyorConstructor,
                position,
                direction,
                inventory: HashMap::new(),
                item: None,
                interval: 5,
                ticks: 0,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (2, 4) => (
            Box::new(Factory {
                factory_type: FactoryType::RigtoriumRodMolder,
                position,
                direction,
                inventory: HashMap::new(),
                item: None,
                interval: 2,
                ticks: 0,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (3, 1) => (
            Box::new(Extractor {
                position,
                direction,
                extractor_type: ExtractorType::RawRigtorium,
                item: None,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (3, 2) => (
            Box::new(Extractor {
                position,
                direction,
                extractor_type: ExtractorType::RawFlextorium,
                item: None,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (3, 3) => (
            Box::new(Extractor {
                position,
                direction,
                extractor_type: ExtractorType::Electrine,
                item: None,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (4, 1) => (
            Box::new(Portal {
                position,
                item: None,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (5, 1) => (
            Box::new(Storage {
                position,
                direction,
                inventory: HashMap::new(),
                storage_type: StorageType::SmallVault,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (6, 1) => (
            Box::new(Core {
                position,
                interval: 10,
                ticks: 0,
                tile_id: (1, 1),
            }) as Box<dyn Tile>,
            tile_type,
        ),
        _ => (
            Box::new(Conveyor {
                position,
                direction,
                item: None,
            }) as Box<dyn Tile>,
            tile_type,
        ),
    }
}

fn move_camera(
    mut camera: Query<&mut Transform, With<Camera2d>>,
    placer: Res<Placer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    inventory_query: Query<(), With<Inventory>>,
) {
    if inventory_query.is_empty() {
        let mut direction = Vec2::ZERO;
        if keyboard_input.pressed(KeyCode::KeyW) {
            direction.y = 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction.y = -1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction.x = -1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction.x = 1.0;
        }
        if let Ok(mut camera) = camera.single_mut() {
            camera.translation +=
                direction.normalize_or_zero().extend(0.0) * CAMERA_SPEED / placer.zoom_level;
        }
    }
}

fn update_inventory_view(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    world: Res<WorldRes>,
    inventory_query: Query<&Inventory>,
    category_query: Query<(&InventoryCategory, Entity)>,
    item_panel_query: Query<Entity, With<InventoryItemsPanel>>,
    item_query: Query<Entity, With<InventoryItem>>,
) {
    if let Ok(inventory) = inventory_query.single() {
        for (category, entity) in category_query.iter() {
            let color = if category.category == inventory.selected_category {
                Color::srgb(0.3, 0.5, 0.7)
            } else {
                Color::srgb(0.2, 0.22, 0.25)
            };
            let bundle = commands.get_entity(entity);
            if bundle.is_ok() {
                bundle.unwrap().insert(BackgroundColor(color));
            } else {
                panic!("bacon!");
            }
        }

        for entity in item_query.iter() {
            commands.entity(entity).despawn();
        }

        if let Ok(panel_entity) = item_panel_query.single() {
            for ((type_a, type_b), count) in world.resources.iter() {
                if *count > 0 && *type_a == inventory.selected_category {
                    let texture_path = get_tile_texture((*type_a, *type_b));

                    let child = commands
                        .spawn((
                            Node {
                                width: Val::Px(80.0),
                                height: Val::Px(80.0),
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..Default::default()
                            },
                            BackgroundColor(Color::srgb(0.25, 0.27, 0.3)),
                            InventoryItem {
                                tile_type: (*type_a, *type_b),
                            },
                            Interaction::default(),
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

                    commands.entity(panel_entity).add_child(child);
                }
            }
        }
    }
}

fn handle_inventory_interaction(
    mut commands: Commands,
    category_query: Query<(&Interaction, &InventoryCategory), Changed<Interaction>>,
    item_query: Query<(&Interaction, &InventoryItem, Entity)>,
    mut inventory_query: Query<(Entity, &mut Inventory)>,
    mut placer: ResMut<Placer>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    context_menu_query: Query<Entity, With<ContextMenu>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left)
        || mouse_button_input.just_pressed(MouseButton::Right)
    {
        let mut close_menu = true;

        if mouse_button_input.just_pressed(MouseButton::Right) {
            for (interaction, _, _) in item_query.iter() {
                if matches!(interaction, Interaction::Hovered) {
                    close_menu = false;
                    break;
                }
            }
        }

        if close_menu {
            for entity in context_menu_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }

    for (interaction, category) in category_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, mut inventory)) = inventory_query.single_mut() {
                inventory.selected_category = category.category;
            }
        }
    }

    for (interaction, item, _) in item_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, _)) = inventory_query.single() {
                placer.tile_type = item.tile_type;
            }
        }
    }

    if mouse_button_input.just_pressed(MouseButton::Right) {
        for (interaction, item, _) in item_query.iter() {
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
                    ContextMenu,
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
                    ),],
                ));

                break;
            }
        }
    }
}

fn handle_context_menu(
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
}

fn handle_hotkey_assignment(
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

fn handle_core_context_menu(
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

                // Refresh tile options
                update_core_tiles(
                    &mut commands,
                    &world,
                    core_menu.selected_category,
                    &asset_server,
                    &tile_panel_query,
                    &tile_option_query,
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
                                BackgroundColor(Color::srgb(0.4, 0.6, 0.4))
                            } else {
                                BackgroundColor(Color::srgb(0.3, 0.3, 0.3))
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
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn update_core_menu_ui(
    mut commands: Commands,
    mut core_menu_query: Query<&CoreContextMenu>,
    mut category_query: Query<(&CoreCategory, &mut BackgroundColor)>,
) {
    if let Ok(core_menu) = core_menu_query.single() {
        // Update the category colors based on the selection
        for (cat, mut bg_color) in category_query.iter_mut() {
            *bg_color = if cat.category == core_menu.selected_category {
                BackgroundColor(Color::srgb(0.3, 0.5, 0.7))
            } else {
                BackgroundColor(Color::srgb(0.2, 0.22, 0.25))
            };
        }
    }
}

fn update_core_progress_text(
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
) {
    // Remove existing tile options
    for entity in tile_query.iter() {
        commands.entity(entity).despawn_recursive();
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

            let tile_entity = commands
                .spawn((
                    Node {
                        width: Val::Px(110.0),
                        height: Val::Px(140.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
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
