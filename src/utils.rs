use crate::{
    Action, Direction, ExtractorType, FactoryType, Item, Position, RouterOutputIndex, StorageType,
    WorldRes, tiles::*,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

/// Get the texture path for a specific tile type
pub fn get_tile_texture(tile_type: (u8, u8)) -> &'static str {
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

/// Format a tile ID in a human-readable format
pub fn format_tile_id(tile_type: (u8, u8)) -> String {
    format!("{}, {}", tile_type.0, tile_type.1)
}

/// Get a human-readable name for a tile type
pub fn get_tile_name(tile_type: (u8, u8)) -> String {
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

/// Get the production interval for a tile type when used in the core
pub fn get_tile_core_interval(tile_type: (u8, u8)) -> u32 {
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

/// Create a new tile instance based on type, position, and direction
pub fn get_new_tile(
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

/// Rotate a direction 90 degrees clockwise
pub fn rotate_direction_clockwise(dir: Direction) -> Direction {
    match dir {
        Direction::Up => Direction::Right,
        Direction::Right => Direction::Down,
        Direction::Down => Direction::Left,
        Direction::Left => Direction::Up,
    }
}

/// Rotate a direction 90 degrees counterclockwise
pub fn rotate_direction_counterclockwise(dir: Direction) -> Direction {
    match dir {
        Direction::Up => Direction::Left,
        Direction::Left => Direction::Down,
        Direction::Down => Direction::Right,
        Direction::Right => Direction::Up,
    }
}

/// Check if a tile at a given position is a conveyor pointing to a specific direction
pub fn is_conveyor_pointing_to(
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

/// Determine the appropriate conveyor texture based on its surroundings
pub fn determine_conveyor_texture(world: &WorldRes, conveyor: &Conveyor) -> &'static str {
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

/// Check if a tile can accept a specific item
pub fn can_tile_accept_item(tile: &(Box<dyn Tile>, (u8, u8)), item: Item) -> bool {
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

/// Get the destination position for a production action
pub fn get_produce_destination(pos: Position, world: &WorldRes) -> Option<(Position, Position)> {
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

/// Sort a list of actions in topological order to ensure proper execution
pub fn sort_moves_topologically(actions: Vec<Action>, world: &WorldRes) -> Vec<Action> {
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
