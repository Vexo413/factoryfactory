use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::router::RouterOutputIndex;

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

#[derive(PartialEq, Eq, Clone, Hash, Debug, Copy, Deserialize, Serialize, Encode, Decode)]
pub enum Item {
    RawFlextorium,
    RawRigtorium,
    Flextorium,
    Rigtorium,
    Electrine,
    RigtoriumRod,
    Conveyor,
    Router,
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
            Item::Router => "embedded://textures/items/router.png",
        }
    }

    pub fn to_tile(&self) -> Option<(u8, u8)> {
        match self {
            Item::Conveyor => Some((1, 1)),
            Item::Router => Some((1, 2)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Recipe {
    pub inputs: HashMap<Item, u32>,
    pub output: Item,
}

#[derive(Debug, Clone)]
pub enum Action {
    Move(Position, Position, Item),
    MoveRouter(Position, Position, Item, RouterOutputIndex),
    Produce(Position),
    Teleport(Position, (u8, u8)),
    IncreaseTicks(Position),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum TerrainTileType {
    RawFlextoriumDeposit,
    RawRigtoriumDeposit,
    ElectrineDeposit,
    Stone,
}
