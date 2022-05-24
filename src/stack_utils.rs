use crate::card::{
    Card, CardFonts, CardImages, CardStack, IsCardHoverOverlay, StackPhysics, DELTA_Z,
};
use bevy::prelude::*;

/// How much of the previous card you can see when stacking cards.
pub const CARD_STACK_Y_SPACING: f32 = 50.0;

const CARD_HOVER_OVERLAY_COLOR: Color = Color::rgba(1., 1., 1., 0.1);
const CARD_FOREGROUND_COLOR: Color = Color::rgb(0.8, 0.8, 0.8);

/// Resource that contains everything needed to create new cards.
pub struct StackCreation {
    background: Handle<Image>,
    border: Handle<Image>,
    hover_overlay: Handle<Image>,
    title_style: TextStyle,
    title_transform: Transform,
}

impl StackCreation {
    pub fn new(images: &CardImages, fonts: &CardFonts, visual_size: Vec2) -> Self {
        StackCreation {
            background: images.background.clone(),
            border: images.border.clone(),
            hover_overlay: images.hover_overlay.clone(),
            title_style: TextStyle {
                font: fonts.title.clone(),
                font_size: CARD_STACK_Y_SPACING,
                color: CARD_FOREGROUND_COLOR,
            },
            title_transform: Transform::from_xyz(
                0.,
                0.5 * (visual_size.y - CARD_STACK_Y_SPACING),
                DELTA_Z,
            ),
        }
    }

    pub fn spawn_stack(&self, commands: &mut Commands, position: Vec2, cards: &[Card]) {
        let entities: Vec<Entity> = cards
            .iter()
            .map(|card| self.spawn_card(commands, card))
            .collect();

        spawn_stack_root(commands, position, &entities);
        set_stack_card_transforms(commands, &entities);
    }

    /// Spawns a loose card. Should be added to a stack straight away.
    fn spawn_card(&self, commands: &mut Commands, card: &Card) -> Entity {
        commands
            .spawn_bundle(SpriteBundle {
                texture: self.background.clone(),
                sprite: Sprite {
                    color: card.card_type.background_color(),
                    ..default()
                },
                ..default()
            })
            .insert(card.clone())
            .with_children(|parent| {
                // Border
                parent.spawn_bundle(SpriteBundle {
                    texture: self.border.clone(),
                    transform: Transform::from_xyz(0.0, 0.0, DELTA_Z),
                    sprite: Sprite {
                        color: CARD_FOREGROUND_COLOR,
                        ..default()
                    },
                    ..default()
                });
                // Title text
                parent.spawn_bundle(Text2dBundle {
                    text: Text::with_section(
                        card.title,
                        self.title_style.clone(),
                        TextAlignment {
                            vertical: VerticalAlign::Center,
                            horizontal: HorizontalAlign::Center,
                        },
                    ),
                    transform: self.title_transform,
                    ..default()
                });
                // Hover overlay
                parent
                    .spawn_bundle(SpriteBundle {
                        texture: self.hover_overlay.clone(),
                        sprite: Sprite {
                            color: CARD_HOVER_OVERLAY_COLOR,
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, 0.0, DELTA_Z * 1.5),
                        visibility: Visibility { is_visible: false },
                        ..default()
                    })
                    .insert(IsCardHoverOverlay);
            })
            .id()
    }
}

fn spawn_stack_root(commands: &mut Commands, position: Vec2, cards: &[Entity]) -> Entity {
    commands
        .spawn_bundle(TransformBundle::from_transform(Transform::from_xyz(
            position.x, position.y, 0.,
        )))
        .insert(StackPhysics)
        .insert_children(0, cards)
        .insert(CardStack(Vec::from(cards)))
        .id()
}

/// When given a stack of cards, this function stacks them all nicely.
/// Applies via commands, so effects are only visible next frame.
pub fn set_stack_card_transforms(commands: &mut Commands, stack: &[Entity]) {
    for (i, &card) in stack.iter().enumerate() {
        commands.entity(card).insert(Transform::from_xyz(
            0.,
            -CARD_STACK_Y_SPACING * i as f32,
            // Leave Z spacing for card overlays and such.
            // TODO (Wybe 2022-05-24): Is there a better way than just arbitrarily keeping a certain space?
            DELTA_Z * i as f32 * 2.,
        ));
    }
}

pub fn stack_visual_size(single_card_visual_size: Vec2, cards_in_stack: usize) -> Vec2 {
    Vec2::new(
        single_card_visual_size.x,
        single_card_visual_size.y + (cards_in_stack as f32 * CARD_STACK_Y_SPACING),
    )
}

/// Removes a card from the world.
/// It does not matter if this card is in the middle of a stack,
/// or the only card in a stack. This function will handle it gracefully.
/// The effects are applied via [Commands].
pub fn delete_card(
    commands: &mut Commands,
    card_to_delete: Entity,
    stack_root: Entity,
    stack: &[Entity],
) {
    // TODO (Wybe 2022-05-24): There is probably a more efficient way than re-initializing
    //      the whole stack's Vec every time a card is deleted. but this works for now.
    //      (don't do pre-mature optimizations and all that).

    if stack[0] == card_to_delete && stack.len() == 1 {
        // Last card in the stack. Delete the stack as well.
        commands.entity(stack_root).despawn_recursive();
    } else {
        let new_stack = CardStack(
            stack
                .iter()
                .copied()
                .filter(|&e| e != card_to_delete)
                .collect(),
        );
        set_stack_card_transforms(commands, &new_stack.0);
        commands.entity(stack_root).insert(new_stack);
    }

    commands.entity(card_to_delete).despawn_recursive();
}

pub fn center_of_top_card(
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

/// Adds the cards of the `source_stack` to the top of the `target_stack`.
/// Assumes no duplicate cards.
///
/// Effects are applied via `Commands`, which means it is visible next update.
pub fn merge_stacks(
    commands: &mut Commands,
    source_root: Entity,
    source_stack: &[Entity],
    target_root: Entity,
    target_stack: &[Entity],
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
}

/// Splits a stack so that the `new_root` card is the root of a new stack.
/// Effects are applied via `Commands`, which means it is visible next update.
///
/// Returns the Entity id of the newly created stack root, if the stack needed to be split.
pub fn split_stack(
    commands: &mut Commands,
    stack_root: Entity,
    stack: &[Entity],
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
        let new_root_id = spawn_stack_root(
            commands,
            new_bottom_card_global_transform.translation.truncate(),
            top_stack,
        );

        set_stack_card_transforms(commands, top_stack);

        Some(new_root_id)
    } else {
        None
    }
}
