use std::{any::Any, fmt::Debug};

use crate::{Action, Item, WorldRes};

/// Trait for all placeable tiles in the game
pub trait Tile: Send + Sync + Debug {
    /// Execute tile behavior on tick and return action if needed
    fn tick(&self, tiles: &WorldRes) -> Option<Action>;

    /// Set the item on this tile (if supported)
    fn set_item(&mut self, item: Option<Item>);

    /// Get the current item on this tile
    fn get_item(&self) -> Option<Item>;

    /// Return as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Return as mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
