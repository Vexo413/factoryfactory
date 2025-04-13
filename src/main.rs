use bevy::{color::palettes::css, input::mouse::MouseWheel, prelude::*, window::PrimaryWindow};
use bincode::{Decode, Encode, config};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use noise::{NoiseFn, Perlin};
use serde::{Deserialize, Serialize};

use std::{
    any::Any,
    collections::{HashMap, HashSet},
    f32::consts::{FRAC_PI_2, PI},
    ffi::OsStr,
    fmt::Debug,
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

const TILE_SIZE: f32 = 64.0;
const ITEM_SIZE: f32 = 32.0;
const IMAGE_SIZE: f32 = 128.0;
const TICK_LENGTH: f32 = 1.0;
const CAMERA_SPEED: f32 = 5.0;

// Serializable versions of our game objects
#[derive(Serialize, Deserialize, Encode, Decode)]
enum SerializableTile {
    Conveyor {
        position: Position,
        direction: Direction,
        item: Item,
    },
    Extractor {
        position: Position,
        direction: Direction,
        spawn_item: Item,
        interval: i32,
        required_terrain: TerrainTileType,
    },
    Factory {
        position: Position,
        direction: Direction,
        factory_type: FactoryType,
        inventory: HashMap<Item, u32>,
        capacity: HashMap<Item, u32>,
    },
}

#[derive(Serialize, Deserialize, Encode, Decode)]
struct SerializableWorld {
    // Use a u64 key instead of String for position
    tiles: HashMap<u64, (SerializableTile, u32)>,
    resources: HashMap<u32, u32>,
    world_seed: u32,
    tick_count: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
enum TerrainTileType {
    Grass,
    Dirt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
enum FactoryType {
    Assembler,
}

#[derive(Debug, Clone)]
enum Action {
    Move(Position, Position, Item),
    Produce(Position),
    None,
}

#[derive(PartialEq, Eq, Clone, Hash, Debug, Copy, Deserialize, Serialize, Encode, Decode)]
enum Item {
    None,
    Wood,
    Stone,
    Product,
}

#[derive(Debug, Clone)]
struct Recipe {
    inputs: HashMap<Item, u32>,
    output: (Item, u32),
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
    fn to_string_key(&self) -> String {
        format!("{},{}", self.x, self.y)
    }

    // Parse a string key back to Position
    fn from_string_key(key: &str) -> Option<Self> {
        let parts: Vec<&str> = key.split(',').collect();
        if parts.len() != 2 {
            return None;
        }

        let x = parts[0].parse::<i32>().ok()?;
        let y = parts[1].parse::<i32>().ok()?;

        Some(Position::new(x, y))
    }
    fn to_key(&self) -> u64 {
        // Use 32 bits for each coordinate (can handle ±2 billion)
        ((self.x as u64) & 0xFFFFFFFF) | (((self.y as u64) & 0xFFFFFFFF) << 32)
    }

    // Extract x,y from the packed key
    fn from_key(key: u64) -> Self {
        let x = (key & 0xFFFFFFFF) as i32;
        let y = ((key >> 32) & 0xFFFFFFFF) as i32;
        Position::new(x, y)
    }
}

#[derive(Resource)]
struct ConveyorPlacer {
    direction: Direction,
    tile_type: u32,
    preview_entity: Option<Entity>,
}

impl Default for ConveyorPlacer {
    fn default() -> Self {
        Self {
            direction: Direction::Up,
            tile_type: 1,
            preview_entity: None,
        }
    }
}

#[derive(Resource)]
struct WorldRes {
    tiles: HashMap<Position, (Box<dyn Tile>, u32)>,
    terrain: HashMap<Position, TerrainTileType>,
    resources: HashMap<u32, u32>,
    world_seed: u32, // Store the seed for terrain generation
    tick_timer: Timer,
    tick_count: i32,
    actions: Vec<Action>,
}

impl WorldRes {
    // Updated save function for binary serialization with bincode 2.0.1
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), io::Error> {
        // Convert WorldRes to SerializableWorld with string keys
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
                                spawn_item: extractor.spawn_item,
                                interval: extractor.interval,
                                required_terrain: extractor.required_terrain,
                            }
                        } else if let Some(factory) = tile.as_any().downcast_ref::<Factory>() {
                            SerializableTile::Factory {
                                position: factory.position,
                                direction: factory.direction,
                                factory_type: factory.factory_type,
                                inventory: factory.inventory.clone(),
                                capacity: factory.capacity.clone(),
                            }
                        } else {
                            SerializableTile::Conveyor {
                                position: *pos,
                                direction: Direction::Up,
                                item: Item::None,
                            }
                        };
                    (pos.to_key(), (serializable_tile, *id))
                })
                .collect(),
            resources: self.resources.clone(),
            world_seed: self.world_seed,
            tick_count: self.tick_count,
        };

        // Configure bincode with default settings
        let config = config::standard().with_fixed_int_encoding().with_no_limit();

        // Serialize with bincode
        let serialized = bincode::encode_to_vec(&serializable_world, config)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Compress with flate2
        let file = File::create(path)?;
        let mut encoder = DeflateEncoder::new(file, Compression::best());
        encoder.write_all(&serialized)?;
        encoder.finish()?;

        Ok(())
    }

    // Updated load function for binary serialization with bincode 2.0.1
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        // Read the entire file into a buffer
        let file = File::open(path)?;

        let mut decoder = DeflateDecoder::new(file);
        let mut buffer = Vec::new();
        decoder.read_to_end(&mut buffer)?;

        // Configure bincode with default settings
        let config = config::standard().with_fixed_int_encoding().with_no_limit();

        // Deserialize from binary
        let (serializable_world, _): (SerializableWorld, _) =
            bincode::decode_from_slice(&buffer, config)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Convert SerializableWorld back to WorldRes
        let mut tiles = HashMap::new();
        let mut terrain = HashMap::new();

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
                    spawn_item,
                    interval,
                    required_terrain,
                } => Box::new(Extractor {
                    position,
                    direction,
                    spawn_item,
                    interval,
                    required_terrain,
                }),
                SerializableTile::Factory {
                    position,
                    direction,
                    factory_type,
                    inventory,
                    capacity,
                } => Box::new(Factory {
                    position,
                    direction,
                    factory_type,
                    inventory,
                    capacity,
                }),
            };

            tiles.insert(pos, (boxed_tile, id));
        }

        // Regenerate terrain based on the seed
        let perlin = Perlin::new(serializable_world.world_seed);
        let noise_scale = 0.1;

        for x in -20..=20 {
            for y in -20..=20 {
                let noise_val = perlin.get([x as f64 * noise_scale, y as f64 * noise_scale]);
                let terrain_type = if noise_val > 0.0 {
                    TerrainTileType::Grass
                } else {
                    TerrainTileType::Dirt
                };
                terrain.insert(Position::new(x, y), terrain_type);
            }
        }

        Ok(WorldRes {
            tiles,
            terrain,
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
        resources.insert(1, 10);
        resources.insert(2, 1);

        return world.unwrap_or(WorldRes {
            tiles: HashMap::new(),
            terrain: HashMap::new(),
            resources,
            world_seed: 59, // Default seed value
            tick_timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Repeating),
            tick_count: 0,
            actions: Vec::new(),
        });
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
    fn tick(&self, tiles: &WorldRes) -> Action;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Debug)]
