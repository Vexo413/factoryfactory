use crate::{components::*, get_tile_price, resources::*};
use bevy::prelude::*;

pub fn handle_core_context_menu(
    mut commands: Commands,
    buy_interaction_query: Query<(&Interaction, &BuyOption), Changed<Interaction>>,
    context_menu_query: Query<Entity, With<CoreContextMenu>>,
    mut world: ResMut<WorldRes>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    for (interaction, sell_option) in buy_interaction_query.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok(entity) = context_menu_query.single() {
                if world.money >= get_tile_price(sell_option.tile_type) {
                    world.money -= get_tile_price(sell_option.tile_type);
                    *world
                        .resources
                        .entry(sell_option.tile_type)
                        .or_insert(0_u32) += 1;
                    commands.entity(entity).despawn();
                }
            }
        }
    }
    if mouse_input.just_pressed(MouseButton::Left) && buy_interaction_query.is_empty() {
        if let Ok(entity) = context_menu_query.single() {
            commands.entity(entity).despawn();
        }
    }
}
