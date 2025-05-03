use crate::{CAMERA_SPEED, CoreContextMenu, Inventory, Placer};
use bevy::prelude::*;

pub fn move_camera(
    mut camera: Query<&mut Transform, With<Camera2d>>,
    placer: Res<Placer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    inventory_query: Query<(), With<Inventory>>,
    core_menu_query: Query<(), With<CoreContextMenu>>,
) {
    if inventory_query.is_empty() && core_menu_query.is_empty() {
        let mut direction = Vec2::ZERO;
        if keyboard_input.pressed(KeyCode::KeyW) {
            direction.y = 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction.y = -1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction.x = -1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction.x = 1.0;
        }
        if let Ok(mut camera) = camera.single_mut() {
            camera.translation +=
                direction.normalize_or_zero().extend(0.0) * CAMERA_SPEED / placer.zoom_level;
        }
    }
}