struct Conveyor {
    position: Position,
    direction: Direction,
    item: Item,
}

impl Tile for Conveyor {
    fn tick(&self, world: &WorldRes) -> Action {
        let start_position = self.position;
        let mut end_position = self.position;

        match self.direction {
            Direction::Up => end_position.y += 1,
            Direction::Down => end_position.y -= 1,
            Direction::Left => end_position.x -= 1,
            Direction::Right => end_position.x += 1,
        }

        if world.tiles.get(&end_position).is_some() {
            if self.item != Item::None {
                return Action::Move(start_position, end_position, self.item);
            }
        }

        Action::None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug)]
struct Extractor {
    position: Position,
    direction: Direction,
    spawn_item: Item,
    interval: i32,
    required_terrain: TerrainTileType,
}

impl Tile for Extractor {
    fn tick(&self, world: &WorldRes) -> Action {
        if world.tick_count % self.interval == 0
            && *world.terrain.get(&self.position).unwrap() == self.required_terrain
        {
            return Action::Produce(self.position);
        }
        Action::None
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
    capacity: HashMap<Item, u32>,
}

impl Factory {
    fn can_produce(&self) -> bool {
        let recipe = recipe_for(self.factory_type);
        recipe
            .inputs
            .iter()
            .all(|(item, &qty_required)| self.inventory.get(item).unwrap_or(&0) >= &qty_required)
    }

