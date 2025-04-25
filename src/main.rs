use bevy::{
    color::palettes::css,
    input::mouse::MouseWheel,
    prelude::*,
    window::{PresentMode, PrimaryWindow, WindowTheme},
};
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};
use bincode::{Decode, Encode, config};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use noise::{NoiseFn, Perlin};
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
// Add these constants at the top of your file with the other constants
const TERRAIN_GEN_RANGE: i32 = 200; // How far to generate terrain (±20)
const TERRAIN_BASE_THRESHOLD: f64 = 0.4; // Base threshold for resource generation

// Scale determines the size of clusters - smaller values = larger clusters
const RIGTORIUM_NOISE_SCALE: f64 = 0.08; // Rigtorium cluster size
const FLEXTORIUM_NOISE_SCALE: f64 = 0.12; // Flextorium cluster size
const ELECTRINE_NOISE_SCALE: f64 = 0.4; // Electrine cluster size

// Density affects how common each resource is (higher = more common)
const RIGTORIUM_DENSITY: f64 = 0.05; // Additional bias for Rigtorium
const FLEXTORIUM_DENSITY: f64 = 0.0; // Additional bias for Flextorium
const ELECTRINE_DENSITY: f64 = -0.4; // Additional bias for Electrine

const CHUNK_SIZE: i32 = 8;
const CHUNK_TILE_COUNT: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;
const VISIBLE_CHUNKS_RADIUS: i32 = 1; // How many chunks to render around the camera

const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 3.0;
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

    // Get world position from chunk position
    fn to_world_position(&self) -> Vec2 {
        Vec2::new(
            self.x as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
            self.y as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
        )
    }

    // Get chunk position from world position
    fn from_world_position(world_pos: Vec2) -> Self {
        Self {
            x: (world_pos.x / (CHUNK_SIZE as f32 * TILE_SIZE)).floor() as i32,
            y: (world_pos.y / (CHUNK_SIZE as f32 * TILE_SIZE)).floor() as i32,
        }
    }
}

// Define a component to identify chunk entities
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
    Extractor {
        position: Position,
        direction: Direction,
        extractor_type: ExtractorType,
    },
    Factory {
        position: Position,
        direction: Direction,
        factory_type: FactoryType,
        inventory: HashMap<Item, u32>,
    },

    Storage {
        position: Position,
        direction: Direction,
        inventory: HashMap<Item, u32>,
        storage_type: StorageType,
    },
    // todo: add portal
}

#[derive(Serialize, Deserialize, Encode, Decode)]
struct SerializableWorld {
    tiles: HashMap<u64, (SerializableTile, (u32, u32))>,
    resources: HashMap<(u32, u32), u32>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
enum FactoryType {
    RigtoriumSmelter,
    FlextoriumFabricator,
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
            FactoryType::ConveyorConstructor => {
                let mut hashmap = HashMap::new();
                hashmap.insert(Item::Flextorium, 4);
                hashmap.insert(Item::Rigtorium, 2);
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
            FactoryType::ConveyorConstructor => {
                let mut inputs = HashMap::new();
                inputs.insert(Item::Flextorium, 2);
                inputs.insert(Item::Rigtorium, 1);
                Recipe {
                    inputs,
                    output: Item::Conveyor,
                }
            }
        }
    }
    fn sprite(&self) -> String {
        match self {
            FactoryType::RigtoriumSmelter => {
                "embedded://textures/tiles/factories/rigtorium_smelter.png"
            }
            FactoryType::FlextoriumFabricator => {
                "embedded://textures/tiles/factories/flextorium_fabricator.png"
            }
            FactoryType::ConveyorConstructor => {
                "embedded://textures/tiles/factories/conveyor_constructor.png"
            }
        }
        .to_string()
    }
}

#[derive(Debug, Clone)]
enum Action {
    Move(Position, Position, Item),
    Produce(Position),
    Teleport(Position, Item),
}

#[derive(PartialEq, Eq, Clone, Hash, Debug, Copy, Deserialize, Serialize, Encode, Decode)]
enum Item {
    RawFlextorium,
    RawRigtorium,
    Flextorium,
    Rigtorium,
    Electrine,
    Product,
    Conveyor,
}

