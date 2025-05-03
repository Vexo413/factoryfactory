use std::any::Any;

use crate::{Action, Item, Position, WorldRes};

use super::Tile;

#[derive(Debug)]
pub struct Core {
    pub position: Position,
    pub interval: u32,
    pub ticks: u32,
    pub tile_id: (u8, u8),
}

impl Tile for Core {
    fn tick(&self, _world: &WorldRes) -> Option<Action> {
        dbg!(self.ticks);
        if self.ticks >= self.interval {
            println!("created: {:?}", self.tile_id);
            return Some(Action::Teleport(self.position, self.tile_id));
        } else {
            return Some(Action::IncreaseTicks(self.position));
        }
    }
    fn set_item(&mut self, _item: Option<Item>) {}

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