    fn produce(&mut self) -> Option<(Item, u32)> {
        let recipe = recipe_for(self.factory_type);
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
    fn get_produce_item(&self) -> Option<(Item, u32)> {
        let recipe = recipe_for(self.factory_type);
        if self.can_produce() {
            Some(recipe.output)
        } else {
            None
        }
    }
}

impl Tile for Factory {
    fn tick(&self, world: &WorldRes) -> Action {
        let mut end_position = self.position;

        match self.direction {
            Direction::Up => end_position.y += 1,
            Direction::Down => end_position.y -= 1,
            Direction::Left => end_position.x -= 1,
            Direction::Right => end_position.x += 1,
        }
        if let Some(tile) = world.tiles.get(&end_position) {
            if let Some(conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
                if self.can_produce() && conveyor.item == Item::None {
                    return Action::Produce(self.position);
                }
            }
        }

        Action::None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn recipe_for(factory_type: FactoryType) -> Recipe {
    match factory_type {
        FactoryType::Assembler => {
            let mut inputs = HashMap::new();
            inputs.insert(Item::Wood, 1);
            inputs.insert(Item::Stone, 1);
            Recipe {
                inputs,
                output: (Item::Product, 1),
            }
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(WorldRes::default())
        .insert_resource(ConveyorPlacer::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
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
    commands.spawn(Camera2d::default());

    // Only generate terrain if it's empty (new world)
    if world.terrain.is_empty() {
        let perlin = Perlin::new(world.world_seed);
        let noise_scale = 0.1;

        for x in -20..=20 {
            for y in -20..=20 {
                let noise_val = perlin.get([x as f64 * noise_scale, y as f64 * noise_scale]);
                let terrain_type = if noise_val > 0.0 {
                    TerrainTileType::Grass
                } else {
                    TerrainTileType::Dirt
                };
                world.terrain.insert(Position::new(x, y), terrain_type);
            }
        }
    }

    // Spawn terrain visuals
    for (pos, terrain) in world.terrain.iter() {
        let texture_path = match terrain {
            TerrainTileType::Grass => "textures/terrain/grass.png",
            TerrainTileType::Dirt => "textures/terrain/dirt.png",
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

    // Add default tiles if it's a new world
    if world.tiles.is_empty() {
        world.tiles.insert(
            Position::new(-3, -3),
            (
                Box::new(Extractor {
                    interval: 5,
                    position: Position::new(-3, -3),
                    spawn_item: Item::Stone,
                    direction: Direction::Right,
                    required_terrain: TerrainTileType::Dirt,
                }),
                3,
            ),
        );
        world.tiles.insert(
            Position::new(3, 3),
            (
                Box::new(Extractor {
                    interval: 5,
                    position: Position::new(3, 3),
                    spawn_item: Item::Wood,
                    direction: Direction::Left,
                    required_terrain: TerrainTileType::Grass,
                }),
                3,
            ),
        );
    }

    // Spawn tile visuals
    for (pos, _) in world.tiles.iter() {
        commands
            .spawn((
                Sprite::from_image(asset_server.load("textures/tiles/belt.png")),
                Transform {
                    translation: Vec3::new(pos.x as f32 * TILE_SIZE, pos.y as f32 * TILE_SIZE, 0.0),
                    scale: Vec3::splat(TILE_SIZE / IMAGE_SIZE),
                    ..Default::default()
                },
                TileSprite { pos: *pos },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Sprite::from_image(asset_server.load("textures/items/none.png")),
                    Transform::from_scale(Vec3::splat(0.5)),
                ));
            });
    }
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
                            if end_conveyor.item == Item::None {
                                end_conveyor.item = item;
                                if let Some(tile) = world.tiles.get_mut(&start) {
                                    if let Some(start_conveyor) =
                                        tile.0.as_any_mut().downcast_mut::<Conveyor>()
                                    {
                                        start_conveyor.item = Item::None;
                                    }
                                }
                            }
                        } else if let Some(factory) = tile.0.as_any_mut().downcast_mut::<Factory>()
                        {
                            if factory.capacity.get(&item).unwrap_or(&0_u32)
                                > factory.inventory.get(&item).unwrap_or(&0_u32)
                            {
                                *factory.inventory.entry(item).or_insert(0) += 1;
                                if let Some(start_conveyor) = world
                                    .tiles
                                    .get_mut(&start)
                                    .unwrap()
                                    .0
                                    .as_any_mut()
                                    .downcast_mut::<Conveyor>()
                                {
                                    start_conveyor.item = Item::None;
                                }
                            }
                        }
                    }
                }
                Action::Produce(position) => {
                    if let Some(factory) = world
                        .tiles
                        .get_mut(&position)
                        .unwrap()
                        .0
                        .as_any_mut()
                        .downcast_mut::<Factory>()
                    {
                        if let Some((produced_item, _produced_qty)) = factory.produce() {
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
                                    if conveyor.item == Item::None {
                                        conveyor.item = produced_item;
                                    }
                                }
                            }
                        }
                    } else if let Some(extractor) = world
                        .tiles
                        .get_mut(&position)
                        .unwrap()
                        .0
                        .as_any_mut()
                        .downcast_mut::<Extractor>()
                    {
                        let mut end_position = extractor.position;
                        let item = extractor.spawn_item;
                        match extractor.direction {
                            Direction::Up => end_position.y += 1,
                            Direction::Down => end_position.y -= 1,
                            Direction::Left => end_position.x -= 1,
                            Direction::Right => end_position.x += 1,
                        }

                        if let Some(tiles) = world.tiles.get_mut(&end_position) {
                            if let Some(conveyor) = tiles.0.as_any_mut().downcast_mut::<Conveyor>()
                            {
                                if conveyor.item == Item::None {
                                    conveyor.item = item;
                                }
                            }
                        }
                    }
                }
                Action::None => {}
            }
        }

