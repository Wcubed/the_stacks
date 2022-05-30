use crate::card_packs::BUY_FOREST_PACK;
use crate::card_types::{CardCategory, CardType, COIN, MARKET, TREE, WORKER};
use crate::recipe::{is_ongoing_recipe_valid_for_stack, OngoingRecipe, Recipes};
use crate::stack_utils::{
    get_semi_random_stack_root_z, global_center_of_top_card, merge_stacks,
    relative_center_of_nth_card_in_stack, StackCreation, CARD_STACK_Y_SPACING, STACK_ROOT_Z_RANGE,
};
use crate::GameState;
use bevy::math::{const_vec2, const_vec3};
use bevy::prelude::*;
use bevy::render::camera::Camera2d;
use bevy_asset_loader::{AssetCollection, AssetLoader};
use std::collections::HashSet;

/// Dragged cards have a z value that is higher than the cards that are still on the "floor".
/// This way, they will never be overlapped by cards that they should logically be floating above.
pub const STACK_DRAG_Z: f32 = STACK_ROOT_Z_RANGE.end + 100.0;
/// Stacks that move on their own are above everything else, but below stacks dragged by the user.
const STACK_AUTO_MOVE_Z: f32 = STACK_DRAG_Z - 10.0;

/// Extra scaling a stack gets when a user "picks it up".
/// This should help in giving the illusion of the stack being above the other stacks.
const STACK_DRAG_SCALE: Vec3 = const_vec3!([1.1, 1.1, 1.]);

/// Max amount of display units a stack moves per second if it overlaps with another.
const STACK_OVERLAP_MOVEMENT: f32 = 1000.0;
/// Spacing that stacks want to keep between each other.
const STACK_OVERLAP_SPACING: Vec2 = const_vec2!([10.0, 10.0]);

/// Tiny change in Z position, used to put sprites "in front" of other sprites.
pub const DELTA_Z: f32 = 0.001;

/// Stack movement speed in units per second.
/// Used when a stack is moving on it's own.
const STACK_AUTO_MOVEMENT_SPEED: f32 = 2000.0;

const DROP_TARGET_SCALE_ANIMATION_AMOUNT: f32 = 0.02;
const DROP_TARGET_SCALE_ANIMATION_SPEED: f32 = 4.0;

pub struct StackPlugin;

