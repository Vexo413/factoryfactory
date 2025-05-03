use std::any::Any;

use crate::{Action, Direction, ExtractorType, Item, Position, WorldRes};

use super::Tile;

#[derive(Debug)]
pub struct Extractor {
    pub position: Position,
    pub direction: Direction,
    pub extractor_type: ExtractorType,
    pub item: Option<Item>,
}

impl Tile for Extractor {
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        if world.tick_count % self.extractor_type.interval() == 0
            && world.terrain.get(&self.position) == Some(&self.extractor_type.terrain())
        {
            return Some(Action::Produce(self.position));
        }
        let start_position = self.position;
        let mut end_position = self.position;

        match self.direction {
            Direction::Up => end_position.y += 1,
            Direction::Down => end_position.y -= 1,
            Direction::Left => end_position.x -= 1,
            Direction::Right => end_position.x += 1,
        }

        if world.tiles.contains_key(&end_position) {
            if let Some(item) = self.item {
                return Some(Action::Move(start_position, end_position, item));
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
