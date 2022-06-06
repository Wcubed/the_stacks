use crate::card_types::CardType;
use crate::recipe::OngoingRecipe;
use crate::stack::{
    CardFonts, CardImages, CardStack, IsCardHoverOverlay, StackLookingForMovementTarget,
    StackPhysics, DELTA_Z,
};
use bevy::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Range;

/// How much of the previous card you can see when stacking cards.
pub const CARD_STACK_Y_SPACING: f32 = 50.0;

/// Range of Z positions stacks have when laying on the ground.
/// When a stack is created or dropped, it picks a semi-random number in the range.
/// This should minimize overlap among card foreground sprites.
pub const STACK_ROOT_Z_RANGE: Range<f32> = 1.0..100.0;

const CARD_HOVER_OVERLAY_COLOR: Color = Color::rgba(1., 1., 1., 0.1);
const CARD_BORDER_COLOR: Color = Color::BLACK;

pub const CARD_VALUE_SPACING_FROM_CARD_EDGE: f32 = 10.0;

/// - `move_to_empty_space`: Whether this stack should try moving somewhere relatively empty nearby.
///   Stacking on top of another, compatible, stack is also considered "moving to empty space".
pub fn spawn_stack(
    commands: &mut Commands,
    position: Vec2,
    card_type: &CardType,
    card_amount: usize,
    card_images: &Res<CardImages>,
    card_fonts: &Res<CardFonts>,
    title_transform: Transform,
    card_value_transform: Transform,
) {
    if card_amount == 0 {
        return;
    }

    let entities: Vec<Entity> = (0..card_amount)
        .map(|_| {
            spawn_card(
                commands,
                &card_type,
                card_images,
                card_fonts,
                title_transform,
                card_value_transform,
            )
        })
        .collect();

    spawn_stack_root(commands, position, &entities, true);
    set_stack_card_transforms(commands, &entities);
}

/// Spawns a loose card. The new card should be added to a stack straight away.
fn spawn_card(
    commands: &mut Commands,
    card: &CardType,
    card_images: &Res<CardImages>,
    card_fonts: &Res<CardFonts>,
    title_transform: Transform,
    card_value_transform: Transform,
) -> Entity {
    let foreground_color = card.category.text_color();

    let (card_component, description_component) = card.get_card_components();

    let entity = commands
        .spawn_bundle(SpriteBundle {
            texture: card_images.background.clone(),
            sprite: Sprite {
                color: card.category.background_color(),
                ..default()
            },
            ..default()
        })
        .insert(card_component)
        .insert(description_component)
        .with_children(|parent| {
            // Border
            parent.spawn_bundle(SpriteBundle {
                texture: card_images.border.clone(),
                transform: Transform::from_xyz(0.0, 0.0, DELTA_Z),
                sprite: Sprite {
                    color: CARD_BORDER_COLOR,
                    ..default()
                },
                ..default()
            });
            // Title text
            parent.spawn_bundle(Text2dBundle {
                text: Text::with_section(
                    card.title,
                    TextStyle {
                        font: card_fonts.title.clone(),
                        font_size: CARD_STACK_Y_SPACING,
                        color: foreground_color,
                    },
                    TextAlignment {
                        vertical: VerticalAlign::Center,
                        horizontal: HorizontalAlign::Center,
                    },
                ),
                transform: title_transform,
                ..default()
            });
            // Hover overlay
            parent
                .spawn_bundle(SpriteBundle {
                    texture: card_images.hover_overlay.clone(),
                    sprite: Sprite {
                        color: CARD_HOVER_OVERLAY_COLOR,
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, DELTA_Z * 1.5),
                    visibility: Visibility { is_visible: false },
                    ..default()
                })
                .insert(IsCardHoverOverlay);

            // Card coin value
            if let Some(value) = card.value {
                parent.spawn_bundle(Text2dBundle {
                    text: Text::with_section(
                        format!("{} C", value),
                        TextStyle {
                            font: card_fonts.title.clone(),
                            font_size: CARD_STACK_Y_SPACING,
                            color: foreground_color,
                        },
                        TextAlignment {
                            vertical: VerticalAlign::Bottom,
                            horizontal: HorizontalAlign::Left,
                        },
                    ),
                    transform: card_value_transform,
                    ..default()
                });
            }
        })
        .id();

    // Call the custom on_spawn function, if there is one.
    if let Some(func) = card.on_spawn {
        func(commands, entity)
    }

    entity
}

