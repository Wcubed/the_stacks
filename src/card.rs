use crate::card_types::{CardCategory, CardType, TREE, WORKER};
use crate::stack_utils::{set_stack_card_transforms, StackCreation, CARD_STACK_Y_SPACING};
use crate::{card_types, GameState};
use bevy::math::{const_vec2, const_vec3};
use bevy::prelude::*;
use bevy::render::camera::Camera2d;
use bevy_asset_loader::{AssetCollection, AssetLoader};
use std::collections::HashSet;

/// Z position stacks have when laying on the ground.
pub const STACK_ROOT_Z: f32 = 1.0;
pub const STACK_DRAG_Z: f32 = 200.0;
/// Extra scaling a stack gets when a user "picks it up".
/// This should help in giving the illusion of the stack being above the other stacks.
const STACK_DRAG_SCALE: Vec3 = const_vec3!([1.1, 1.1, 1.]);

/// Max amount of display units a stack moves per second if it overlaps with another.
const STACK_OVERLAP_MOVEMENT: f32 = 1000.0;
/// Spacing that stacks want to keep between each other.
const STACK_OVERLAP_SPACING: Vec2 = const_vec2!([10.0, 10.0]);

/// Tiny change in Z position, used to put sprites "in front" of other sprites.
pub const DELTA_Z: f32 = 0.001;

pub struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        AssetLoader::new(GameState::AssetLoading)
            .continue_to_state(GameState::Run)
            .with_collection::<CardImages>()
            .with_collection::<CardFonts>()
            .build(app);

        app.add_event::<StackDroppedEvent>()
            .add_event::<CardPickedUpEvent>()
            .insert_resource(MouseWorldPos(None))
            .add_system_to_stage(CoreStage::PreUpdate, mouse_world_pos_update_system)
            .add_system_set(
                SystemSet::on_exit(GameState::AssetLoading).with_system(on_assets_loaded),
            )
            .add_system_set(SystemSet::on_enter(GameState::Run).with_system(spawn_test_cards))
            .add_system_set(
                SystemSet::on_update(GameState::Run)
                    .with_system(stack_mouse_drag_system)
                    .with_system(card_mouse_pickup_system)
                    .with_system(stack_mouse_drop_system)
                    .with_system(card_hover_system)
                    .with_system(hover_drag_cursor_system)
                    .with_system(stack_overlap_nudging_system)
                    .with_system(dropped_stack_merging_system),
            );
    }
}

#[derive(AssetCollection)]
pub struct CardImages {
    #[asset(path = "vector_images/card_background.png")]
    pub background: Handle<Image>,
    #[asset(path = "vector_images/card_border.png")]
    pub border: Handle<Image>,
    #[asset(path = "vector_images/card_hover_overlay.png")]
    pub hover_overlay: Handle<Image>,
}

#[derive(AssetCollection)]
pub struct CardFonts {
    #[asset(path = "fonts/FallingSky-JKwK.otf")]
    pub title: Handle<Font>,
}

/// Resource which indicates where in the world the mouse currently is.
pub struct MouseWorldPos(Option<Vec2>);

/// Resource indicating how large the card texture looks on-screen.
#[derive(Deref, DerefMut)]
pub struct CardVisualSize(pub(crate) Vec2);

#[derive(Component, Clone, Eq, PartialEq)]
pub struct Card {
    pub(crate) title: &'static str,
    pub(crate) category: CardCategory,
}

impl Card {
    pub fn is_type(&self, card_type: CardType) -> bool {
        self.title == card_type.title && self.category == card_type.category
    }
}

/// Marks stacks which should have physics applied.
#[derive(Component)]
pub struct StackPhysics;

/// Indicates this is the root entity of a stack of cards.
/// Contains all cards, in-order.
/// Each individual card is a stack with a single card.
/// Stacked cards are all direct children of the Stack entity.
/// TODO (Wybe 2022-05-15): Write extensive tests for stacking and un-stacking.
#[derive(Component, Deref, Debug)]
pub struct CardStack(pub Vec<Entity>);

