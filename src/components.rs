use crate::Position;
use bevy::prelude::*;

#[derive(Component)]
pub struct TerrainChunk {
    pub position: crate::resources::ChunkPosition,
}

#[derive(Component)]
pub struct TileSprite {
    pub pos: Position,
}

#[derive(Component)]
pub struct ItemAnimation {
    pub start_pos: Vec3,
    pub end_pos: Vec3,
    pub timer: Timer,
}

#[derive(Component)]
pub struct Inventory {
    pub selected_category: u8,
}

#[derive(Component)]
pub struct InventoryCategory {
    pub category: u8,
}

#[derive(Component)]
pub struct InventoryItemsPanel;

#[derive(Component)]
pub struct InventoryItem {
    pub tile_type: (u8, u8),
}

#[derive(Component)]
pub struct InventoryContextMenu;

#[derive(Component)]
pub struct HotkeyOption {
    pub tile_type: (u8, u8),
}

#[derive(Component)]
pub struct SellOption {
    pub tile_type: (u8, u8),
}

#[derive(Component)]
pub struct HotkeyButton {
    pub key: u8,
    pub tile_type: (u8, u8),
}

#[derive(Component)]
pub struct CoreMenu {
    pub position: Position,
    pub selected_category: u8,
}

#[derive(Component)]
pub struct CoreCategory {
    pub category: u8,
}

#[derive(Component)]
pub struct CoreItemsPanel;

#[derive(Component)]
pub struct CoreMenuItem {
    pub tile_type: (u8, u8),
}

#[derive(Component)]
pub struct CoreContextMenu;

#[derive(Component)]
pub struct BuyOption {
    pub tile_type: (u8, u8),
}

#[derive(Component)]
pub struct MoneyWidget;
