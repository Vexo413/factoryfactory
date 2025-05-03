use std::{any::Any, collections::HashMap};

use crate::{Action, Direction, FactoryType, Item, Position, WorldRes};

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