/// Marks an entity that shows a card is being hovered.
#[derive(Component)]
pub struct IsCardHoverOverlay;

#[derive(Component, Deref, DerefMut)]
pub struct StackRelativeDragPosition(Vec2);

/// Indicates a card is being hovered with the mouse.
#[derive(Component)]
pub struct HoveredCard {
    relative_hover_pos: Vec2,
}

/// Event sent by the [card_mouse_drag_system] when the user drops a card.
pub struct StackDroppedEvent(Entity, GlobalTransform);

/// Event sent by the [card_mouse_drag_system] when the user picks up a card.
pub struct CardPickedUpEvent(Entity);

pub fn on_assets_loaded(
    mut commands: Commands,
    card_images: Res<CardImages>,
    card_fonts: Res<CardFonts>,
    images: Res<Assets<Image>>,
) {
    // Can call `unwrap()` because the asset_loader will have caught any missing assets already.
    let card_background = images.get(card_images.background.clone()).unwrap();
    commands.insert_resource(CardVisualSize(card_background.size()));

    commands.insert_resource(StackCreation::new(
        &card_images,
        &card_fonts,
        card_background.size(),
    ));
}

pub fn spawn_test_cards(mut commands: Commands, creation: Res<StackCreation>) {
    creation.spawn_stack(&mut commands, Vec2::ZERO, &[TREE, TREE, TREE]);
    creation.spawn_stack(&mut commands, Vec2::ZERO, &[WORKER, WORKER]);
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

pub fn stack_mouse_drop_system(
    mut commands: Commands,
    mouse_button: Res<Input<MouseButton>>,
    mut dragged_stack_query: Query<
        (Entity, &mut Transform, &GlobalTransform),
        With<StackRelativeDragPosition>,
    >,
    mut stack_dropped_writer: EventWriter<StackDroppedEvent>,
) {
    if mouse_button.just_released(MouseButton::Left) {
        for (root, mut transform, global_transform) in dragged_stack_query.iter_mut() {
            commands.entity(root).remove::<StackRelativeDragPosition>();
            transform.translation.z = STACK_ROOT_Z;
            transform.scale = Vec3::ONE;

            stack_dropped_writer.send(StackDroppedEvent(root, *global_transform));
        }
    }
}

pub fn stack_mouse_drag_system(
    maybe_mouse_world_pos: Res<MouseWorldPos>,
    mut dragged_stack_query: Query<
        (Entity, &mut Transform, &StackRelativeDragPosition),
        With<CardStack>,
    >,
    mut stack_dropped_reader: EventReader<StackDroppedEvent>,
) {
    if let Some(mouse_world_pos) = maybe_mouse_world_pos.0 {
        let dropped_stacks: HashSet<Entity> =
            stack_dropped_reader.iter().map(|entry| entry.0).collect();

        for (stack, mut transform, drag_position) in dragged_stack_query.iter_mut() {
            if dropped_stacks.contains(&stack) {
                // Shouldn't drag a stack around that has just gotten dropped.
                continue;
            }

            transform.translation = (mouse_world_pos - drag_position.0).extend(STACK_DRAG_Z);
            transform.scale = STACK_DRAG_SCALE;
        }
    }
}

/// Relies on the [card_hover_system] to supply info on which card is being hovered.
/// If the card is the bottom most card of a stack, picks up the whole stack.
/// If it is in the middle of a stack, the stack will be split.
pub fn card_mouse_pickup_system(
    mut commands: Commands,
    mouse_button: Res<Input<MouseButton>>,
    hovered_card_query: Query<(Entity, &Parent, &HoveredCard, &GlobalTransform), With<Card>>,
    stacks: Query<&CardStack>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        for (card_entity, stack_root, hovered_card_component, global_transform) in
            hovered_card_query.iter()
        {
            if let Ok(stack) = stacks.get(stack_root.0) {
                if stack[0] == card_entity {
                    // Picking up the whole stack
                    commands
                        .entity(stack_root.0)
                        .insert(StackRelativeDragPosition(
                            hovered_card_component.relative_hover_pos,
                        ))
                        .remove::<StackPhysics>();
                } else {
                    // Picking up some other card in the stack, which means splitting it.
                    let new_root = crate::stack_utils::split_stack(
                        &mut commands,
                        stack_root.0,
                        &stack.0,
                        card_entity,
                        global_transform,
                    );

                    if let Some(root) = new_root {
                        commands
                            .entity(root)
                            .insert(StackRelativeDragPosition(
                                hovered_card_component.relative_hover_pos,
                            ))
                            .remove::<StackPhysics>();
                    }
                }
            }
        }
    }
}

