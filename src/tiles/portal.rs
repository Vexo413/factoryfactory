use std::any::Any;

use crate::{Action, Item, Position, WorldRes};

use super::Tile;

#[derive(Debug)]
pub struct Portal {
    pub position: Position,
    pub item: Option<Item>,
}
impl Tile for Portal {
    fn tick(&self, _world: &WorldRes) -> Option<Action> {
        if let Some(item) = self.item {
            if let Some(tile) = item.to_tile() {
                return Some(Action::Teleport(self.position, tile));
            }
        }

        None
    }

    fn set_item(&mut self, item: Option<Item>) {
        self.item = item;
    }

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
