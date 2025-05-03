use std::any::Any;

use crate::{Action, Direction, Item, Position, WorldRes, can_tile_accept_item};

use super::Tile;

#[derive(Debug)]
pub struct Junction {
    pub position: Position,

    pub horizontal_item: Option<(Item, Direction)>,

    pub vertical_item: Option<(Item, Direction)>,
}
impl Tile for Junction {
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        if let Some((item, input_dir)) = self.horizontal_item {
            let output = match input_dir {
                Direction::Left => Direction::Right,
                Direction::Right => Direction::Left,
                _ => return None,
            };
            let end_pos = self.position.shift(output);
            if world.tiles.get(&end_pos).is_some()
                && can_tile_accept_item(world.tiles.get(&end_pos).unwrap(), item)
            {
                return Some(Action::Move(self.position, end_pos, item));
            }
        }

        if let Some((item, input_dir)) = self.vertical_item {
            let output = match input_dir {
                Direction::Down => Direction::Up,
                Direction::Up => Direction::Down,
                _ => return None,
            };
            let end_pos = self.position.shift(output);
            if world.tiles.get(&end_pos).is_some()
                && can_tile_accept_item(world.tiles.get(&end_pos).unwrap(), item)
            {
                return Some(Action::Move(self.position, end_pos, item));
            }
        }
        None
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