impl Item {
    fn sprite(&self) -> String {
        match self {
            Item::RawFlextorium => "embedded://textures/items/raw_flextorium.png",
            Item::RawRigtorium => "embedded://textures/items/raw_rigtorium.png",
            Item::Flextorium => "embedded://textures/items/flextorium.png",
            Item::Rigtorium => "embedded://textures/items/rigtorium.png",
            Item::Electrine => "embedded://textures/items/electrine.png",
            Item::Product => "embedded://textures/items/product.png",
            Item::Conveyor => "embedded://textures/items/conveyor.png",
        }
        .to_string()
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
    tile_type: (u32, u32),
    preview_entity: Option<Entity>,
    zoom_level: f32, // Add this field to track camera zoom
}

impl Default for Placer {
    fn default() -> Self {
        Self {
            direction: Direction::Up,
            tile_type: (1, 1),
            preview_entity: None,
            zoom_level: 1.0, // Default zoom level (1.0 = 100%)
        }
    }
}

#[derive(Resource)]
struct WorldRes {
    tiles: HashMap<Position, (Box<dyn Tile>, (u32, u32))>,
    terrain: HashMap<Position, TerrainTileType>,
    loaded_chunks: HashSet<ChunkPosition>,
    resources: HashMap<(u32, u32), u32>,
    world_seed: u32,
    tick_timer: Timer,
    tick_count: i32,
    actions: Vec<Action>,
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
                        } else if let Some(extractor) = tile.as_any().downcast_ref::<Extractor>() {
                            SerializableTile::Extractor {
                                position: extractor.position,
                                direction: extractor.direction,
                                extractor_type: extractor.extractor_type,
                            }
                        } else if let Some(factory) = tile.as_any().downcast_ref::<Factory>() {
                            SerializableTile::Factory {
                                position: factory.position,
                                direction: factory.direction,
                                factory_type: factory.factory_type,
                                inventory: factory.inventory.clone(),
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
        let mut loaded_chunks = HashSet::new();

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
                } => Box::new(Extractor {
                    position,
                    direction,
                    extractor_type,
                }),
                SerializableTile::Factory {
                    position,
                    direction,
                    factory_type,
                    inventory,
                } => Box::new(Factory {
                    position,
                    direction,
                    factory_type,
                    inventory,
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
            };

            tiles.insert(pos, (boxed_tile, id));
        }

        if terrain.is_empty() {
            // Create separate noise generators for each resource type
            let seed = serializable_world.world_seed;
            let rigtorium_noise = Perlin::new(seed);
            let flextorium_noise = Perlin::new(seed.wrapping_add(1));
            let electrine_noise = Perlin::new(seed.wrapping_add(2));

            for x in -TERRAIN_GEN_RANGE..=TERRAIN_GEN_RANGE {
                for y in -TERRAIN_GEN_RANGE..=TERRAIN_GEN_RANGE {
                    // Calculate separate noise values with different scales for each resource type
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

                    // Determine terrain type based on highest noise value that exceeds threshold
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
                        TerrainTileType::Stone // Default is stone
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
        resources.insert((2, 1), 10);
        resources.insert((2, 2), 10);
        resources.insert((2, 3), 10);
        resources.insert((3, 1), 10);
        resources.insert((3, 2), 10);
        resources.insert((3, 3), 10);
        resources.insert((4, 1), 10);
        resources.insert((5, 1), 10);

        world.unwrap_or(WorldRes {
            tiles: HashMap::new(),
            terrain: HashMap::new(),
            loaded_chunks: HashSet::new(),
            resources,
            world_seed: 59,
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
    fn spawn_item(&self) -> Option<Item> {
        match self {
            ExtractorType::RawRigtorium => Some(Item::RawRigtorium),
            ExtractorType::RawFlextorium => Some(Item::RawFlextorium),
            ExtractorType::Electrine => Some(Item::Electrine),
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
}

impl Tile for Extractor {
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        if world.tick_count % self.extractor_type.interval() == 0
            && world.terrain.get(&self.position) == Some(&self.extractor_type.terrain())
        {
            return Some(Action::Produce(self.position));
        }
        None
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
}

impl Factory {
    fn can_produce(&self) -> bool {
        let recipe = self.factory_type.recipe();
        recipe
            .inputs
            .iter()
            .all(|(item, &qty_required)| self.inventory.get(item).unwrap_or(&0) >= &qty_required)
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
    fn get_produce_item(&self) -> Option<Item> {
        let recipe = self.factory_type.recipe();
        if self.can_produce() {
            Some(recipe.output)
        } else {
            None
        }
    }
}

impl Tile for Factory {
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
                if self.can_produce() && conveyor.item.is_none() {
                    return Some(Action::Produce(self.position));
                }
            }
        }

        None
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
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        if let Some(item) = self.item {
            return Some(Action::Teleport(self.position, item));
        }

        None
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
                    // Tells Wasm to resize the window according to the available canvas
                    fit_canvas_to_parent: true,
                    // Tells Wasm not to override default event handling, like F5, Ctrl+R etc.
                    prevent_default_event_handling: false,

                    ..default()
                }),
                ..default()
            }),
            EmbeddedAssetPlugin {
                mode: PluginMode::AutoLoad,
            },
        ))
        .insert_resource(WorldRes::default())
        .insert_resource(Placer::default())
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
            ),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut world: ResMut<WorldRes>) {
    commands.spawn(Camera2d);

    for (pos, terrain) in world.terrain.iter() {
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
    }

    if world.tiles.is_empty() {
        world.tiles.insert(
            Position::new(-3, -3),
            (
                Box::new(Extractor {
                    position: Position::new(-3, -3),
                    direction: Direction::Right,
                    extractor_type: ExtractorType::RawRigtorium,
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
    camera_query: Query<&Transform, With<Camera2d>>,
    chunk_query: Query<(Entity, &TerrainChunk)>,
    asset_server: Res<AssetServer>,
) {
    if let Ok(camera_transform) = camera_query.single() {
        let camera_pos = camera_transform.translation.truncate();

        // Get the chunk the camera is in
        let camera_chunk = ChunkPosition::from_world_position(camera_pos);

        // Calculate which chunks should be visible
        let mut visible_chunks = HashSet::new();
        for x in
            (camera_chunk.x - VISIBLE_CHUNKS_RADIUS)..(camera_chunk.x + VISIBLE_CHUNKS_RADIUS + 1)
        {
            for y in (camera_chunk.y - VISIBLE_CHUNKS_RADIUS)
                ..(camera_chunk.y + VISIBLE_CHUNKS_RADIUS + 1)
            {
                visible_chunks.insert(ChunkPosition::new(x, y));
            }
        }

        // Unload chunks that are no longer visible
        let mut chunks_to_unload = HashSet::new();
        for &loaded_chunk in &world.loaded_chunks {
            if !visible_chunks.contains(&loaded_chunk) {
                chunks_to_unload.insert(loaded_chunk);
            }
        }

        for chunk_pos in &chunks_to_unload {
            // Find and despawn the chunk entity
            for (entity, chunk) in &chunk_query {
                if chunk.position == *chunk_pos {
                    commands.entity(entity).despawn();
                    break;
                }
            }
            world.loaded_chunks.remove(chunk_pos);
        }

        // Load new chunks that have become visible
        for chunk_pos in &visible_chunks {
            if !world.loaded_chunks.contains(chunk_pos) {
                // Generate and spawn the chunk
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
    // Create a parent entity for the chunk
    let chunk_entity = commands
        .spawn((TerrainChunk {
            position: chunk_pos,
        },))
        .id();

    // Generate terrain for all tiles in the chunk
    let chunk_world_pos = chunk_pos.to_world_position();

    // Create noise generators with different seeds
    let seed = world.world_seed;
    let rigtorium_noise = Perlin::new(seed);
    let flextorium_noise = Perlin::new(seed.wrapping_add(1));
    let electrine_noise = Perlin::new(seed.wrapping_add(2));

    // Spawn all the terrain tiles as children of the chunk entity
    commands.entity(chunk_entity).with_children(|parent| {
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                // Calculate world position for this tile
                let world_x = chunk_pos.x * CHUNK_SIZE + x;
                let world_y = chunk_pos.y * CHUNK_SIZE + y;
                let pos = Position::new(world_x, world_y);

                // Calculate noise values for each resource type
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

                // Determine terrain type
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

                // Store terrain type in world
                world.terrain.insert(pos, terrain_type);

                // Choose texture based on terrain type
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

                // Spawn the terrain tile sprite as a child of the chunk
                parent.spawn((
                    Sprite::from_image(asset_server.load(texture_path)),
                    Transform {
                        translation: Vec3::new(
                            world_x as f32 * TILE_SIZE,
                            world_y as f32 * TILE_SIZE,
                            -1.0,
                        ),
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
                    if let Some(tile) = world.tiles.get_mut(&end) {
                        if let Some(end_conveyor) = tile.0.as_any_mut().downcast_mut::<Conveyor>() {
                            if end_conveyor.item.is_none() {
                                end_conveyor.item = Some(item);
                                if let Some(tile) = world.tiles.get_mut(&start) {
                                    if let Some(start_conveyor) =
                                        tile.0.as_any_mut().downcast_mut::<Conveyor>()
                                    {
                                        start_conveyor.item = None;
                                    }
                                }
                            }
                        } else if let Some(factory) = tile.0.as_any_mut().downcast_mut::<Factory>()
                        {
                            if factory.factory_type.capacity().get(&item).unwrap_or(&0_u32)
                                > factory.inventory.get(&item).unwrap_or(&0_u32)
                            {
                                *factory.inventory.entry(item).or_insert(0) += 1;
                                if let Some(start_tile) = world.tiles.get_mut(&start) {
                                    if let Some(start_conveyor) =
                                        start_tile.0.as_any_mut().downcast_mut::<Conveyor>()
                                    {
                                        start_conveyor.item = None;
                                    }
                                }
                            }
                        } else if let Some(end_portal) =
                            tile.0.as_any_mut().downcast_mut::<Portal>()
                        {
                            if end_portal.item.is_none() {
                                end_portal.item = Some(item);
                                if let Some(tile) = world.tiles.get_mut(&start) {
                                    if let Some(start_conveyor) =
                                        tile.0.as_any_mut().downcast_mut::<Conveyor>()
                                    {
                                        start_conveyor.item = None;
                                    }
                                }
                            }
                        }
                    }
                }
                Action::Produce(position) => {
                    if let Some(tile) = world.tiles.get_mut(&position) {
                        if let Some(factory) = tile.0.as_any_mut().downcast_mut::<Factory>() {
                            if let Some(produced_item) = factory.produce() {
                                let mut end_position = factory.position;
                                match factory.direction {
                                    Direction::Up => end_position.y += 1,
                                    Direction::Down => end_position.y -= 1,
                                    Direction::Left => end_position.x -= 1,
                                    Direction::Right => end_position.x += 1,
                                }

                                if let Some(tile) = world.tiles.get_mut(&end_position) {
                                    if let Some(conveyor) =
                                        tile.0.as_any_mut().downcast_mut::<Conveyor>()
                                    {
                                        if conveyor.item.is_none() {
                                            conveyor.item = Some(produced_item);
                                        }
                                    }
                                }
                            }
                        } else if let Some(extractor) =
                            tile.0.as_any_mut().downcast_mut::<Extractor>()
                        {
                            let mut end_position = extractor.position;
                            let item = extractor.extractor_type.spawn_item();
                            match extractor.direction {
                                Direction::Up => end_position.y += 1,
                                Direction::Down => end_position.y -= 1,
                                Direction::Left => end_position.x -= 1,
                                Direction::Right => end_position.x += 1,
                            }

                            if let Some(tiles) = world.tiles.get_mut(&end_position) {
                                if let Some(conveyor) =
                                    tiles.0.as_any_mut().downcast_mut::<Conveyor>()
                                {
                                    if conveyor.item.is_none() {
                                        conveyor.item = item;
                                    }
                                }
                            }
                        }
                    }
                }
                Action::Teleport(position, item) => {
                    if let Some(tiles) = world.tiles.get_mut(&position) {
                        if let Some(portal) = tiles.0.as_any_mut().downcast_mut::<Portal>() {
                            portal.item = None;
                            match item {
                                Item::Conveyor => *world.resources.entry((1, 1)).or_insert(0) += 1,
                                _ => {}
                            }
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

        world.actions = sort_moves_topologically(next);
        world.actions.reverse();

        let mut moved: Vec<Position> = Vec::new();

        for action in &world.actions {
            match action {
                Action::Move(start, end, item) => {
                    if let Some(tile) = world.tiles.get(end) {
                        if let Some(end_conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
                            if end_conveyor.item.is_none() || moved.contains(end) {
                                moved.push(*start);

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
                                moved.push(*start);

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
                                moved.push(*start);

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
                Action::Produce(position) => {
                    if let Some(tile) = world.tiles.get(position) {
                        if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
                            if let Some(produced_item) = factory.get_produce_item() {
                                let mut end_position = factory.position;
                                match factory.direction {
                                    Direction::Up => end_position.y += 1,
                                    Direction::Down => end_position.y -= 1,
                                    Direction::Left => end_position.x -= 1,
                                    Direction::Right => end_position.x += 1,
                                }

                                if let Some(tile) = world.tiles.get(&end_position) {
                                    if let Some(conveyor) =
                                        tile.0.as_any().downcast_ref::<Conveyor>()
                                    {
                                        if conveyor.item.is_none() || moved.contains(&end_position)
                                        {
                                            moved.push(*position);

                                            let start_pos = Vec3::new(
                                                position.x as f32 * TILE_SIZE,
                                                position.y as f32 * TILE_SIZE,
                                                1.0,
                                            );
                                            let end_pos = Vec3::new(
                                                end_position.x as f32 * TILE_SIZE,
                                                end_position.y as f32 * TILE_SIZE,
                                                1.0,
                                            );
                                            commands.spawn((
                                                ItemAnimation {
                                                    start_pos,
                                                    end_pos,
                                                    timer: Timer::from_seconds(
                                                        TICK_LENGTH,
                                                        TimerMode::Once,
                                                    ),
                                                },
                                                Sprite::from_image(
                                                    asset_server.load(produced_item.sprite()),
                                                ),
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
                        } else if let Some(extractor) = tile.0.as_any().downcast_ref::<Extractor>()
                        {
                            let mut end_position = extractor.position;
                            let item = extractor.extractor_type.spawn_item();
                            match extractor.direction {
                                Direction::Up => end_position.y += 1,
                                Direction::Down => end_position.y -= 1,
                                Direction::Left => end_position.x -= 1,
                                Direction::Right => end_position.x += 1,
                            }
                            if let Some(tiles) = world.tiles.get(&end_position) {
                                if let Some(conveyor) = tiles.0.as_any().downcast_ref::<Conveyor>()
                                {
                                    if conveyor.item.is_none() || moved.contains(&end_position) {
                                        moved.push(*position);
                                        let start_pos = Vec3::new(
                                            position.x as f32 * TILE_SIZE,
                                            position.y as f32 * TILE_SIZE,
                                            1.0,
                                        );
                                        let end_pos = Vec3::new(
                                            end_position.x as f32 * TILE_SIZE,
                                            end_position.y as f32 * TILE_SIZE,
                                            1.0,
                                        );
                                        commands.spawn((
                                            ItemAnimation {
                                                start_pos,
                                                end_pos,
                                                timer: Timer::from_seconds(
                                                    TICK_LENGTH,
                                                    TimerMode::Once,
                                                ),
                                            },
                                            Sprite::from_image(if let Some(unwraped_item) = item {
                                                asset_server.load(unwraped_item.sprite())
                                            } else {
                                                asset_server.load("textures/items/none.png")
                                            }),
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
                    }
                }
                Action::Teleport(_, _) => {}
            }
        }
        if let Err(err) = world.save("savegame.ff") {
            eprintln!("Error saving game: {}", err);
        }
    }
}

fn sort_moves_topologically(actions: Vec<Action>) -> Vec<Action> {
    let mut from_map: HashMap<Position, usize> = HashMap::new();
    for (i, action) in actions.iter().enumerate() {
        if let Action::Move(from, _, _) = action {
            from_map.insert(*from, i);
        }
    }

    let mut graph: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut in_degree: HashMap<usize, usize> = HashMap::new();

    for (i, action) in actions.iter().enumerate() {
        if let Action::Move(_, to, _) = action {
            if let Some(&dep) = from_map.get(to) {
                graph.entry(i).or_default().push(dep);
                *in_degree.entry(dep).or_insert(0) += 1;
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

                // Determine which texture to use based on incoming conveyors
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
            } else {
                sprite.color = css::GRAY.into();
            }
        } else {
            commands.entity(entity).despawn_recursive();
        }
    }
}
fn determine_conveyor_texture(world: &WorldRes, conveyor: &Conveyor) -> &'static str {
    let pos = conveyor.position;
    let dir = conveyor.direction;

    // Calculate positions to check (behind, left, right relative to conveyor direction)
    let (behind_pos, left_pos, right_pos) = match dir {
        Direction::Up => (
            Position::new(pos.x, pos.y - 1), // behind = down
            Position::new(pos.x - 1, pos.y), // left
            Position::new(pos.x + 1, pos.y), // right
        ),
        Direction::Down => (
            Position::new(pos.x, pos.y + 1), // behind = up
            Position::new(pos.x + 1, pos.y), // left (from perspective of down-facing conveyor)
            Position::new(pos.x - 1, pos.y), // right (from perspective of down-facing conveyor)
        ),
        Direction::Left => (
            Position::new(pos.x + 1, pos.y), // behind = right
            Position::new(pos.x, pos.y - 1), // left (from perspective of left-facing conveyor)
            Position::new(pos.x, pos.y + 1), // right (from perspective of left-facing conveyor)
        ),
        Direction::Right => (
            Position::new(pos.x - 1, pos.y), // behind = left
            Position::new(pos.x, pos.y + 1), // left (from perspective of right-facing conveyor)
            Position::new(pos.x, pos.y - 1), // right (from perspective of right-facing conveyor)
        ),
    };

    // Check if conveyors at these positions are pointing to current conveyor
    let has_behind = is_conveyor_pointing_to(world, behind_pos, dir);
    let has_left = is_conveyor_pointing_to(world, left_pos, rotate_direction_clockwise(dir));
    let has_right =
        is_conveyor_pointing_to(world, right_pos, rotate_direction_counterclockwise(dir));

    // Select texture based on incoming conveyors
    match (has_behind, has_left, has_right) {
        (true, true, false) => "embedded://textures/tiles/conveyors/left_back.png",
        (true, false, true) => "embedded://textures/tiles/conveyors/right_back.png",
        (false, true, true) => "embedded://textures/tiles/conveyors/sides.png",
        (true, false, false) => "embedded://textures/tiles/conveyors/back.png",
        (false, true, false) => "embedded://textures/tiles/conveyors/left.png",
        (false, false, true) => "embedded://textures/tiles/conveyors/right.png",
        (true, true, true) => "embedded://textures/tiles/conveyors/all.png",
        _ => "embedded://textures/tiles/conveyors/back.png", // Default for all other cases
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
        } else if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
            return factory.direction == pointing_direction;
        } else if let Some(extractor) = tile.0.as_any().downcast_ref::<Extractor>() {
            return extractor.direction == pointing_direction;
        }
    }
    false
}

fn get_tile_texture(tile_type: (u32, u32)) -> String {
    match tile_type {
        (0, 1) => "embedded://textures/tiles/none.png",
        (1, 1) => "embedded://textures/tiles/conveyors/back.png",
        (2, 1) => "embedded://textures/tiles/factories/rigtorium_smelter.png",
        (2, 2) => "embedded://textures/tiles/factories/flextorium_fabricator.png",
        (2, 3) => "embedded://textures/tiles/factories/conveyor_constructor.png",
        (3, 1) => "embedded://textures/tiles/extractors/raw_rigtorium.png",
        (3, 2) => "embedded://textures/tiles/extractors/raw_flextorium.png",
        (3, 3) => "embedded://textures/tiles/extractors/electrine.png",
        (4, 1) => "embedded://textures/tiles/portal.png",
        (5, 1) => "embedded://textures/tiles/storage.png",
        _ => "embedded://textures/tiles/conveyors/back.png",
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
) {
    if keyboard_input.pressed(KeyCode::Digit1) {
        placer.tile_type = (1, 1);
    }
    if keyboard_input.pressed(KeyCode::Digit2) {
        placer.tile_type = (2, 1);
    }
    if keyboard_input.pressed(KeyCode::Digit3) {
        placer.tile_type = (3, 1);
    }
    if keyboard_input.pressed(KeyCode::Digit4) {
        placer.tile_type = (3, 2);
    }
    if keyboard_input.pressed(KeyCode::Digit5) {
        placer.tile_type = (3, 3);
    }
    if keyboard_input.pressed(KeyCode::Digit6) {
        placer.tile_type = (4, 3);
    }
    if keyboard_input.pressed(KeyCode::Digit7) {
        placer.tile_type = (2, 3);
    }
    if keyboard_input.pressed(KeyCode::Digit8) {
        placer.tile_type = (4, 1);
    }
    if keyboard_input.pressed(KeyCode::Digit0) {
        placer.tile_type = (2, 2);
    }

    for event in mouse_wheel_events.read() {
        if placer.tile_type == (0, 1) {
            // When not placing (empty hand) - use mouse wheel for zoom
            let zoom_delta = event.y * ZOOM_SPEED;
            placer.zoom_level = (placer.zoom_level + zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);

            // Apply zoom to camera
            if let Ok((_, mut transform)) = camera_query.get_single_mut() {
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

            if let Some(preview_entity) = placer.preview_entity {
                commands.entity(preview_entity).despawn();
            }

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
        }
    }

    if mouse_button_input.pressed(MouseButton::Left) {
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
                            }
                        }
                    } else {
                        if *world.resources.get(&tile_type).unwrap_or(&0) >= 1 {
                            *world.resources.entry(tile_type).or_insert(0) -= 1;

                            let new_tile = get_new_tile(tile_type, pos, direction);

                            world.tiles.insert(pos, new_tile);

                            commands
                                .spawn((
                                    Sprite::from_image(
                                        asset_server
                                            .load("embedded://textures/tiles/conveyors/back.png"),
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
                                            asset_server.load("embedded://textures/items/none.png"),
                                        ),
                                        Transform::from_scale(Vec3::splat(0.5)),
                                    ));
                                });
                        }
                    }
                }
            }
        }
    }
    if mouse_button_input.pressed(MouseButton::Right) {
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

                    if let Some(entry) = world.tiles.remove_entry(&pos) {
                        *world.resources.entry(entry.1.1).or_insert(0) += 1;
                    }
                }
            }
        }
    }
}

fn get_new_tile(
    tile_type: (u32, u32),
    position: Position,
    direction: Direction,
) -> (Box<dyn Tile>, (u32, u32)) {
    match tile_type {
        (1, 1) => (
            Box::new(Conveyor {
                position,
                direction,
                item: None,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (2, 1) => (
            Box::new(Factory {
                factory_type: FactoryType::RigtoriumSmelter,
                position,
                direction,
                inventory: HashMap::new(),
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (2, 2) => (
            Box::new(Factory {
                factory_type: FactoryType::FlextoriumFabricator,
                position,
                direction,
                inventory: HashMap::new(),
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (2, 3) => (
            Box::new(Factory {
                factory_type: FactoryType::ConveyorConstructor,
                position,
                direction,
                inventory: HashMap::new(),
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (3, 1) => (
            Box::new(Extractor {
                position,
                direction,
                extractor_type: ExtractorType::RawRigtorium,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (3, 2) => (
            Box::new(Extractor {
                position,
                direction,
                extractor_type: ExtractorType::RawFlextorium,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (3, 3) => (
            Box::new(Extractor {
                position,
                direction,
                extractor_type: ExtractorType::Electrine,
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
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
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
        camera.translation += direction.normalize_or_zero().extend(0.0) * CAMERA_SPEED;
    }
}
