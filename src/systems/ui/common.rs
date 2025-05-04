use crate::{WorldRes, components::*};
use bevy::prelude::*;

pub fn exit_menu(
    mut commands: Commands,
    inventory_query: Query<Entity, With<Inventory>>,
    inventory_context_query: Query<Entity, With<InventoryContextMenu>>,
    core_menu_query: Query<Entity, With<CoreMenu>>,
    core_context_query: Query<Entity, With<CoreContextMenu>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        for entity in inventory_context_query.iter() {
            commands.entity(entity).despawn();
        }
        for entity in core_context_query.iter() {
            commands.entity(entity).despawn();
        }
        for entity in core_menu_query.iter() {
            commands.entity(entity).despawn();
        }
        for entity in inventory_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn update_money_widget(
    mut money_widget_query: Query<&mut Text, With<MoneyWidget>>,
    world: Res<WorldRes>,
) {
    if let Ok(mut text) = money_widget_query.single_mut() {
        text.0 = format!("${}", world.money);
    }
}
