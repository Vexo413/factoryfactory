use crate::components::*;
use bevy::prelude::*;

pub fn exit_menu(
    mut commands: Commands,
    inventory_query: Query<Entity, With<Inventory>>,
    core_menu_query: Query<Entity, With<CoreContextMenu>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        for entity in core_menu_query.iter() {
            commands.entity(entity).despawn();
        }
        for entity in inventory_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}
