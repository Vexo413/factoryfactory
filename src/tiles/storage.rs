use std::any::Any;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::{Action, Direction, Item, Position, WorldRes};

use super::Tile;

#[derive(Debug)]
pub struct Storage {
    pub position: Position,
    pub direction: Direction,
    pub inventory: u32,
    pub storage_type: StorageType,
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

        if world.tiles.contains_key(&end_position) {
            if self.inventory >= 1 {
                return Some(Action::Move(
                    self.position,
                    end_position,
                    self.storage_type.stored_item(),
                ));
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
pub enum StorageType {
    SmallRigotriumVault,
    SmallFlextoriumVault,
    SmallBattery,
}

impl StorageType {
    pub fn capacity(&self) -> u32 {
        match self {
            StorageType::SmallRigotriumVault => 10,
            StorageType::SmallFlextoriumVault => 10,
            StorageType::SmallBattery => 10,
        }
    }
    fn stored_item(&self) -> Item {
        match self {
            StorageType::SmallRigotriumVault => Item::Rigtorium,
            StorageType::SmallFlextoriumVault => Item::Flextorium,
            StorageType::SmallBattery => Item::Electrine,
        }
    }
}
