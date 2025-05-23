use crate::{
    Action, Direction, Item, Position, WorldRes, extractor::ExtractorType, factory::FactoryType,
    router::RouterOutputIndex, storage::StorageType, tiles::*,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

pub fn get_tile_texture(tile_type: (u8, u8)) -> &'static str {
    match tile_type {
        (0, 1) => "embedded://textures/tiles/none.png",
        (1, 1) => "embedded://textures/tiles/core.png",
        (1, 2) => "embedded://textures/tiles/portal.png",
        (2, 1) => "embedded://textures/tiles/conveyors/back.png",
        (2, 2) => "embedded://textures/tiles/conveyors/router.png",
        (2, 3) => "embedded://textures/tiles/conveyors/junction.png",
        (3, 1) => "embedded://textures/tiles/extractors/raw_rigtorium.png",
        (3, 2) => "embedded://textures/tiles/extractors/raw_flextorium.png",
        (3, 3) => "embedded://textures/tiles/extractors/electrine.png",
        (4, 1) => "embedded://textures/tiles/factories/rigtorium_smelter.png",
        (4, 2) => "embedded://textures/tiles/factories/flextorium_fabricator.png",
        (4, 3) => "embedded://textures/tiles/factories/rigtorium_rod_molder.png",
        (4, 4) => "embedded://textures/tiles/factories/conveyor_constructor.png",
        (4, 5) => "embedded://textures/tiles/factories/router_constructor.png",
        (5, 1) => "embedded://textures/tiles/small_rigtorium_vault.png",
        (5, 2) => "embedded://textures/tiles/small_flextorium_vault.png",
        (5, 3) => "embedded://textures/tiles/small_battery.png",
        _ => "embedded://textures/tiles/conveyors/back.png",
    }
}

pub fn format_tile_id(tile_type: (u8, u8)) -> String {
    format!("{}, {}", tile_type.0, tile_type.1)
}

pub fn get_tile_name(tile_type: (u8, u8)) -> String {
    match tile_type {
        (1, 1) => "Core",
        (1, 2) => "Portal",
        (2, 1) => "Conveyor",
        (2, 2) => "Router",
        (2, 3) => "Junction",
        (3, 1) => "Raw Rigtorium Extractor",
        (3, 2) => "Raw Flextorium Extractor",
        (3, 3) => "Electrine Extractor",
        (4, 1) => "Rigtorium Smelter",
        (4, 2) => "Flextorium Fabricator",
        (4, 3) => "Rigtorium Rod Molder",
        (4, 4) => "Conveyor Constructor",
        (4, 5) => "Router Constructor",
        (5, 1) => "Small Rigtorium Vault",
        (5, 2) => "Small Flextorium Vault",
        (5, 3) => "Small Battery",
        _ => "Unknown Tile",
    }
    .to_string()
}

pub fn get_tile_core_interval(tile_type: (u8, u8)) -> u32 {
    match tile_type {
        (1, 2) => 100,
        (2, 1) => 20,
        (2, 2) => 30,
        (2, 3) => 30,

        (3, 1) => 40,
        (3, 2) => 40,
        (3, 3) => 40,
        (4, 1) => 60,
        (4, 2) => 60,
        (4, 3) => 70,
        (4, 4) => 80,
        (4, 5) => 80,
        (5, 1) => 50,
        (5, 2) => 50,
        (5, 3) => 50,
        _ => 6942,
    }
}

pub fn get_tile_price(tile_type: (u8, u8)) -> u32 {
    match tile_type {
        (1, 2) => 50,
        (2, 1) => 10,
        (2, 2) => 15,
        (2, 3) => 15,

        (3, 1) => 20,
        (3, 2) => 20,
        (3, 3) => 20,
        (4, 1) => 30,
        (4, 2) => 30,
        (4, 3) => 35,
        (4, 4) => 40,
        (4, 5) => 40,
        (5, 1) => 25,
        (5, 2) => 25,
        (5, 3) => 25,
        _ => 60,
    }
}

pub fn get_new_tile(
    tile_type: (u8, u8),
    position: Position,
    direction: Direction,
) -> (Box<dyn Tile>, (u8, u8)) {
    match tile_type {
        (1, 1) => (
            Box::new(Core {
                position,
                interval: 10,
                ticks: 0,
                tile_id: (1, 1),
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (1, 2) => (
            Box::new(Portal {
                position,
                item: None,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (2, 1) => (
            Box::new(Conveyor {
                position,
                direction,
                item: None,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (2, 2) => (
            Box::new(Router {
                position,
                direction,
                item: None,
                last_output: RouterOutputIndex::Forward,
            }) as Box<dyn Tile>,
            tile_type,
        ),

        (2, 3) => (
            Box::new(Junction {
                position,
                horizontal_item: None,
                vertical_item: None,
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
        (4, 2) => (
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
        (4, 3) => (
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
        (4, 4) => (
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
        (4, 5) => (
            Box::new(Factory {
                factory_type: FactoryType::RouterConstructor,
                position,
                direction,
                inventory: HashMap::new(),
                item: None,
                interval: 5,
                ticks: 0,
            }) as Box<dyn Tile>,
            tile_type,
        ),

        (5, 1) => (
            Box::new(Storage {
                position,
                direction,
                inventory: 0,
                storage_type: StorageType::SmallRigotriumVault,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (5, 2) => (
            Box::new(Storage {
                position,
                direction,
                inventory: 0,
                storage_type: StorageType::SmallFlextoriumVault,
            }) as Box<dyn Tile>,
            tile_type,
        ),
        (5, 3) => (
            Box::new(Storage {
                position,
                direction,
                inventory: 0,
                storage_type: StorageType::SmallBattery,
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

pub fn rotate_direction_clockwise(dir: Direction) -> Direction {
    match dir {
        Direction::Up => Direction::Right,
        Direction::Right => Direction::Down,
        Direction::Down => Direction::Left,
        Direction::Left => Direction::Up,
    }
}

pub fn rotate_direction_counterclockwise(dir: Direction) -> Direction {
    match dir {
        Direction::Up => Direction::Left,
        Direction::Left => Direction::Down,
        Direction::Down => Direction::Right,
        Direction::Right => Direction::Up,
    }
}

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