        let mut next = Vec::new();

        for tile in world.tiles.values() {
            let action = tile.0.tick(&world);
            next.push(action);
        }

        world.actions = sort_moves_topologically(next);
        world.actions.reverse();

        let mut moved: Vec<Position> = Vec::new();

        for action in &world.actions {
            match action {
                Action::Move(start, end, item) => {
                    if let Some(tile) = world.tiles.get(&end) {
                        if let Some(end_conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
                            if end_conveyor.item == Item::None || moved.contains(&end) {
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
                                    Sprite::from_image(asset_server.load(match item {
                                        Item::None => "textures/items/none.png",
                                        Item::Wood => "textures/items/wood.png",
                                        Item::Stone => "textures/items/stone.png",
                                        Item::Product => "textures/items/product.png",
                                    })),
                                    Transform {
                                        translation: start_pos,
                                        scale: Vec3::splat(ITEM_SIZE / IMAGE_SIZE),
                                        ..Default::default()
                                    },
                                ));
                            }
                        } else if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
                            if factory.capacity.get(&item).unwrap_or(&0_u32)
                                > factory.inventory.get(&item).unwrap_or(&0_u32)
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
                                    Sprite::from_image(asset_server.load(match item {
                                        Item::None => "textures/items/none.png",
                                        Item::Wood => "textures/items/wood.png",
                                        Item::Stone => "textures/items/stone.png",
                                        Item::Product => "textures/items/product.png",
                                    })),
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
                    if let Some(factory) = world
                        .tiles
                        .get(&position)
                        .unwrap()
                        .0
                        .as_any()
                        .downcast_ref::<Factory>()
                    {
                        if let Some((produced_item, _produced_qty)) = factory.get_produce_item() {
                            let mut end_position = factory.position;
                            match factory.direction {
                                Direction::Up => end_position.y += 1,
                                Direction::Down => end_position.y -= 1,
                                Direction::Left => end_position.x -= 1,
                                Direction::Right => end_position.x += 1,
                            }

                            if let Some(tile) = world.tiles.get(&end_position) {
                                if let Some(conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
                                    if conveyor.item == Item::None || moved.contains(&end_position)
                                    {
                                        moved.push(*position);
                                        // Spawn production animation
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
                                            Sprite::from_image(asset_server.load(
                                                match produced_item {
                                                    Item::None => "textures/items/none.png",
                                                    Item::Wood => "textures/items/wood.png",
                                                    Item::Stone => "textures/items/stone.png",
                                                    Item::Product => "textures/items/product.png",
                                                },
                                            )),
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
                    } else if let Some(extractor) = world
                        .tiles
                        .get(&position)
                        .unwrap()
                        .0
                        .as_any()
                        .downcast_ref::<Extractor>()
                    {
                        let mut end_position = extractor.position;
                        let item = extractor.spawn_item;
                        match extractor.direction {
                            Direction::Up => end_position.y += 1,
                            Direction::Down => end_position.y -= 1,
                            Direction::Left => end_position.x -= 1,
                            Direction::Right => end_position.x += 1,
                        }
                        if let Some(tiles) = world.tiles.get(&end_position) {
                            if let Some(conveyor) = tiles.0.as_any().downcast_ref::<Conveyor>() {
                                if conveyor.item == Item::None || moved.contains(&end_position) {
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
                                        Sprite::from_image(asset_server.load(match item {
                                            Item::None => "textures/items/none.png",
                                            Item::Wood => "textures/items/wood.png",
                                            Item::Stone => "textures/items/stone.png",
                                            Item::Product => "textures/items/product.png",
                                        })),
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

                _ => {}
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
                let entry = in_degree.get_mut(&dep).unwrap();
                *entry -= 1;
                if *entry == 0 {
                    queue.push(dep);
                }
            }
        }
    }

    for i in 0..actions.len() {
        if !visited.contains(&i) {
            sorted.push(actions[i].clone());
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

    // Collect positions that are currently animated
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
                sprite.image = asset_server.load("textures/tiles/belt.png");
                transform.rotation = match conveyor.direction {
                    Direction::Up => Quat::IDENTITY,
                    Direction::Down => Quat::from_rotation_z(PI),
                    Direction::Left => Quat::from_rotation_z(FRAC_PI_2),
                    Direction::Right => Quat::from_rotation_z(-FRAC_PI_2),
                };

                if let Ok(children) = children_query.get(entity) {
                    for &child in children.iter() {
                        if let Ok((mut child_sprite, mut child_transform)) =
                            child_sprite_query.get_mut(child)
                        {
                            // Update visibility based on animation state
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

                            child_sprite.image = match conveyor.item {
                                Item::None => asset_server.load("textures/items/none.png"),
                                Item::Wood => asset_server.load("textures/items/wood.png"),
                                Item::Stone => asset_server.load("textures/items/stone.png"),
                                Item::Product => asset_server.load("textures/items/product.png"),
                            };
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
                    match tile
                        .0
                        .as_any()
                        .downcast_ref::<Factory>()
                        .unwrap()
                        .factory_type
                    {
                        FactoryType::Assembler => "textures/tiles/assembler.png",
                    },
                );
                transform.rotation = match factory.direction {
                    Direction::Up => Quat::IDENTITY,
                    Direction::Down => Quat::from_rotation_z(PI),
                    Direction::Left => Quat::from_rotation_z(FRAC_PI_2),
                    Direction::Right => Quat::from_rotation_z(-FRAC_PI_2),
                };

                if let Ok(children) = children_query.get(entity) {
                    for &child in children.iter() {
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
                sprite.image = asset_server.load("textures/tiles/extractor.png");
                transform.rotation = match extractor.direction {
                    Direction::Up => Quat::IDENTITY,
                    Direction::Down => Quat::from_rotation_z(PI),
                    Direction::Left => Quat::from_rotation_z(FRAC_PI_2),
                    Direction::Right => Quat::from_rotation_z(-FRAC_PI_2),
                };
                if let Ok(children) = children_query.get(entity) {
                    for &child in children.iter() {
                        if let Ok((mut child_sprite, mut child_transform)) =
                            child_sprite_query.get_mut(child)
                        {
                            child_transform.translation = Vec3::new(0.0, 0.0, 1.0);
                            child_sprite.image = match extractor.spawn_item {
                                Item::None => asset_server.load("textures/items/none.png"),
                                Item::Wood => asset_server.load("textures/items/wood.png"),
                                Item::Stone => asset_server.load("textures/items/stone.png"),
                                Item::Product => asset_server.load("textures/items/product.png"),
                            };
                        }
                    }
                }
            } else {
                sprite.color = css::GRAY.into();
            }
        } else {
            commands.entity(entity).despawn_recursive();
        }
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
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut placer: ResMut<ConveyorPlacer>,
    mut world: ResMut<WorldRes>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if keyboard_input.pressed(KeyCode::Digit1) {
        placer.tile_type = 1;
    }
    if keyboard_input.pressed(KeyCode::Digit2) {
        placer.tile_type = 2;
    }
    if keyboard_input.pressed(KeyCode::Digit3) {
        placer.tile_type = 3;
    }

    for event in mouse_wheel_events.read() {
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

    let window = windows.single();
    if let Some(screen_pos) = window.cursor_position() {
        let (camera, camera_transform) = camera_query.single();
        let window_size = Vec2::new(window.width(), window.height());

        let mut ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
        ndc.y *= -1.0;
        let ndc_to_world = camera_transform.compute_matrix() * camera.clip_from_view().inverse();
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
        let world_pos: Vec2 = world_pos.truncate();

        let grid_x = (world_pos.x / TILE_SIZE).round() as i32;
        let grid_y = (world_pos.y / TILE_SIZE).round() as i32;
        let pos = Position::new(grid_x, grid_y);

        if let Some(preview_entity) = placer.preview_entity {
            commands.entity(preview_entity).despawn();
        }

        let texture_path = match placer.tile_type {
            0 => "textures/tiles/none.png",
            1 => "textures/tiles/belt.png",
            2 => "textures/tiles/assembler.png",
            3 => "textures/tiles/extractor.png",
            _ => "textures/tiles/belt.png",
        };

        let preview_entity = commands
            .spawn((
                Sprite {
                    image: asset_server.load(texture_path),
                    color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                    ..Default::default()
                },
                Transform {
                    translation: Vec3::new(pos.x as f32 * TILE_SIZE, pos.y as f32 * TILE_SIZE, 5.0),
                    scale: Vec3::splat(TILE_SIZE / IMAGE_SIZE),
                    rotation: match placer.direction {
                        Direction::Up => Quat::IDENTITY,
                        Direction::Down => Quat::from_rotation_z(PI),
                        Direction::Left => Quat::from_rotation_z(FRAC_PI_2),
                        Direction::Right => Quat::from_rotation_z(-FRAC_PI_2),
                    },
                    ..Default::default()
                },
            ))
            .id();

        placer.preview_entity = Some(preview_entity);
    }

    if mouse_button_input.pressed(MouseButton::Left) {
        let window = windows.single();
        if let Some(screen_pos) = window.cursor_position() {
            let (camera, camera_transform) = camera_query.single();
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
                // 1. Get current tile ID and check resources first
                let current_tile_id = world.tiles.get(&pos).map(|(_, id)| *id).unwrap_or(0);

                // 2. Check if we have enough resources
                if *world.resources.get(&tile_type).unwrap_or(&0) >= 1
                    || placer.tile_type == current_tile_id
                {
                    // 3. Update resources first (avoid nested borrows)
                    *world.resources.entry(current_tile_id).or_insert(0) += 1;
                    *world.resources.entry(tile_type).or_insert(0) -= 1;

                    // 4. Now create the new tile (in a separate scope to avoid multiple borrows)
                    let new_tile = match tile_type {
                        1 => (
                            Box::new(Conveyor {
                                position: pos,
                                direction,
                                item: Item::None,
                            }) as Box<dyn Tile>,
                            1,
                        ),
                        2 => {
                            let mut hashmap = HashMap::new();
                            hashmap.insert(Item::Wood, 5);
                            hashmap.insert(Item::Stone, 5);
                            (
                                Box::new(Factory {
                                    factory_type: FactoryType::Assembler,
                                    position: pos,
                                    direction,
                                    inventory: HashMap::new(),
                                    capacity: hashmap,
                                }) as Box<dyn Tile>,
                                2,
                            )
                        }
                        3 => (
                            Box::new(Extractor {
                                position: pos,
                                direction,
                                interval: 5,
                                spawn_item: Item::Stone,
                                required_terrain: TerrainTileType::Dirt,
                            }) as Box<dyn Tile>,
                            3,
                        ),
                        _ => (
                            Box::new(Conveyor {
                                position: pos,
                                direction,
                                item: Item::None,
                            }) as Box<dyn Tile>,
                            1,
                        ),
                    };

                    // 5. Update the tile in the world
                    if let Some(entry) = world.tiles.get_mut(&pos) {
                        *entry = new_tile;
                    }
                }
            } else {
                // Creating a new tile
                if *world.resources.get(&tile_type).unwrap_or(&0) >= 1 {
                    // Update resources first
                    *world.resources.entry(tile_type).or_insert(0) -= 1;

                    // Create the new tile
                    let new_tile = match tile_type {
                        1 => (
                            Box::new(Conveyor {
                                position: pos,
                                direction,
                                item: Item::None,
                            }) as Box<dyn Tile>,
                            1,
                        ),
                        2 => {
                            let mut hashmap = HashMap::new();
                            hashmap.insert(Item::Wood, 5);
                            hashmap.insert(Item::Stone, 5);
                            (
                                Box::new(Factory {
                                    factory_type: FactoryType::Assembler,
                                    position: pos,
                                    direction,
                                    inventory: HashMap::new(),
                                    capacity: hashmap,
                                }) as Box<dyn Tile>,
                                2,
                            )
                        }
                        3 => (
                            Box::new(Extractor {
                                position: pos,
                                direction,
                                interval: 5,
                                spawn_item: Item::Stone,
                                required_terrain: TerrainTileType::Dirt,
                            }) as Box<dyn Tile>,
                            3,
                        ),
                        _ => (
                            Box::new(Conveyor {
                                position: pos,
                                direction,
                                item: Item::None,
                            }) as Box<dyn Tile>,
                            1,
                        ),
                    };

                    // Insert the tile
                    world.tiles.insert(pos, new_tile);

                    // Create the visual representation
                    commands
                        .spawn((
                            Sprite::from_image(asset_server.load("textures/tiles/belt.png")),
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
                                Sprite::from_image(asset_server.load("textures/items/none.png")),
                                Transform::from_scale(Vec3::splat(0.5)),
                            ));
                        });
                }
            }
        }
    }
    if mouse_button_input.just_pressed(MouseButton::Right) {
        let window = windows.single();
        if let Some(screen_pos) = window.cursor_position() {
            let (camera, camera_transform) = camera_query.single();
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
            } else {
                placer.tile_type = 0;
            }
        }
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
    camera.single_mut().translation += direction.normalize_or_zero().extend(0.0) * CAMERA_SPEED;
}
