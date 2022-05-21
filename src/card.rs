use crate::GameState;
use bevy::math::{const_vec2, const_vec3};
use bevy::prelude::*;
use bevy::render::camera::Camera2d;
use bevy_asset_loader::{AssetCollection, AssetLoader};
use std::collections::HashSet;

const CARD_Z: f32 = 1.0;
const CARD_DRAG_Z: f32 = 200.0;
/// Extra scaling a card gets when a user "picks it up".
/// This should help in giving the illusion of the card being above the other cards.
const CARD_DRAG_SCALE: Vec3 = const_vec3!([1.1, 1.1, 1.]);

/// Max amount of display units a card moves per second if it overlaps with another.
const CARD_OVERLAP_MOVEMENT: f32 = 1000.0;
/// Spacing that cards want to keep between each other.
const CARD_OVERLAP_SPACING: Vec2 = const_vec2!([10.0, 10.0]);

/// Tiny change in Z position, used to put sprites "in front" of other sprites.
const DELTA_Z: f32 = 0.001;

/// How much of the previous card you can see when stacking cards.
const CARD_STACK_Y_SPACING: f32 = 30.0;

const CARD_COLOR: Color = Color::rgb(0.25, 0.25, 0.75);
/// TODO (Wybe 2022-05-14): Convert this into an overlay somehow, instead of changing the card sprite color.
const CARD_HOVER_COLOR: Color = Color::rgb(0.35, 0.35, 0.85);
const CARD_BORDER_COLOR: Color = Color::BLACK;

pub struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        AssetLoader::new(GameState::AssetLoading)
            .continue_to_state(GameState::Run)
            .with_collection::<CardImages>()
            .build(app);

        app.add_event::<CardDroppedEvent>()
            .add_event::<CardPickedUpEvent>()
            .insert_resource(MouseWorldPos(None))
            .add_system_to_stage(CoreStage::PreUpdate, mouse_world_pos_update_system)
            .add_system_set(
                SystemSet::on_exit(GameState::AssetLoading).with_system(on_assets_loaded),
            )
            .add_system_set(SystemSet::on_enter(GameState::Run).with_system(spawn_test_cards))
            .add_system_set(
                SystemSet::on_update(GameState::Run)
                    .with_system(card_mouse_drag_system)
                    .with_system(card_mouse_pickup_system)
                    .with_system(card_mouse_drop_system)
                    .with_system(card_hover_system)
                    .with_system(stack_overlap_nudging_system)
                    .with_system(card_stacking_system),
            );
    }
}

#[derive(AssetCollection)]
pub struct CardImages {
    #[asset(path = "vector_images/card_background.png")]
    pub(crate) background: Handle<Image>,
    #[asset(path = "vector_images/card_border.png")]
    border: Handle<Image>,
}

/// Resource which indicates where in the world the mouse currently is.
pub struct MouseWorldPos(Option<Vec2>);

#[derive(Component)]
pub struct Card;

#[derive(Component)]
pub struct CardPhysics;

#[derive(Component)]
/// Indicates this is the topmost (root) card of a stack of cards.
/// All individual cards are root cards.
/// Contains the list of cards in the stack, starting from the root.
/// TODO (Wybe 2022-05-15): Write extensive tests for stacking and un-stacking.
pub struct CardsInStack(Vec<Entity>);

#[derive(Component)]
pub struct IsBottomCardOfStack;

#[derive(Component)]
/// Points to the root card of a stack.
/// When this card is the root, this points to itself.
pub struct RootCardOfThisStack(Entity);

#[derive(Component, Deref, DerefMut)]
pub struct CardRelativeDragPosition(Vec2);

#[derive(Component)]
/// Indicates a card is being hovered with the mouse.
pub struct HoveredCard {
    relative_hover_pos: Vec2,
}

#[derive(Deref, DerefMut)]
pub struct CardVisualSize(Vec2);

/// Event sent by the [card_mouse_drag_system] when the user drops a card.
pub struct CardDroppedEvent(Entity, GlobalTransform);

/// Event sent by the [card_mouse_drag_system] when the user picks up a card.
pub struct CardPickedUpEvent(Entity);

pub fn on_assets_loaded(
    mut commands: Commands,
    card_images: Res<CardImages>,
    images: Res<Assets<Image>>,
) {
    // Can call `unwrap()` because the asset_loader will have caught any missing assets already.
    let card_background = images.get(card_images.background.clone()).unwrap();
    commands.insert_resource(CardVisualSize(card_background.size()));
}