fn spawn_stack_root(
    commands: &mut Commands,
    position: Vec2,
    cards: &[Entity],
    move_to_empty_space: bool,
) -> Entity {
    let root_id = commands
        .spawn_bundle(TransformBundle::from_transform(Transform::from_xyz(
            position.x,
            position.y,
            get_semi_random_stack_root_z(cards[0]),
        )))
        .insert_children(0, cards)
        .insert(CardStack(Vec::from(cards)))
        .id();

    if move_to_empty_space {
        commands
            .entity(root_id)
            .insert(StackLookingForMovementTarget);
    } else {
        commands.entity(root_id).insert(StackPhysics);
    }

    root_id
}

/// Generates a semi-random z position for a stack, based on either the entity id of the stack
/// itself, or if that is not available, any other entity id.
/// Should have the effect of minimizing the clipping of card foreground sprites.
pub fn get_semi_random_stack_root_z(entity: Entity) -> f32 {
    let mut hasher = DefaultHasher::new();
    entity.hash(&mut hasher);

    STACK_ROOT_Z_RANGE.start
        + (hasher.finish() as f32 * DELTA_Z) % (STACK_ROOT_Z_RANGE.end - STACK_ROOT_Z_RANGE.start)
}

/// When given a stack of cards, this function stacks them all nicely.
/// Applies via commands, so effects are only visible next frame.
pub fn set_stack_card_transforms(commands: &mut Commands, stack: &[Entity]) {
    for (i, &card) in stack.iter().enumerate() {
        commands.entity(card).insert(Transform::from_translation(
            relative_center_of_nth_card_in_stack(i),
        ));
    }
}

pub fn stack_visual_size(single_card_visual_size: Vec2, cards_in_stack: usize) -> Vec2 {
    Vec2::new(
        single_card_visual_size.x,
        single_card_visual_size.y + ((cards_in_stack - 1) as f32 * CARD_STACK_Y_SPACING),
    )
}

/// Removes a card from the world.
/// It does not matter if this card is in the middle of a stack,
/// or the only card in a stack. This function will handle it gracefully.
/// The effects are applied via [Commands].
///
/// Do not call this multiple times on the same stack, if the `commands` have not been applied
/// in-between calls. Otherwise the re-positioning of the cards will panic, because the
/// `Transform` is to be added to a card that was removed.
pub fn delete_cards(
    commands: &mut Commands,
    cards_to_delete: &[Entity],
    stack_root: Entity,
    stack: &[Entity],
) {
    // TODO (Wybe 2022-05-24): There is probably a more efficient way than re-initializing
    //      the whole stack's Vec every time a card is deleted. but this works for now.
    //      (don't do pre-mature optimizations and all that).

    if stack[0] == cards_to_delete[0] && stack.len() == 1 && cards_to_delete.len() == 1 {
        // Last card in the stack. Delete the stack as well.
        commands.entity(stack_root).despawn_recursive();
    } else {
        let new_stack = CardStack(
            stack
                .iter()
                .copied()
                .filter(|e| !cards_to_delete.contains(e))
                .collect(),
        );
        set_stack_card_transforms(commands, &new_stack.0);
        commands.entity(stack_root).insert(new_stack);
    }

    for &card in cards_to_delete.iter() {
        commands.entity(card).despawn_recursive();
    }
}

/// Returns the global transform which indicates the center of the top card of a stack.
pub fn global_center_of_top_card(
    root_transform: &GlobalTransform,
    amount_of_cards: usize,
) -> GlobalTransform {
    GlobalTransform::from_translation(
        root_transform.translation
            + root_transform.down()
                * root_transform.scale
                * CARD_STACK_Y_SPACING
                * amount_of_cards as f32,
    )
}

/// Does not need to keep rotation or scaling in mind, because that is applied to the stack root.
/// And the card positions are relative, so any scaling is auto-applied to them.
pub fn relative_center_of_nth_card_in_stack(nth_card: usize) -> Vec3 {
    // Leave Z spacing for card overlays and such.
    // TODO (Wybe 2022-05-24): Is there a better way than just arbitrarily keeping a certain space?
    Vec3::new(
        0.,
        -CARD_STACK_Y_SPACING * nth_card as f32,
        DELTA_Z * nth_card as f32 * 2.,
    )
}

