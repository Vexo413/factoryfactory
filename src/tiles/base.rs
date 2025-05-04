use std::{any::Any, fmt::Debug};

use crate::{Action, Item, WorldRes};

pub trait Tile: Send + Sync + Debug {
    fn tick(&self, tiles: &WorldRes) -> Option<Action>;

    fn set_item(&mut self, item: Option<Item>);

    fn get_item(&self) -> Option<Item>;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}