pub fn spawn_test_cards(mut commands: Commands, card_images: Res<CardImages>) {
    for _ in 0..10 {
        spawn_card(&mut commands, &card_images);
    }
}

pub fn spawn_card(commands: &mut Commands, card_images: &Res<CardImages>) {
    let id = commands
        .spawn_bundle(SpriteBundle {
            texture: card_images.background.clone(),
            sprite: Sprite {
                color: CARD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Card)
        .insert(IsBottomCardOfStack)
        .insert(CardPhysics)
        .with_children(|parent| {
            parent.spawn_bundle(SpriteBundle {
                texture: card_images.border.clone(),
                transform: Transform::from_xyz(0.0, 0.0, DELTA_Z),
                sprite: Sprite {
                    color: CARD_BORDER_COLOR,
                    ..default()
                },
                ..default()
            });
        })
        .id();

    // Add the components that rely on knowing the entity id.
    commands
        .entity(id)
        .insert(RootCardOfThisStack(id))
        .insert(CardsInStack(vec![id]));
}

/// Should be added to [PreUpdate](CoreStage::PreUpdate) to make sure the mouse position is
/// up-to-date when the rest of the systems run.
pub fn mouse_world_pos_update_system(
    windows: Res<Windows>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut mouse_world_pos: ResMut<MouseWorldPos>,
) {
    let primary_window = windows.get_primary().expect("No primary window!");

    if let (Ok((camera, camera_transform)), Some(mouse_window_pos)) =
        (camera_query.get_single(), primary_window.cursor_position())
    {
        mouse_world_pos.0 = Some(window_pos_to_world_pos(
            camera,
            camera_transform,
            primary_window,
            mouse_window_pos,
        ));
    } else {
        mouse_world_pos.0 = None;
    }
}

pub fn card_mouse_drop_system(
    mut commands: Commands,
    mouse_button: Res<Input<MouseButton>>,
    mut dragged_card_query: Query<
        (Entity, &mut Transform, &GlobalTransform),
        (With<Card>, With<CardRelativeDragPosition>),
    >,
    mut card_dropped_writer: EventWriter<CardDroppedEvent>,
) {
    if mouse_button.just_released(MouseButton::Left) {
        for (card, mut transform, global_transform) in dragged_card_query.iter_mut() {
            commands.entity(card).remove::<CardRelativeDragPosition>();
            transform.translation.z = CARD_Z;
            transform.scale = Vec3::ONE;

            card_dropped_writer.send(CardDroppedEvent(card, *global_transform));
        }
    }
}

pub fn card_mouse_drag_system(
    maybe_mouse_world_pos: Res<MouseWorldPos>,
    mut dragged_card_query: Query<(Entity, &mut Transform, &CardRelativeDragPosition), With<Card>>,
    mut card_dropped_reader: EventReader<CardDroppedEvent>,
) {
    if let Some(mouse_world_pos) = maybe_mouse_world_pos.0 {
        let dropped_cards: HashSet<Entity> =
            card_dropped_reader.iter().map(|entry| entry.0).collect();

        for (card, mut transform, drag_position) in dragged_card_query.iter_mut() {
            if dropped_cards.contains(&card) {
                // Shouldn't drag a card around that has just gotten dropped.
                continue;
            }

            transform.translation = (mouse_world_pos - drag_position.0).extend(CARD_DRAG_Z);
            transform.scale = CARD_DRAG_SCALE;
        }
    }
}

pub fn card_mouse_pickup_system(
    mut commands: Commands,
    mouse_button: Res<Input<MouseButton>>,
    hovered_but_not_dragged_card_query: Query<
        (Entity, &HoveredCard),
        (With<Card>, Without<CardRelativeDragPosition>),
    >,
    root_card_transforms: Query<(&RootCardOfThisStack, &GlobalTransform), With<Card>>,
    stacks: Query<&CardsInStack, With<Card>>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        for (picked_up_card, hovered_card) in hovered_but_not_dragged_card_query.iter() {
            commands
                .entity(picked_up_card)
                .insert(CardRelativeDragPosition(hovered_card.relative_hover_pos))
                .remove::<CardPhysics>();

            if let Ok((RootCardOfThisStack(root_card), picked_up_global_transform)) =
                root_card_transforms.get(picked_up_card)
            {
                if *root_card == picked_up_card {
                    // No need to split a stack at the root card.
                    continue;
                }

                if let Ok(CardsInStack(stack)) = stacks.get(*root_card) {
                    split_stack(
                        &mut commands,
                        stack,
                        picked_up_card,
                        picked_up_global_transform,
                    );
                }
            }
        }
    }
}

pub fn card_hover_system(
    mut commands: Commands,
    maybe_mouse_world_pos: Res<MouseWorldPos>,
    mut card_query: Query<(Entity, &GlobalTransform, &mut Sprite), With<Card>>,
    card_visual_size: Res<CardVisualSize>,
) {
    if let Some(mouse_world_pos) = maybe_mouse_world_pos.0 {
        let mut topmost_card = None;

        for (entity, transform, _) in card_query.iter_mut() {
            if let Some(relative_pos) = in_bounds(card_visual_size.0, transform, mouse_world_pos) {
                if let Some((_, _, highest_z)) = topmost_card {
                    if highest_z < transform.translation.z {
                        topmost_card = Some((entity, relative_pos, transform.translation.z));
                    }
                } else {
                    topmost_card = Some((entity, relative_pos, transform.translation.z));
                }
            }
        }

        if let Some((hovered_entity, relative_pos, _)) = &topmost_card {
            let mut topmost_sprite = card_query
                .get_component_mut::<Sprite>(*hovered_entity)
                .unwrap();
            topmost_sprite.color = CARD_HOVER_COLOR;

            commands.entity(*hovered_entity).insert(HoveredCard {
                relative_hover_pos: *relative_pos,
            });
        }

        // Clear all other hovers, so we don't leave stray ones lying around.
        for (entity, _, mut sprite) in card_query.iter_mut() {
            if let Some((hovered_entity, _, _)) = topmost_card {
                if entity == hovered_entity {
                    continue;
                }
            }

            sprite.color = CARD_COLOR;
            commands.entity(entity).remove::<HoveredCard>();
        }
    }
}

pub fn card_stacking_system(
    mut commands: Commands,
    targetable_card_query: Query<
        (Entity, &GlobalTransform, &RootCardOfThisStack),
        (With<Card>, With<IsBottomCardOfStack>),
    >,
    stacks: Query<&CardsInStack, With<Card>>,
    card_visual_size: Res<CardVisualSize>,
    mut card_dropped_reader: EventReader<CardDroppedEvent>,
) {
    for CardDroppedEvent(dropped_entity, dropped_global_transform) in card_dropped_reader.iter() {
        let mut closest_drop_target = None;

        // Find which card we are overlapping the most.
        // TODO (Wybe 2022-05-14): This should also check if the card we are overlapping is
        //   a valid target to stack with.
        for (entity, global_transform, root_card_of_stack) in targetable_card_query.iter() {
            if entity == *dropped_entity {
                // Cannot drop onto self.
                continue;
            }
            if root_card_of_stack.0 == *dropped_entity {
                // Should not drop onto the bottom card of our own stack.
                continue;
            }

            if let Some(distance) = get_movement_to_no_longer_overlap(
                global_transform.translation.truncate(),
                card_visual_size.0,
                dropped_global_transform.translation.truncate(),
                card_visual_size.0,
            )
            .map(|v| v.length())
            {
                if let Some((shortest_distance, _)) = closest_drop_target {
                    if distance < shortest_distance {
                        closest_drop_target = Some((distance, root_card_of_stack.0));
                    }
                } else {
                    closest_drop_target = Some((distance, root_card_of_stack.0));
                }
            }
        }

        // If we have a target. We need to add this card on top of it.
        if let Some((_, root_card_of_stack)) = closest_drop_target {
            if let (Ok(source_stack), Ok(target_stack)) =
                (stacks.get(*dropped_entity), stacks.get(root_card_of_stack))
            {
                add_card_to_stack(&mut commands, &source_stack.0, &target_stack.0);
            }
        } else {
            // Re-enable physics for the card.
            commands.entity(*dropped_entity).insert(CardPhysics);
        }
    }
}

/// Adds the cards of the `source_stack` to the bottom of the `target_stack`.
/// Assumes no duplicate cards.
///
/// Effects are applied via `Commands`, which means it is visible next update.
pub fn add_card_to_stack(
    commands: &mut Commands,
    source_stack: &[Entity],
    target_stack: &[Entity],
) {
    if source_stack.is_empty() || target_stack.is_empty() {
        return;
    }

    let &root_card_of_source_stack = source_stack.first().unwrap();
    let &root_card_of_target_stack = target_stack.first().unwrap();
    let &bottom_card_of_target_stack = target_stack.last().unwrap();

    // Put this card in front of the parent.
    let new_transform = Transform::from_xyz(0., -CARD_STACK_Y_SPACING, DELTA_Z);

    commands
        .entity(bottom_card_of_target_stack)
        .add_child(root_card_of_source_stack)
        .remove::<IsBottomCardOfStack>();
    commands
        .entity(root_card_of_source_stack)
        .remove::<CardsInStack>()
        .insert(new_transform);

    for &card in source_stack {
        commands
            .entity(card)
            .insert(RootCardOfThisStack(root_card_of_target_stack));
    }

    let mut cards_in_new_stack = target_stack.to_owned();
    cards_in_new_stack.extend(source_stack);
    commands
        .entity(root_card_of_target_stack)
        .insert(CardsInStack(cards_in_new_stack));
}

/// Splits a stack so that the `new_root` card is the root of a new stack.
/// Effects are applied via `Commands`, which means it is visible next update.
pub fn split_stack(
    commands: &mut Commands,
    stack: &[Entity],
    new_root: Entity,
    new_root_global_transform: &GlobalTransform,
) {
    if stack.is_empty() {
        return;
    }
    if let Some(new_root_index) = stack.iter().position(|&e| e == new_root) {
        if new_root_index == 0 {
            // Picking up the root of a stack. No need to split.
            return;
        }

        let &new_bottom_card = stack.get(new_root_index - 1).unwrap();
        commands
            .entity(new_bottom_card)
            .remove_children(&[new_root])
            .insert(IsBottomCardOfStack);

        let bottom_stack = &stack[0..new_root_index];
        let top_stack = &stack[new_root_index..stack.len()];

        let &bottom_root = stack.first().unwrap();
        commands
            .entity(bottom_root)
            .insert(CardsInStack(bottom_stack.to_vec()));

        let new_root_transform = Transform::from(*new_root_global_transform);
        commands
            .entity(new_root)
            .insert(CardsInStack(top_stack.to_vec()))
            .insert(new_root_transform);

        for &card in top_stack {
            commands.entity(card).insert(RootCardOfThisStack(new_root));
        }
    }
}

/// Slowly nudges stacks with [CardPhysics], until they don't overlap.
/// TODO (Wybe 2022-05-21): This currently nudges cards that were just dropped, but not yet added to a stack.
///      It would probably be better to add dropped cards to a stack right away. And to remove picked up cards from a stack right away,
///      instead of next frame.
pub fn stack_overlap_nudging_system(
    time: Res<Time>,
    mut physics_stacks: Query<
        (&GlobalTransform, &mut Transform, &CardsInStack),
        (With<Card>, With<CardPhysics>),
    >,
    card_visual_size: Res<CardVisualSize>,
) {
    let mut combinations = physics_stacks.iter_combinations_mut();
    while let Some(
        [(global_transform1, mut transform1, CardsInStack(cards_in_stack1)), (global_transform2, mut transform2, CardsInStack(cards_in_stack2))],
    ) = combinations.fetch_next()
    {
        let stack1_wanted_space =
            stack_visual_size(card_visual_size.0, cards_in_stack1.len()) + CARD_OVERLAP_SPACING;
        let mut stack1_center = global_transform1.translation.truncate();
        stack1_center.y -= 0.5 * cards_in_stack1.len() as f32 * CARD_STACK_Y_SPACING;

        let stack2_wanted_space =
            stack_visual_size(card_visual_size.0, cards_in_stack2.len()) + CARD_OVERLAP_SPACING;
        let mut stack2_center = global_transform2.translation.truncate();
        stack2_center.y -= 0.5 * cards_in_stack2.len() as f32 * CARD_STACK_Y_SPACING;

        // TODO (Wybe 2022-05-14): Should we account for scaling and rotation?
        if let Some(total_movement) = get_movement_to_no_longer_overlap(
            stack1_center,
            stack1_wanted_space,
            stack2_center,
            stack2_wanted_space,
        ) {
            let max_movement_this_frame = CARD_OVERLAP_MOVEMENT * time.delta_seconds();

            let movement = if total_movement.length() <= max_movement_this_frame {
                total_movement.extend(0.0)
            } else {
                (total_movement.normalize() * max_movement_this_frame).extend(0.0)
            };

            transform1.translation += movement;
            transform2.translation -= movement;
        }
    }
}

fn stack_visual_size(single_card_visual_size: Vec2, cards_in_stack: usize) -> Vec2 {
    Vec2::new(
        single_card_visual_size.x,
        single_card_visual_size.y + (cards_in_stack as f32 * CARD_STACK_Y_SPACING),
    )
}

fn window_pos_to_world_pos(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    window: &Window,
    window_pos: Vec2,
) -> Vec2 {
    let window_size = Vec2::new(window.width(), window.height());
    // Converts to [-1..1] range.
    let gpu_mouse_position = (window_pos / window_size) * 2.0 - Vec2::ONE;
    (camera_transform.compute_matrix() * camera.projection_matrix.inverse())
        .project_point3(gpu_mouse_position.extend(-1.0))
        .truncate()
}

/// Returns where in the bounds the position is located.
/// `None` if the position is not in bounds.
/// Assumes the `size`'s origin is at it's center.
fn in_bounds(size: Vec2, transform: &GlobalTransform, position_to_check: Vec2) -> Option<Vec2> {
    // TODO (Wybe 2022-05-14): Take into account rotation.
    let half_size = size * transform.scale.truncate() / 2.0;

    let pos_in_bounds = position_to_check - transform.translation.truncate();

    if pos_in_bounds.x >= -half_size.x
        && pos_in_bounds.x <= half_size.x
        && pos_in_bounds.y >= -half_size.y
        && pos_in_bounds.y <= half_size.y
    {
        Some(pos_in_bounds)
    } else {
        None
    }
}

/// Returns the shortest distance two rectangles should move in, in order not to overlap anymore.
/// The first rectangle given should use the movement vector as-is, the second should invert it.
/// Returns `None` if the rectangles are not overlapping.
/// TODO (Wybe 2022-05-14): Take into account scaling and rotation?
///                         And then accept a GlobalTransform instead of a Vec2.
fn get_movement_to_no_longer_overlap(
    pos1: Vec2,
    size1: Vec2,
    pos2: Vec2,
    size2: Vec2,
) -> Option<Vec2> {
    let minimum_allowed_distance = (size1 / 2.0) + (size2 / 2.0);

    let distance = pos1 - pos2;
    let abs_distance = distance.abs();
    let overlap = minimum_allowed_distance - abs_distance;
    let mut movement = overlap * (distance / abs_distance);

    if overlap.x > 0.0 && overlap.y > 0.0 {
        if movement.x.is_nan() {
            movement.x = minimum_allowed_distance.x;
        }
        if movement.y.is_nan() {
            movement.y = minimum_allowed_distance.y;
        }

        // Select shortest distance.
        if overlap.x < overlap.y {
            movement.y = 0.0;
        } else {
            movement.x = 0.0;
        }

        // Divide by 2, because both rectangles are going to move.
        Some(movement / 2.0)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{card::get_movement_to_no_longer_overlap, Vec2};

    #[test]
    fn test_get_movement_to_no_longer_overlap() {
        let overlap = get_movement_to_no_longer_overlap(
            Vec2::new(10.0, 10.0),
            Vec2::new(6.0, 6.0),
            Vec2::new(12.0, 12.0),
            Vec2::new(4.0, 4.0),
        )
        .unwrap();
        assert_eq!(overlap, Vec2::new(0.0, -1.5));

        let overlap_invert_arguments = get_movement_to_no_longer_overlap(
            Vec2::new(12.0, 12.0),
            Vec2::new(4.0, 4.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(6.0, 6.0),
        )
        .unwrap();

        // When the two rectangles are swapped, the output vector should also swap it's sign.
        assert_eq!(overlap, overlap_invert_arguments * -1.0);
    }

    #[test]
    fn test_get_movement_to_no_longer_overlap_when_no_overlap() {
        let overlap = get_movement_to_no_longer_overlap(
            Vec2::new(10.0, 10.0),
            Vec2::new(6.0, 6.0),
            Vec2::new(16.0, 10.0),
            Vec2::new(4.0, 4.0),
        );
        assert_eq!(overlap, None);
    }
}