impl Plugin for StackPlugin {
    fn build(&self, app: &mut App) {
        AssetLoader::new(GameState::AssetLoading)
            .continue_to_state(GameState::Run)
            .with_collection::<CardImages>()
            .with_collection::<CardFonts>()
            .build(app);

        app.add_event::<StackDroppedEvent>()
            .add_event::<CardPickedUpEvent>()
            .insert_resource(MouseWorldPos(None))
            .insert_resource(CardVisualSize(Vec2::ONE))
            .insert_resource(StackCreation::default())
            .add_system_to_stage(CoreStage::PreUpdate, mouse_world_pos_update_system)
            .add_system_set(
                SystemSet::on_exit(GameState::AssetLoading).with_system(on_assets_loaded),
            )
            .add_system_set(SystemSet::on_enter(GameState::Run).with_system(spawn_test_cards))
            .add_system_set(
                SystemSet::on_update(GameState::Run)
                    .with_system(stack_mouse_drag_system)
                    .with_system(stack_drop_target_visuals_system)
                    .with_system(stack_drop_overlay_animation_system)
                    .with_system(card_mouse_pickup_system)
                    .with_system(stack_mouse_drop_system)
                    .with_system(card_hover_system)
                    .with_system(hover_drag_cursor_system)
                    .with_system(dropped_stack_merging_system)
                    .with_system(stack_overlap_nudging_system)
                    .with_system(find_stack_movement_target_system)
                    .with_system(stack_move_to_target_system),
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
    #[asset(path = "vector_images/stack_drop_target.png")]
    pub stack_drop_target: Handle<Image>,
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
    pub title: &'static str,
    pub category: CardCategory,
    pub description: &'static str,
    /// Value on a [CardCategory::SystemCard] means the cost to buy something.
    pub value: Option<usize>,
}

impl Card {
    pub fn is_type(&self, card_type: CardType) -> bool {
        self.title == card_type.title && self.category == card_type.category
    }
}

/// Marks stacks which should have physics applied.
#[derive(Component)]
pub struct StackPhysics;

/// Marks a stack that wants to find a nice place to move to.
/// [find_stack_movement_target_system] handles these stacks.
#[derive(Component)]
pub struct StackLookingForMovementTarget;

/// Marks a stack that is moving on it's own towards a target other stack.
/// The goal of a stack moving towards another stack is to combine with that stack.
/// TODO (Wybe 2022-05-25): allow moving towards a fixed location.
#[derive(Component)]
pub struct MovingStackTarget(Entity);

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

/// Marks an entity that sits above a card, to indicate a stack can be dropped there.
#[derive(Component)]
pub struct IsDropTargetOverlay;

#[derive(Component, Deref, DerefMut)]
pub struct StackRelativeDragPosition(Vec2);

/// Indicates a card is being hovered with the mouse.
#[derive(Component)]
pub struct HoveredCard {
    relative_hover_pos: Vec2,
}

/// Event sent by the [card_mouse_drag_system] when the user drops a card.
/// Contains the stack root entity, and it's global transform upon being dropped.
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
    let top_row_zero = Vec2::new(0., 400.0);
    creation.spawn_stack(&mut commands, top_row_zero, MARKET, 1, false);
    creation.spawn_stack(&mut commands, top_row_zero, BUY_FOREST_PACK, 1, false);

    creation.spawn_stack(&mut commands, Vec2::ZERO, TREE, 3, false);
    creation.spawn_stack(&mut commands, Vec2::ZERO, WORKER, 2, false);
    creation.spawn_stack(&mut commands, Vec2::ZERO, COIN, 5, false);
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
    dragged_stack_query: Query<(Entity, &GlobalTransform), With<StackRelativeDragPosition>>,
    mut stack_dropped_writer: EventWriter<StackDroppedEvent>,
) {
    if mouse_button.just_released(MouseButton::Left) {
        for (root, global_transform) in dragged_stack_query.iter() {
            commands.entity(root).remove::<StackRelativeDragPosition>();
            // Translation is handled by the `dropped_stack_merging_system`
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
    stacks: Query<(&CardStack, Option<&OngoingRecipe>)>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        for (card_entity, stack_root, hovered_card_component, global_transform) in
            hovered_card_query.iter()
        {
            if let Ok((stack, maybe_recipe)) = stacks.get(stack_root.0) {
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
                        maybe_recipe,
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

/// Shows where stacks can be dropped, when a stack is being dragged.
///
/// Prevents breaking recipes by dropping cards onto them.
pub fn stack_drop_target_visuals_system(
    mut commands: Commands,
    dragged_stack_query: Query<
        (&CardStack, Option<&OngoingRecipe>),
        With<StackRelativeDragPosition>,
    >,
    potential_target_stacks_query: Query<
        (
            Entity,
            &CardStack,
            ChangeTrackers<CardStack>,
            Option<&OngoingRecipe>,
            Option<ChangeTrackers<OngoingRecipe>>,
        ),
        Without<StackRelativeDragPosition>,
    >,
    mut drop_target_overlay_query: Query<
        (Entity, &Parent, &mut Transform),
        With<IsDropTargetOverlay>,
    >,
    card_query: Query<&Card>,
    card_images: Res<CardImages>,
    recipes: Res<Recipes>,
) {
    // TODO (Wybe 2022-05-26): Refactor this to be less of an if/else spaghetti. Maybe make it multiple systems
    // TODO (Wybe 2022-05-26): When a stack is moving on it's own. Or starts moving on it's own, it shouldn't be droppable.
    // TODO (Wybe 2022-05-26): Make the droppable criteria a function, so that a dropped card can also make use of it.
    // TODO (Wybe 2022-05-26): Ongoing recipe's are only drop targets if the current card wouldn't break the recipe (for example, when adding another tree to a woodcutting worker)
    // This system only works with a single dragged stack. Multiple dragged stacks will be ignored.
    if let Some((dropped_stack, maybe_dropped_recipe)) = dragged_stack_query.iter().next() {
        for (root, stack, stack_changed, maybe_recipe, maybe_recipe_changed) in
            potential_target_stacks_query.iter()
        {
            let merging_would_break_recipe = would_merging_break_ongoing_recipes(
                maybe_dropped_recipe,
                &dropped_stack.0,
                maybe_recipe,
                &stack.0,
                &card_query,
                &recipes,
            );

            if drop_target_overlay_query.is_empty() {
                // Drag just started. Spawn in all overlays
                if !merging_would_break_recipe {
                    spawn_stack_drop_overlay(
                        &mut commands,
                        root,
                        card_images.stack_drop_target.clone(),
                        stack.len(),
                    );
                }
            } else {
                // Drag ongoing. Update changed stacks.
                let recipe_changed = if let Some(recipe_changed) = maybe_recipe_changed {
                    recipe_changed.is_changed() || recipe_changed.is_added()
                } else {
                    false
                };
                if !(stack_changed.is_changed() || recipe_changed) {
                    continue;
                }

                let maybe_overlay = drop_target_overlay_query
                    .iter_mut()
                    .find(|(_, &parent, _)| parent.0 == root)
                    .map(|(overlay, _, transform)| (overlay, transform));

                if let Some((overlay, mut transform)) = maybe_overlay {
                    if merging_would_break_recipe {
                        commands.entity(overlay).despawn_recursive();
                    } else {
                        transform.translation = stack_drop_overlay_relative_transform(stack.len());
                    }
                } else if !merging_would_break_recipe {
                    spawn_stack_drop_overlay(
                        &mut commands,
                        root,
                        card_images.stack_drop_target.clone(),
                        stack.len(),
                    );
                }
            }
        }
    } else {
        // Nothing is being dragged. Delete the overlays.
        for (overlay, _, _) in drop_target_overlay_query.iter() {
            commands.entity(overlay).despawn();
        }
    }
}

fn would_merging_break_ongoing_recipes(
    maybe_dropped_recipe: Option<&OngoingRecipe>,
    dropped_stack: &[Entity],
    maybe_target_recipe: Option<&OngoingRecipe>,
    target_stack: &[Entity],
    card_query: &Query<&Card>,
    recipes: &Res<Recipes>,
) -> bool {
    if maybe_dropped_recipe.is_none() && maybe_target_recipe.is_none() {
        // Can't break recipes that are not there.
        return false;
    }

    let mut merged_stack = target_stack.to_owned();
    merged_stack.extend(dropped_stack);

    let cards: Vec<&Card> = merged_stack
        .iter()
        .filter_map(|&e| card_query.get(e).ok())
        .collect();

    if maybe_dropped_recipe.is_some()
        && !is_ongoing_recipe_valid_for_stack(maybe_dropped_recipe, &cards, recipes)
    {
        // Breaks recipe of the dropped stack.
        return true;
    }
    if maybe_target_recipe.is_some()
        && !is_ongoing_recipe_valid_for_stack(maybe_target_recipe, &cards, recipes)
    {
        // Breaks recipe on the target stack.
        return true;
    }

    false
}

pub fn stack_drop_overlay_animation_system(
    mut drop_target_overlay_query: Query<&mut Transform, With<IsDropTargetOverlay>>,
    time: Res<Time>,
) {
    let scale = Vec3::splat(
        1.0 + (time.seconds_since_startup() * DROP_TARGET_SCALE_ANIMATION_SPEED as f64).sin()
            as f32
            * DROP_TARGET_SCALE_ANIMATION_AMOUNT,
    );

    for mut transform in drop_target_overlay_query.iter_mut() {
        transform.scale = scale;
    }
}

pub fn stack_drop_overlay_relative_transform(amount_of_cards_in_stack: usize) -> Vec3 {
    let mut translation = relative_center_of_nth_card_in_stack(amount_of_cards_in_stack - 1);
    translation.z += DELTA_Z * 3.0;
    translation
}

fn spawn_stack_drop_overlay(
    commands: &mut Commands,
    stack_root: Entity,
    overlay_image: Handle<Image>,
    amount_of_cards_in_stack: usize,
) {
    commands.entity(stack_root).with_children(|parent| {
        parent
            .spawn_bundle(SpriteBundle {
                texture: overlay_image,
                transform: Transform::from_translation(stack_drop_overlay_relative_transform(
                    amount_of_cards_in_stack,
                )),
                ..default()
            })
            .insert(IsDropTargetOverlay);
    });
}

/// TODO (Wybe 2022-05-29): If this is ran before the card_pack_open_system, it tries to add the hover component to an entity that will be deleted.
///                         which causes an error. Fix this in some way.
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
    stack_query: Query<(Entity, &GlobalTransform, &CardStack, Option<&OngoingRecipe>)>,
    card_query: Query<&Card>,
    recipes: Res<Recipes>,
    card_visual_size: Res<CardVisualSize>,
    mut stack_dropped_reader: EventReader<StackDroppedEvent>,
) {
    // TODO (Wybe 2022-05-26): Filter out stacks that were not shown as drop targets.
    for StackDroppedEvent(dropped_stack_root, dropped_global_transform) in
        stack_dropped_reader.iter()
    {
        let mut stack_merged = false;

        let (_, _, dropped_stack, maybe_source_recipe) =
            stack_query.get(*dropped_stack_root).unwrap();

        // Find which card we are overlapping the most.
        // TODO (Wybe 2022-05-14): This should also check if the card we are overlapping is
        //   a valid target to stack with.
        for (stack_root, stack_global_transform, target_stack, maybe_target_recipe) in
            stack_query.iter()
        {
            if stack_root == *dropped_stack_root {
                // Cannot drop onto self.
                continue;
            }
            if would_merging_break_ongoing_recipes(
                maybe_source_recipe,
                dropped_stack,
                maybe_target_recipe,
                target_stack,
                &card_query,
                &recipes,
            ) {
                // Shouldn't break ongoing recipes.
                continue;
            }

            let center_of_top_card = crate::stack_utils::global_center_of_top_card(
                stack_global_transform,
                target_stack.len(),
            );

            // TODO (Wybe 2022-05-24): Also take into account rotating and scaling.
            if in_bounds(
                card_visual_size.0,
                &center_of_top_card,
                dropped_global_transform.translation.truncate(),
            )
            .is_some()
            {
                crate::stack_utils::merge_stacks(
                    &mut commands,
                    *dropped_stack_root,
                    dropped_stack,
                    maybe_source_recipe,
                    stack_root,
                    target_stack,
                    maybe_target_recipe,
                );
                // Stack has been merged, no need to check other stacks.
                stack_merged = true;
                break;
            }
        }

        if !stack_merged {
            // Put stack back on the Z "floor"
            let mut new_transform = Transform::from(*dropped_global_transform);
            new_transform.translation.z = get_semi_random_stack_root_z(*dropped_stack_root);
            new_transform.scale = Vec3::ONE;

            // Re-enable physics for the dropped stack.
            commands
                .entity(*dropped_stack_root)
                .insert(StackPhysics)
                .insert(new_transform);
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

pub fn stack_move_to_target_system(
    mut commands: Commands,
    mut stacks_with_target: Query<(
        Entity,
        &GlobalTransform,
        &mut Transform,
        &CardStack,
        &MovingStackTarget,
        Option<&OngoingRecipe>,
    )>,
    all_stacks: Query<(Entity, &GlobalTransform, &CardStack, Option<&OngoingRecipe>)>,
    time: Res<Time>,
) {
    for (
        root,
        global_transform,
        mut transform,
        stack,
        &MovingStackTarget(movement_target),
        maybe_recipe,
    ) in stacks_with_target.iter_mut()
    {
        // TODO (Wybe 2022-05-25): Remove targeting when a stack is targeting itself.

        if let Ok((target_root, target_global_transform, target_stack, maybe_target_recipe)) =
            all_stacks.get(movement_target)
        {
            // TODO (Wybe 2022-05-25): Set the stacks Z position so it is on top of all other stacks. but below the dragged stacks
            let target_pos =
                global_center_of_top_card(target_global_transform, target_stack.len()).translation;

            let total_movement = target_pos.truncate() - global_transform.translation.truncate();

            let movement_this_frame =
                total_movement.normalize() * STACK_AUTO_MOVEMENT_SPEED * time.delta_seconds();

            if total_movement.length() == 0.
                || movement_this_frame.length() >= total_movement.length()
            {
                // Target will be reached in this frame. Snap to it.
                // Don't need to remove the movement target, because the source stack won't exist
                // after this frame.
                merge_stacks(
                    &mut commands,
                    root,
                    stack,
                    maybe_recipe,
                    target_root,
                    target_stack,
                    maybe_target_recipe,
                );
            } else {
                transform.translation += movement_this_frame.extend(0.);
                transform.translation.z = STACK_AUTO_MOVE_Z;
            }
        } else {
            // Target does not exist.
            remove_movement_target(&mut commands, root);
        }
    }
}

fn remove_movement_target(commands: &mut Commands, stack_root: Entity) {
    commands
        .entity(stack_root)
        .remove::<MovingStackTarget>()
        .insert(StackPhysics);
}

/// Handles stacks marked with [StackLookingForTargetLocation] (and removes the mark).
/// Finds either an open space, or another stack that this one can combine with.
/// Wont auto-combine with ongoing recipes.
pub fn find_stack_movement_target_system(
    mut commands: Commands,
    lost_stack_query: Query<
        (Entity, &GlobalTransform, &CardStack),
        With<StackLookingForMovementTarget>,
    >,
    potential_target_stack_query: Query<
        (Entity, &GlobalTransform, &CardStack),
        (
            Without<StackLookingForMovementTarget>,
            Without<StackRelativeDragPosition>,
            Without<OngoingRecipe>,
        ),
    >,
    cards: Query<&Card>,
    card_visual_size: Res<CardVisualSize>,
) {
    let card_cross_sections_max_search_radius = 1.5;
    let search_radius_range = card_visual_size.length() * card_cross_sections_max_search_radius;

    for (root, global_transform, stack) in lost_stack_query.iter() {
        // Stacks want to auto-stack if the top card on the target stack is of the same type.
        // TODO (Wybe 2022-05-25): Prevent recipes from automatically forming.
        // TODO (Wybe 2022-05-25): don't unwrap here.
        let wanted_top_card = cards.get(stack[0]).unwrap();
        if stack.iter().map(|&e| cards.get(e)).any(|maybe_card| {
            if let Ok(card) = maybe_card {
                card != wanted_top_card
            } else {
                false
            }
        }) {
            // One of the cards has a different category.
            // Therefore we can't auto stack.
            commands
                .entity(root)
                .remove::<StackLookingForMovementTarget>()
                .insert(StackPhysics);
            break;
        }

        let mut target_found = false;

        // TODO (Wybe 2022-05-25): Clean up so it isn't so nested.
        for (target_root, target_global, target_stack) in potential_target_stack_query.iter() {
            let top_card_transform = global_center_of_top_card(target_global, target_stack.len());

            if (global_transform.translation.truncate() - top_card_transform.translation.truncate())
                .length()
                < search_radius_range
            {
                // Top card in range. Check if it is the same as the cards in the seeking stack.
                if cards.get(*target_stack.last().unwrap()).unwrap() == wanted_top_card {
                    // Can auto-stack with this target stack.
                    commands
                        .entity(root)
                        .remove::<StackLookingForMovementTarget>()
                        .insert(MovingStackTarget(target_root));
                    target_found = true;
                    break;
                }
            }
        }

        if !target_found {
            commands
                .entity(root)
                .remove::<StackLookingForMovementTarget>()
                .insert(StackPhysics);
        }
    }

    // TODO (Wybe 2022-05-25): Implement moving (teleporting for now) to the closest empty space
    // TODO (Wybe 2022-05-25): Implement smoothly moving to the target (needs another system which handles the movement).
    // TODO (Wybe 2022-05-25): Implement what happens when cards get picked up by the user during this movement.
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
    use crate::{stack::get_movement_to_no_longer_overlap, Vec2};

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
