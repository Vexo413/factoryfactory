use bevy::prelude::*;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{rotate_direction_clockwise, rotate_direction_counterclockwise};

/// Position in the grid world
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Serialize, Deserialize, Encode, Decode,
)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn shift(&self, direction: Direction) -> Position {
        let mut pos = *self;
        match direction {
            Direction::Up => pos.y += 1,
            Direction::Down => pos.y -= 1,
            Direction::Left => pos.x -= 1,
            Direction::Right => pos.x += 1,
        }
        pos
    }

    pub fn get_as_key(&self) -> u64 {
        ((self.x as u64) & 0xFFFFFFFF) | (((self.y as u64) & 0xFFFFFFFF) << 32)
    }

    pub fn from_key(key: u64) -> Self {
        let x = (key & 0xFFFFFFFF) as i32;
        let y = ((key >> 32) & 0xFFFFFFFF) as i32;
        Position::new(x, y)
    }
}

/// Direction in the grid world
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn shift(&self, i: i32) -> Direction {
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

/// Types of items that can be transported and processed
#[derive(PartialEq, Eq, Clone, Hash, Debug, Copy, Deserialize, Serialize, Encode, Decode)]
pub enum Item {
    RawFlextorium,
    RawRigtorium,
    Flextorium,
    Rigtorium,
    Electrine,
    RigtoriumRod,
    Conveyor,
}

impl Item {
    pub fn sprite(&self) -> &'static str {
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

    pub fn to_tile(&self) -> Option<(u8, u8)> {
        match self {
            Item::Conveyor => Some((1, 1)),
            _ => None,
        }
    }
}

/// Recipe for crafting items
#[derive(Debug, Clone)]
pub struct Recipe {
    pub inputs: HashMap<Item, u32>,
    pub output: Item,
}

/// Actions that can be performed by tiles
#[derive(Debug, Clone)]
pub enum Action {
    Move(Position, Position, Item),
    MoveRouter(Position, Position, Item, RouterOutputIndex),
    Produce(Position),
    Teleport(Position, (u8, u8)),
    IncreaseTicks(Position),
}

/// Types of terrain tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum TerrainTileType {
    RawFlextoriumDeposit,
    RawRigtoriumDeposit,
    ElectrineDeposit,
    Stone,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
pub enum ExtractorType {
    RawFlextorium,
    RawRigtorium,
    Electrine,
}

impl ExtractorType {
    pub fn interval(&self) -> i32 {
        match self {
            ExtractorType::RawRigtorium => 5,
            ExtractorType::RawFlextorium => 5,
            ExtractorType::Electrine => 2,
        }
    }

    pub fn terrain(&self) -> crate::types::TerrainTileType {
        match self {
            ExtractorType::RawRigtorium => crate::types::TerrainTileType::RawRigtoriumDeposit,
            ExtractorType::RawFlextorium => crate::types::TerrainTileType::RawFlextoriumDeposit,
            ExtractorType::Electrine => crate::types::TerrainTileType::ElectrineDeposit,
        }
    }

    pub fn spawn_item(&self) -> Item {
        match self {
            ExtractorType::RawRigtorium => Item::RawRigtorium,
            ExtractorType::RawFlextorium => Item::RawFlextorium,
            ExtractorType::Electrine => Item::Electrine,
        }
    }

    pub fn sprite(&self) -> String {
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

/// Types of factories for processing different resources
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
pub enum FactoryType {
    RigtoriumSmelter,
    FlextoriumFabricator,
    RigtoriumRodMolder,
    ConveyorConstructor,
}

impl FactoryType {
    pub fn capacity(&self) -> HashMap<Item, u32> {
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

    pub fn recipe(&self) -> crate::types::Recipe {
        match self {
            FactoryType::RigtoriumSmelter => {
                let mut inputs = HashMap::new();
                inputs.insert(Item::RawRigtorium, 1);
                inputs.insert(Item::Electrine, 1);
                crate::types::Recipe {
                    inputs,
                    output: Item::Rigtorium,
                }
            }
            FactoryType::FlextoriumFabricator => {
                let mut inputs = HashMap::new();
                inputs.insert(Item::RawFlextorium, 1);
                inputs.insert(Item::Electrine, 1);
                crate::types::Recipe {
                    inputs,
                    output: Item::Flextorium,
                }
            }
            FactoryType::RigtoriumRodMolder => {
                let mut inputs = HashMap::new();
                inputs.insert(Item::Rigtorium, 2);
                inputs.insert(Item::Electrine, 1);
                crate::types::Recipe {
                    inputs,
                    output: Item::RigtoriumRod,
                }
            }
            FactoryType::ConveyorConstructor => {
                let mut inputs = HashMap::new();
                inputs.insert(Item::Flextorium, 4);
                inputs.insert(Item::RigtoriumRod, 2);
                inputs.insert(Item::Electrine, 1);
                crate::types::Recipe {
                    inputs,
                    output: Item::Conveyor,
                }
            }
        }
    }

    pub fn sprite(&self) -> &'static str {
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
pub enum RouterOutputIndex {
    Forward = 0,
    Right = 1,
    Left = 2,
}
impl RouterOutputIndex {
    pub fn next(&self) -> Self {
        match self {
            RouterOutputIndex::Forward => RouterOutputIndex::Right,
            RouterOutputIndex::Right => RouterOutputIndex::Left,
            RouterOutputIndex::Left => RouterOutputIndex::Forward,
        }
    }

    pub fn to_direction(&self, base_direction: Direction) -> Direction {
        match self {
            RouterOutputIndex::Forward => base_direction,
            RouterOutputIndex::Right => rotate_direction_clockwise(base_direction),
            RouterOutputIndex::Left => rotate_direction_counterclockwise(base_direction),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
pub enum StorageType {
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