/// Adds the cards of the `source_stack` to the top of the `target_stack`.
/// Assumes no duplicate cards.
/// If stacks have ongoing recipes, it will prefer the target stack's recipe when deciding
/// which one to keep.
/// Recipes that are no longer valid after the merge will be handled by [recipe_check_system](crate::recipe::recipe_check_system).
///
/// Effects are applied via `Commands`, which means it is visible next update.
pub fn merge_stacks(
    commands: &mut Commands,
    source_root: Entity,
    source_stack: &[Entity],
    source_stack_recipe: Option<&OngoingRecipe>,
    target_root: Entity,
    target_stack: &[Entity],
    target_stack_recipe: Option<&OngoingRecipe>,
) {
    if source_stack.is_empty() || target_stack.is_empty() {
        return;
    }

    let mut combined_stack = target_stack.to_owned();
    combined_stack.extend(source_stack);

    set_stack_card_transforms(commands, &combined_stack);

    // To cleanly remove the source root, the children need to be removed first.
    // Otherwise they would get removed as well on a `despawn_recursive`.
    // The reason the despawn is recursive, is to make sure no effects, overlays, or other
    // things remain where the stack was.
    commands.entity(source_root).remove_children(source_stack);
    commands.entity(source_root).despawn_recursive();

    commands
        .entity(target_root)
        .insert(CardStack(combined_stack))
        .insert_children(0, source_stack);

    // Keep recipes. Prefer target stack recipe.
    let kept_recipe = target_stack_recipe.or(source_stack_recipe);
    if let Some(recipe) = kept_recipe {
        commands.entity(target_root).insert(recipe.clone());
    }
}

/// Splits a stack so that the `new_root` card is the root of a new stack.
/// Effects are applied via `Commands`, which means it is visible next update.
/// If a recipe is ongoing, the recipe will be kept on both child stacks.
/// Recipes that are no longer valid after the split will be handled by [recipe_check_system](crate::recipe::recipe_check_system).
///
/// Returns the Entity id of the newly created stack root, if the stack needed to be split.
pub fn split_stack(
    commands: &mut Commands,
    stack_root: Entity,
    stack: &[Entity],
    ongoing_recipe: Option<&OngoingRecipe>,
    new_bottom_card: Entity,
    new_bottom_card_global_transform: &GlobalTransform,
) -> Option<Entity> {
    if stack.is_empty() {
        return None;
    }
    if let Some(new_root_index) = stack.iter().position(|&e| e == new_bottom_card) {
        if new_root_index == 0 {
            // Picking up the root of a stack. No need to split.
            return None;
        }

        let bottom_stack = &stack[0..new_root_index];
        let top_stack = &stack[new_root_index..stack.len()];

        // Update the old (now bottom) stack root.
        commands
            .entity(stack_root)
            .insert(CardStack(Vec::from(bottom_stack)))
            .remove_children(top_stack);

        // Create the new top stack root.
        let new_root_id = spawn_stack_root(commands, Vec2::ZERO, top_stack, false);
        // We explicitly set the transform here, because the default stack spawner will randomize
        // the Z height. Instead, we want it to be right where the hovered card was.
        commands
            .entity(new_root_id)
            .insert(Transform::from(*new_bottom_card_global_transform));

        set_stack_card_transforms(commands, top_stack);

        if let Some(recipe) = ongoing_recipe {
            commands.entity(new_root_id).insert(recipe.clone());
        }

        Some(new_root_id)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::stack::stack_utils::{stack_visual_size, CARD_STACK_Y_SPACING};
    use bevy::prelude::Vec2;

    #[test]
    fn test_stack_visual_size() {
        let single_card_size = Vec2::new(100.0, 250.0);
        let one_card = stack_visual_size(single_card_size, 1);
        assert_eq!(one_card, single_card_size);

        let four_cards = stack_visual_size(single_card_size, 4);
        assert_eq!(
            four_cards,
            Vec2::new(
                single_card_size.x,
                single_card_size.y + 3.0 * CARD_STACK_Y_SPACING
            )
        );
    }
}
