use bevy::{color::palettes::css, input::mouse::MouseWheel, prelude::*, window::PrimaryWindow};
use noise::{NoiseFn, Perlin};
use std::{
    any::Any,
    collections::{HashMap, HashSet},
    f32::consts::{FRAC_PI_2, PI},
    fmt::Debug,
};

const TILE_SIZE: f32 = 64.0;
const ITEM_SIZE: f32 = 32.0;
const IMAGE_SIZE: f32 = 128.0;
const TICK_LENGTH: f32 = 1.0; // Duration of the animation in seconds

#[derive(Debug, Clone)]
struct Recipe {
    // For simplicity, a recipe is a mapping from input Item to quantity required
    inputs: HashMap<Item, u32>,
    // And one output item (with quantity, if needed)
    output: (Item, u32),
}

#[derive(Resource)]
struct ConveyorPlacer {
    direction: Direction,
}

impl Default for ConveyorPlacer {
    fn default() -> Self {
        Self {
            direction: Direction::Up,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FactoryType {
    Assembler,
}

#[derive(Debug, Clone)]
enum Action {
    Move(Position, Position, Item),
    MoveFactory(Position, Position, Item),
    Produce(Position),
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct Position {
    x: i32,
    y: i32,
}

impl Position {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq, Eq, Clone, Hash, Debug, Copy)]
enum Item {
    None,
    Wood,
    Stone,
    Product,
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

        if let Some(target_tile) = world.tiles.get(&end_position) {
            if self.item != Item::None {
                if target_tile.as_any().is::<Conveyor>() {
                    return Action::Move(start_position, end_position, self.item);
                }
                if target_tile.as_any().is::<Factory>() {
                    return Action::MoveFactory(start_position, end_position, self.item);
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

#[derive(Debug)]
struct Extractor {
    position: Position,
    direction: Direction,
    spawn_item: Item,
    interval: i32, // spawn frequency (in ticks)
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
    // Inventory maps each item to the current quantity
    inventory: HashMap<Item, u32>,
    // Maximum capacity per item type
    capacity: HashMap<Item, u32>,
}

fn recipe_for(factory_type: FactoryType) -> Recipe {
    match factory_type {
        FactoryType::Assembler => {
            let mut inputs = HashMap::new();
            // For simplicity, an assembler needs 1 Wood and 1 Stone
            inputs.insert(Item::Wood, 1);
            inputs.insert(Item::Stone, 1);
            Recipe {
                inputs,
                output: (Item::Product, 1),
            }
        }
    }
}

impl Factory {
    // Check if the factory has enough materials to produce its output
    fn can_produce(&self) -> bool {
        let recipe = recipe_for(self.factory_type);
        recipe
            .inputs
            .iter()
            .all(|(item, &qty_required)| self.inventory.get(item).unwrap_or(&0) >= &qty_required)
    }

    // Produce the output and remove the required materials from the inventory.
    // Returns produced item (or None if not enough materials).
    fn produce(&mut self) -> Option<(Item, u32)> {
        let recipe = recipe_for(self.factory_type);
        if self.can_produce() {
            // Remove inputs
            for (item, &qty_required) in recipe.inputs.iter() {
                if let Some(qty) = self.inventory.get_mut(item) {
                    *qty = qty.saturating_sub(qty_required);
                }
            }
            // Return produced output
            Some(recipe.output)
        } else {
            None
        }
    }

    // Allow adding materials to the factory, if under capacity.
    fn add_material(&mut self, item: Item, amount: u32) -> bool {
        let current = self.inventory.entry(item).or_insert(0);
        let cap = self.capacity.get(&item).copied().unwrap_or(0);
        if *current + amount <= cap {
            *current += amount;
            true
        } else {
            false
        }
    }
}

impl Tile for Factory {
    fn tick(&self, world: &WorldRes) -> Action {
        // In a tick, if the factory can produce, then create a produce action.
        let mut end_position = self.position;

        match self.direction {
            Direction::Up => end_position.y += 1,
            Direction::Down => end_position.y -= 1,
            Direction::Left => end_position.x -= 1,
            Direction::Right => end_position.x += 1,
        }

        if self.can_produce()
            && world
                .tiles
                .get(&end_position)
                .unwrap()
                .as_any()
                .downcast_ref::<Conveyor>()
                .unwrap()
                .item
                == Item::None
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TerrainTileType {
    Grass,
    Dirt,
    // Add more if desired.
}

#[derive(Resource)]
struct WorldRes {
    tiles: HashMap<Position, Box<dyn Tile>>,
    terrain: HashMap<Position, TerrainTileType>,
    tick_timer: Timer,
    tick_count: i32,
    actions: Vec<Action>,
}

// Marker component for our visual representation of a tile.
// We store its grid-position so we can later lookup the corresponding tile in WorldRes.
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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Insert our custom world resource.
        .insert_resource(WorldRes {
            tiles: HashMap::new(),
            terrain: HashMap::new(),
            tick_timer: Timer::from_seconds(TICK_LENGTH, TimerMode::Repeating),
            tick_count: 0,
            actions: Vec::new(),
        })
        .insert_resource(ConveyorPlacer::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                tick,
                update_tile_visual_system.after(tick),
                animate_items_system,
                place_conveyor_system,
            ),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut world: ResMut<WorldRes>) {
    // Spawn a 2D camera.
    commands.spawn(Camera2d::default());
    // Create a Perlin noise instance.
    let perlin = Perlin::new(59);
    // Scale factor for noise sampling.
    let noise_scale = 0.1;

    // Generate terrain over a grid. (Here, -20..=20 is arbitrarily chosen.)
    for x in -20..=20 {
        for y in -20..=20 {
            // Sample the noise function at this grid point.
            let noise_val = perlin.get([x as f64 * noise_scale, y as f64 * noise_scale]);
            // Define the terrain type based on the noise value.
            let terrain_type = if noise_val > 0.0 {
                TerrainTileType::Grass
            } else {
                TerrainTileType::Dirt
            };
            world.terrain.insert(Position::new(x, y), terrain_type);
        }
    }

    // (Optional) You can visualize the terrain by spawning sprites at every (x, y)
    // depending on the terrain type. For example:
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

    // Insert some tiles into our world.
    world.tiles.insert(
        Position::new(-3, -3),
        Box::new(Extractor {
            interval: 5,
            position: Position::new(-3, -3),
            spawn_item: Item::Stone,
            direction: Direction::Right,
            required_terrain: TerrainTileType::Dirt,
        }),
    );
    world.tiles.insert(
        Position::new(3, 3),
        Box::new(Extractor {
            interval: 5,
            position: Position::new(3, 3),
            spawn_item: Item::Wood,
            direction: Direction::Left,
            required_terrain: TerrainTileType::Grass,
        }),
    );

    // Load a texture (assumed to be a white square, e.g., "tile.png" in your assets folder).
    //let texture_handle = asset_server.load("tile.png");

    // For each tile in our world, spawn a sprite entity so we can render it.
    for (pos, _tile) in world.tiles.iter() {
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

fn tick(
    time: Res<Time>,
    mut commands: Commands,
    mut world: ResMut<WorldRes>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&TileSprite, &Children)>,
    mut child_query: Query<&mut Visibility>,
) {
    world.tick_timer.tick(time.delta());
    if world.tick_timer.finished() {
        world.tick_count += 1;

        // Process actions.
        for action in world.actions.clone() {
            match action {
                Action::Move(start, end, item) => {
                    if let Some(end_conveyor) = world
                        .tiles
                        .get_mut(&end)
                        .unwrap()
                        .as_any_mut()
                        .downcast_mut::<Conveyor>()
                    {
                        if end_conveyor.item == Item::None {
                            end_conveyor.item = item;
                            if let Some(start_conveyor) = world
                                .tiles
                                .get_mut(&start)
                                .unwrap()
                                .as_any_mut()
                                .downcast_mut::<Conveyor>()
                            {
                                start_conveyor.item = Item::None;
                            }
                        }
                    }
                }
                Action::MoveFactory(start, end, item) => {
                    if let Some(factory) = world
                        .tiles
                        .get_mut(&end)
                        .unwrap()
                        .as_any_mut()
                        .downcast_mut::<Factory>()
                    {
                        dbg!(&factory);
                        if factory.capacity.get(&item).unwrap_or(&0_u32)
                            > factory.inventory.get(&item).unwrap_or(&0_u32)
                        {
                            *factory.inventory.entry(item).or_insert(0) += 1;
                            if let Some(start_conveyor) = world
                                .tiles
                                .get_mut(&start)
                                .unwrap()
                                .as_any_mut()
                                .downcast_mut::<Conveyor>()
                            {
                                start_conveyor.item = Item::None;
                            }
                        }
                    }
                }
                Action::Produce(position) => {
                    if let Some(factory) = world
                        .tiles
                        .get_mut(&position)
                        .unwrap()
                        .as_any_mut()
                        .downcast_mut::<Factory>()
                    {
                        if let Some((produced_item, _produced_qty)) = factory.produce() {
                            // Determine output position based on factory's direction.
                            let mut end_position = factory.position;
                            match factory.direction {
                                Direction::Up => end_position.y += 1,
                                Direction::Down => end_position.y -= 1,
                                Direction::Left => end_position.x -= 1,
                                Direction::Right => end_position.x += 1,
                            }
                            // Place the produced item into the adjacent conveyor if empty.
                            if let Some(tiles) = world.tiles.get_mut(&end_position) {
                                if let Some(conveyor) =
                                    tiles.as_any_mut().downcast_mut::<Conveyor>()
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
                        .as_any_mut()
                        .downcast_mut::<Extractor>()
                    {
                        // Determine output position based on factory's direction.
                        let mut end_position = extractor.position;
                        let item = extractor.spawn_item;
                        match extractor.direction {
                            Direction::Up => end_position.y += 1,
                            Direction::Down => end_position.y -= 1,
                            Direction::Left => end_position.x -= 1,
                            Direction::Right => end_position.x += 1,
                        }
                        // Place the produced item into the adjacent conveyor if empty.
                        if let Some(tiles) = world.tiles.get_mut(&end_position) {
                            if let Some(conveyor) = tiles.as_any_mut().downcast_mut::<Conveyor>() {
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
        // Gather actions for each tile.
        for tile in world.tiles.values() {
            let action = tile.tick(&world);
            // (This debug helps show the actions in the console.)
            // dbg!(&action);
            next.push(action);
        }

        // Sort moves topologically.
        world.actions = sort_moves_topologically(next);
        world.actions.reverse();

        for action in &world.actions {
            match action {
                Action::Move(start, end, item) => {
                    if let Some(end_conveyor) = world
                        .tiles
                        .get(&end)
                        .unwrap()
                        .as_any()
                        .downcast_ref::<Conveyor>()
                    {
                        if end_conveyor.item == Item::None {
                            for (tile_sprite, children) in query.iter() {
                                if tile_sprite.pos == *start {
                                    for &child in children.iter() {
                                        if let Ok(visibility) = child_query.get_mut(child) {
                                            *visibility.into_inner() = Visibility::Hidden;
                                        }
                                    }
                                }
                            }
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
                Action::MoveFactory(start, end, item) => {
                    if let Some(factory) = world
                        .tiles
                        .get(&end)
                        .unwrap()
                        .as_any()
                        .downcast_ref::<Factory>()
                    {
                        if factory.capacity.get(&item).unwrap_or(&0_u32)
                            > factory.inventory.get(&item).unwrap_or(&0_u32)
                        {
                            for (tile_sprite, children) in query.iter() {
                                if tile_sprite.pos == *start {
                                    for &child in children.iter() {
                                        if let Ok(visibility) = child_query.get_mut(child) {
                                            *visibility.into_inner() = Visibility::Hidden;
                                        }
                                    }
                                }
                            }
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
                _ => {}
            }
        }
    }
    // (We do not call print_center_of_world anymore.)
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

    // Topological sort (Kahn's algorithm)
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

    // Append any unvisited actions.
    for i in 0..actions.len() {
        if !visited.contains(&i) {
            sorted.push(actions[i].clone());
        }
    }

    sorted
}

// This system updates the color and position of each tile sprite based on the current state of that tile.
// It is run after tick() so that changes to a tile’s state (such as a conveyor receiving an item)
// will be reflected in the visuals.
fn update_tile_visual_system(
    world: Res<WorldRes>,
    // Query for parent entities that carry our TileSprite component.
    mut parent_query: Query<(Entity, &TileSprite, &mut Transform, &mut Sprite)>,
    // We query for a parent's children.
    children_query: Query<&Children, With<TileSprite>>,
    // Query for updating the children’s Sprite and Transform.
    mut child_sprite_query: Query<(&mut Sprite, &mut Transform), Without<TileSprite>>,

    asset_server: Res<AssetServer>,
) {
    for (entity, tile_sprite, mut transform, mut sprite) in parent_query.iter_mut() {
        // Update the parent's transform position.
        transform.translation = Vec3::new(
            tile_sprite.pos.x as f32 * TILE_SIZE,
            tile_sprite.pos.y as f32 * TILE_SIZE,
            0.0,
        );

        // Look up the associated tile in our world data.
        if let Some(tile) = world.tiles.get(&tile_sprite.pos) {
            if let Some(conveyor) = tile.as_any().downcast_ref::<Conveyor>() {
                sprite.image = asset_server.load("textures/tiles/belt.png");
                transform.rotation = match conveyor.direction {
                    Direction::Up => Quat::IDENTITY,
                    Direction::Down => Quat::from_rotation_z(PI),
                    Direction::Left => Quat::from_rotation_z(FRAC_PI_2),
                    Direction::Right => Quat::from_rotation_z(-FRAC_PI_2),
                };
                // Now update the (child) item sprite.
                // We assume that each TileSprite entity has one child.
                if let Ok(children) = children_query.get(entity) {
                    for &child in children.iter() {
                        if let Ok((mut child_sprite, mut child_transform)) =
                            child_sprite_query.get_mut(child)
                        {
                            // Reset the child's local translation so it's centered.
                            child_transform.translation = Vec3::new(0.0, 0.0, 1.0);
                            child_transform.rotation = match conveyor.direction {
                                Direction::Up => Quat::IDENTITY,
                                Direction::Down => Quat::from_rotation_z(PI),
                                Direction::Left => Quat::from_rotation_z(-FRAC_PI_2),
                                Direction::Right => Quat::from_rotation_z(FRAC_PI_2),
                            };

                            // Set the child's scale so its effective size is 16x16:
                            // Because parent's scale is TILE_SIZE (=32), we set child scale to 16/TILE_SIZE.

                            // Color the child sprite based on the item type.
                            child_sprite.image = match conveyor.item {
                                Item::None => asset_server.load("textures/items/none.png"), // fully transparent
                                Item::Wood => asset_server.load("textures/items/wood.png"),
                                Item::Stone => asset_server.load("textures/items/stone.png"),
                                Item::Product => asset_server.load("textures/items/product.png"),
                            };
                        }
                    }
                }
            } else if let Some(factory) = tile.as_any().downcast_ref::<Factory>() {
                sprite.image = asset_server.load(
                    match tile
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
                // For factories, leave the parent's sprite tinted purple.
                // Hide any child item sprite.
                if let Ok(children) = children_query.get(entity) {
                    for &child in children.iter() {
                        if let Ok((mut child_sprite, _)) = child_sprite_query.get_mut(child) {
                            child_sprite.color = Color::NONE;
                        }
                    }
                }
            } else if let Some(extractor) = tile.as_any().downcast_ref::<Extractor>() {
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
            sprite.color = css::WHITE.into();
        }
    }
}

fn animate_items_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut ItemAnimation, &mut Transform)>,
    mut tile_query: Query<(&TileSprite, &Children)>,
    mut child_query: Query<&mut Visibility>,
) {
    for (entity, mut animation, mut transform) in query.iter_mut() {
        animation.timer.tick(time.delta());
        let t = animation.timer.fraction();
        transform.translation = animation.start_pos.lerp(animation.end_pos, t);

        if animation.timer.finished() {
            // Show the item sprite at the start position
            for (tile_sprite, children) in tile_query.iter_mut() {
                if tile_sprite.pos
                    == Position::new(
                        (animation.start_pos.x / TILE_SIZE).round() as i32,
                        (animation.start_pos.y / TILE_SIZE).round() as i32,
                    )
                {
                    for &child in children.iter() {
                        if let Ok(mut visibility) = child_query.get_mut(child) {
                            *visibility = Visibility::Visible;
                        }
                    }
                }
            }
            commands.entity(entity).despawn();
        }
    }
}

fn place_conveyor_system(
    windows: Query<&mut Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut placer: ResMut<ConveyorPlacer>,
    mut world: ResMut<WorldRes>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    // Process scroll events to change the placement direction.
    for event in mouse_wheel_events.read() {
        // Here we assume scroll upward rotates clockwise, downward rotates counter-clockwise.
        placer.direction = match (placer.direction, event.y.partial_cmp(&0.0)) {
            // Scroll up (positive y)
            (Direction::Up, Some(std::cmp::Ordering::Greater)) => Direction::Right,
            (Direction::Right, Some(std::cmp::Ordering::Greater)) => Direction::Down,
            (Direction::Down, Some(std::cmp::Ordering::Greater)) => Direction::Left,
            (Direction::Left, Some(std::cmp::Ordering::Greater)) => Direction::Up,
            // Scroll down (negative y)
            (Direction::Up, Some(std::cmp::Ordering::Less)) => Direction::Left,
            (Direction::Left, Some(std::cmp::Ordering::Less)) => Direction::Down,
            (Direction::Down, Some(std::cmp::Ordering::Less)) => Direction::Right,
            (Direction::Right, Some(std::cmp::Ordering::Less)) => Direction::Up,
            (current, _) => current,
        };
        info!(
            "Updated conveyor placement direction: {:?}",
            placer.direction
        );
    }

    // On left-click, attempt to place a new conveyor tile.
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // Get the primary window and the first camera (assuming one camera).
        let window = windows.single();
        if let Some(screen_pos) = window.cursor_position() {
            // Convert the screen coordinates to world coordinates.
            // (Assumes a 2d camera with no projection skew.)
            let (camera, camera_transform) = camera_query.single();
            let window_size = Vec2::new(window.width(), window.height());
            // Convert screen position (with origin at bottom-left)
            let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
            let ndc_to_world =
                camera_transform.compute_matrix() * camera.clip_from_view().inverse();
            let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
            let world_pos: Vec2 = world_pos.truncate();

            // Compute the grid position. (Adjust rounding if your grid origin is different.)
            let grid_x = (world_pos.x / TILE_SIZE).round() as i32;
            let grid_y = -(world_pos.y / TILE_SIZE).round() as i32;
            let pos = Position::new(grid_x, grid_y);

            // Only add a conveyor if the tile is empty.
            if world.tiles.contains_key(&pos) {
                if let Some(obj) = world.tiles.get_mut(&pos) {
                    *obj = Box::new(Conveyor {
                        position: pos,
                        direction: placer.direction,
                        item: Item::None,
                    });
                }
                info!("Tile at {:?} is already occupied", pos);
            } else {
                info!(
                    "Placing conveyor at {:?} facing {:?}",
                    pos, placer.direction
                );
                // Insert the new conveyor tile into our world.
                world.tiles.insert(
                    pos,
                    Box::new(Conveyor {
                        position: pos,
                        direction: placer.direction,
                        item: Item::None,
                    }),
                );

                // Spawn its sprite so it will automatically be drawn.
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
    if mouse_button_input.just_pressed(MouseButton::Right) {
        // Get the primary window and the first camera (assuming one camera).
        let window = windows.single();
        if let Some(screen_pos) = window.cursor_position() {
            // Convert the screen coordinates to world coordinates.
            // (Assumes a 2d camera with no projection skew.)
            let (camera, camera_transform) = camera_query.single();
            let window_size = Vec2::new(window.width(), window.height());
            // Convert screen position (with origin at bottom-left)
            let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
            let ndc_to_world =
                camera_transform.compute_matrix() * camera.clip_from_view().inverse();
            let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
            let world_pos: Vec2 = world_pos.truncate();

            // Compute the grid position. (Adjust rounding if your grid origin is different.)
            let grid_x = (world_pos.x / TILE_SIZE).round() as i32;
            let grid_y = -(world_pos.y / TILE_SIZE).round() as i32;
            let pos = Position::new(grid_x, grid_y);

            // Only add a conveyor if the tile is empty.
            if world.tiles.contains_key(&pos) {
                if let Some(obj) = world.tiles.get_mut(&pos) {
                    let mut hashmap = HashMap::new();
                    hashmap.insert(Item::Wood, 5);
                    hashmap.insert(Item::Stone, 5);
                    *obj = Box::new(Factory {
                        factory_type: FactoryType::Assembler,
                        position: pos,
                        direction: placer.direction,
                        inventory: HashMap::new(),
                        capacity: hashmap,
                    });
                }
                info!("Tile at {:?} is already occupied", pos);
            } else {
                info!(
                    "Placing conveyor at {:?} facing {:?}",
                    pos, placer.direction
                );
                // Insert the new conveyor tile into our world.
                let mut hashmap = HashMap::new();
                hashmap.insert(Item::Wood, 5);
                hashmap.insert(Item::Stone, 5);
                world.tiles.insert(
                    pos,
                    Box::new(Factory {
                        factory_type: FactoryType::Assembler,
                        position: pos,
                        direction: placer.direction,
                        inventory: HashMap::new(),
                        capacity: hashmap,
                    }),
                );

                // Spawn its sprite so it will automatically be drawn.
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