pub fn card_hover_system(
    mut commands: Commands,
    maybe_mouse_world_pos: Res<MouseWorldPos>,
    card_query: Query<(Entity, &GlobalTransform, &Children), With<Card>>,
    stack_dragged_query: Query<(&StackRelativeDragPosition, &CardStack)>,
    mut card_hover_overlay_query: Query<&mut Visibility, With<IsCardHoverOverlay>>,
    card_visual_size: Res<CardVisualSize>,
) {
    if let Some(mouse_world_pos) = maybe_mouse_world_pos.0 {
        let mut hovered_card = None;

        if let Ok((relative_drag_pos, cards_in_stack)) = stack_dragged_query.get_single() {
            // User is dragging a stack. The root is the card they are hovering.
            let (_, global_transform, _) = card_query.get(cards_in_stack[0]).unwrap();
            hovered_card = Some((cards_in_stack[0], relative_drag_pos.0, global_transform));
        } else {
            // User isn't dragging a stack. See which card they are hovering.

            for (entity, transform, _) in card_query.iter() {
                if let Some(relative_pos) =
                    in_bounds(card_visual_size.0, transform, mouse_world_pos)
                {
                    if let Some((_, _, highest_transform)) = hovered_card {
                        if highest_transform.translation.z < transform.translation.z {
                            hovered_card = Some((entity, relative_pos, transform));
                        }
                    } else {
                        hovered_card = Some((entity, relative_pos, transform));
                    }
                }
            }
        }

        if let Some((hovered_entity, relative_pos, _)) = hovered_card {
            commands.entity(hovered_entity).insert(HoveredCard {
                relative_hover_pos: relative_pos,
            });

            let children = card_query
                .get_component::<Children>(hovered_entity)
                .unwrap();
            for &child in children.iter() {
                if let Ok(mut visibility) = card_hover_overlay_query.get_mut(child) {
                    // Don't mutate if not necessary.
                    if !visibility.is_visible {
                        visibility.is_visible = true;
                    }
                }
            }
        }

        // Clear all other hover markers, so there aren't any stray ones lying around.
        for (entity, _, children) in card_query.iter() {
            if let Some((hovered_entity, _, _)) = hovered_card {
                if entity == hovered_entity {
                    continue;
                }
            }

            commands.entity(entity).remove::<HoveredCard>();

            for &child in children.iter() {
                if let Ok(mut visibility) = card_hover_overlay_query.get_mut(child) {
                    // Don't mutate if not necessary.
                    if visibility.is_visible {
                        visibility.is_visible = false;
                    }
                }
            }
        }
    }
}

pub fn hover_drag_cursor_system(
    mut windows: ResMut<Windows>,
    hovered_card_query: Query<Entity, With<HoveredCard>>,
    dragged_stack_query: Query<Entity, With<StackRelativeDragPosition>>,
) {
    let primary_window = windows.get_primary_mut().expect("No primary window!");

    if !dragged_stack_query.is_empty() {
        primary_window.set_cursor_icon(CursorIcon::Grabbing);
    } else if !hovered_card_query.is_empty() {
        primary_window.set_cursor_icon(CursorIcon::Grab);
    } else {
        primary_window.set_cursor_icon(CursorIcon::Default);
    }
}

