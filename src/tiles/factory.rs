use std::{any::Any, collections::HashMap};

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::{Action, Direction, Item, Position, WorldRes};

use super::Tile;

#[derive(Debug)]
pub struct Factory {
    pub position: Position,
    pub direction: Direction,
    pub factory_type: FactoryType,
    pub inventory: HashMap<Item, u32>,
    pub item: Option<Item>,
    pub interval: u32,
    pub ticks: u32,
}

impl Factory {
    pub fn can_produce(&self) -> bool {
        let recipe = self.factory_type.recipe();
        recipe
            .inputs
            .iter()
            .all(|(item, &qty_required)| self.inventory.get(item).unwrap_or(&0) >= &qty_required)
            && self.item.is_none()
    }

    pub fn produce(&mut self) -> Option<Item> {
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
pub enum FactoryType {
    RigtoriumSmelter,
    FlextoriumFabricator,
    RigtoriumRodMolder,
    ConveyorConstructor,
    RouterConstructor,
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
            FactoryType::RouterConstructor => {
                let mut hashmap = HashMap::new();
                hashmap.insert(Item::Flextorium, 4);
                hashmap.insert(Item::Conveyor, 2);
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
            FactoryType::RouterConstructor => {
                let mut inputs = HashMap::new();
                inputs.insert(Item::Flextorium, 2);
                inputs.insert(Item::Conveyor, 1);
                crate::types::Recipe {
                    inputs,
                    output: Item::Router,
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
            FactoryType::RouterConstructor => {
                "embedded://textures/tiles/factories/router_constructor.png"
            }
        }
    }
}
