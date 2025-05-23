use std::any::Any;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::{
    Action, Direction, Item, Position, WorldRes, rotate_direction_clockwise,
    rotate_direction_counterclockwise,
};

use super::{Conveyor, Factory, Tile};

#[derive(Debug)]
pub struct Router {
    pub position: Position,
    pub direction: Direction,
    pub item: Option<Item>,
    pub last_output: RouterOutputIndex,
}

impl Tile for Router {
    fn tick(&self, world: &WorldRes) -> Option<Action> {
        if let Some(item) = self.item {
            let mut next_output = self.last_output.next();
            let start_position = self.position;

            for _ in 0..3 {
                let dir = next_output.to_direction(self.direction);
                let mut end_pos = self.position;

                match dir {
                    Direction::Up => end_pos.y += 1,
                    Direction::Down => end_pos.y -= 1,
                    Direction::Left => end_pos.x -= 1,
                    Direction::Right => end_pos.x += 1,
                }

                if let Some(tile) = world.tiles.get(&end_pos) {
                    let can_accept =
                        if let Some(conveyor) = tile.0.as_any().downcast_ref::<Conveyor>() {
                            conveyor.item.is_none()
                        } else if let Some(router) = tile.0.as_any().downcast_ref::<Router>() {
                            router.item.is_none()
                        } else if let Some(factory) = tile.0.as_any().downcast_ref::<Factory>() {
                            factory.factory_type.capacity().get(&item).unwrap_or(&0)
                                > factory.inventory.get(&item).unwrap_or(&0)
                        } else {
                            false
                        };

                    if can_accept {
                        return Some(Action::MoveRouter(
                            start_position,
                            end_pos,
                            item,
                            next_output,
                        ));
                    }
                }
                next_output = next_output.next();
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