pub fn dropped_stack_merging_system(
    mut commands: Commands,
    stack_query: Query<(Entity, &GlobalTransform, &CardStack)>,
    card_visual_size: Res<CardVisualSize>,
    mut stack_dropped_reader: EventReader<StackDroppedEvent>,
) {
    for StackDroppedEvent(dropped_stack_root, dropped_global_transform) in
        stack_dropped_reader.iter()
    {
        let mut stack_merged = false;

        // Find which card we are overlapping the most.
        // TODO (Wybe 2022-05-14): This should also check if the card we are overlapping is
        //   a valid target to stack with.
        for (stack_root, stack_global_transform, target_stack) in stack_query.iter() {
            if stack_root == *dropped_stack_root {
                // Cannot drop onto self.
                continue;
            }

            let center_of_top_card =
                crate::stack_utils::center_of_top_card(stack_global_transform, target_stack.len());

            // TODO (Wybe 2022-05-24): Also take into account rotating and scaling.
            if in_bounds(
                card_visual_size.0,
                &center_of_top_card,
                dropped_global_transform.translation.truncate(),
            )
            .is_some()
            {
                let (_, _, dropped_stack) = stack_query.get(*dropped_stack_root).unwrap();
                crate::stack_utils::merge_stacks(
                    &mut commands,
                    *dropped_stack_root,
                    dropped_stack,
                    stack_root,
                    target_stack,
                );
                // Stack has been merged, no need to check other stacks.
                stack_merged = true;
                break;
            }
        }

        if !stack_merged {
            // Re-enable physics for the dropped stack.
            commands.entity(*dropped_stack_root).insert(StackPhysics);
        }
    }
}

/// Slowly nudges stacks with [CardPhysics], until they don't overlap.
/// TODO (Wybe 2022-05-21): This currently nudges cards that were just dropped, but not yet added to a stack.
///      It would probably be better to add dropped cards to a stack right away. And to remove picked up cards from a stack right away,
///      instead of next frame.
/// TODO (Wybe 2022-05-24): Take into account scaling and rotation?
pub fn stack_overlap_nudging_system(
    time: Res<Time>,
    mut physics_stacks: Query<(&GlobalTransform, &mut Transform, &CardStack), With<StackPhysics>>,
    card_visual_size: Res<CardVisualSize>,
) {
    let mut combinations = physics_stacks.iter_combinations_mut();
    while let Some(
        [(global_transform1, mut transform1, CardStack(cards_in_stack1)), (global_transform2, mut transform2, CardStack(cards_in_stack2))],
    ) = combinations.fetch_next()
    {
        let stack1_wanted_space =
            crate::stack_utils::stack_visual_size(card_visual_size.0, cards_in_stack1.len())
                + STACK_OVERLAP_SPACING;
        let mut stack1_center = global_transform1.translation.truncate();
        stack1_center.y -= 0.5 * cards_in_stack1.len() as f32 * CARD_STACK_Y_SPACING;

        let stack2_wanted_space =
            crate::stack_utils::stack_visual_size(card_visual_size.0, cards_in_stack2.len())
                + STACK_OVERLAP_SPACING;
        let mut stack2_center = global_transform2.translation.truncate();
        stack2_center.y -= 0.5 * cards_in_stack2.len() as f32 * CARD_STACK_Y_SPACING;

        // TODO (Wybe 2022-05-14): Should we account for scaling and rotation?
        if let Some(total_movement) = get_movement_to_no_longer_overlap(
            stack1_center,
            stack1_wanted_space,
            stack2_center,
            stack2_wanted_space,
        ) {
            let max_movement_this_frame = STACK_OVERLAP_MOVEMENT * time.delta_seconds();

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
fn in_bounds(
    unscaled_size: Vec2,
    transform: &GlobalTransform,
    position_to_check: Vec2,
) -> Option<Vec2> {
    // TODO (Wybe 2022-05-14): Take into account rotation.
    let half_size = unscaled_size * transform.scale.truncate() / 2.0;

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
