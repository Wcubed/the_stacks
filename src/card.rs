use crate::GameState;
use bevy::math::{const_vec2, const_vec3};
use bevy::prelude::*;
use bevy::render::camera::Camera2d;
use bevy_asset_loader::{AssetCollection, AssetLoader};

const CARD_Z: f32 = 1.0;
const CARD_DRAG_Z: f32 = 2.0;
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
            .add_system_set(
                SystemSet::on_exit(GameState::AssetLoading).with_system(on_assets_loaded),
            )
            .add_system_set(SystemSet::on_enter(GameState::Run).with_system(spawn_test_cards))
            .add_system_set(
                SystemSet::on_update(GameState::Run)
                    .with_system(card_mouse_drag_system)
                    .with_system(card_hover_system)
                    .with_system(card_overlap_nudging_system)
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

#[derive(Component)]
pub struct Card;

#[derive(Component)]
/// Indicates this is the topmost card of a stack of cards.
/// All individual cards are stack roots.
pub struct CardStackRoot;

#[derive(Component)]
pub struct NextCardInStack(Entity);

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
    commands
        .spawn_bundle(SpriteBundle {
            texture: card_images.background.clone(),
            sprite: Sprite {
                color: CARD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Card)
        .insert(CardStackRoot)
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
        });
}

pub fn card_mouse_drag_system(
    mut commands: Commands,
    mouse_button: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    hovered_but_not_dragged_card_query: Query<
        (Entity, &HoveredCard),
        (With<Card>, Without<CardRelativeDragPosition>),
    >,
    mut dragged_card_query: Query<
        (
            Entity,
            &mut Transform,
            &GlobalTransform,
            &CardRelativeDragPosition,
        ),
        With<Card>,
    >,
    mut card_dropped_writer: EventWriter<CardDroppedEvent>,
) {
    let primary_window = windows.get_primary().expect("No primary window!");
    let (camera, camera_transform) = camera_query.single();

    if let Some(mouse_window_pos) = primary_window.cursor_position() {
        let mouse_world_pos =
            window_pos_to_world_pos(camera, camera_transform, primary_window, mouse_window_pos);

        // Check for newly dragged cards.
        for (entity, hovered_card) in hovered_but_not_dragged_card_query.iter() {
            if mouse_button.just_pressed(MouseButton::Left) {
                commands
                    .entity(entity)
                    .insert(CardRelativeDragPosition(hovered_card.relative_hover_pos));
            }
        }

        // In principle there should ever only be 1 hovered card.
        // But the system can work with multiple if need be.
        for (entity, mut transform, global_transform, drag_position) in
            dragged_card_query.iter_mut()
        {
            if mouse_button.just_released(MouseButton::Left) {
                // Drop the cards.

                commands.entity(entity).remove::<CardRelativeDragPosition>();
                transform.translation.z = CARD_Z;
                transform.scale = Vec3::ONE;

                card_dropped_writer.send(CardDroppedEvent(entity, *global_transform));
            } else {
                // Drag the cards.

                transform.translation = (mouse_world_pos - drag_position.0).extend(CARD_DRAG_Z);
                transform.scale = CARD_DRAG_SCALE;
            }
        }
    }
}

pub fn card_hover_system(
    mut commands: Commands,
    windows: Res<Windows>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut card_query: Query<(Entity, &GlobalTransform, &mut Sprite), With<Card>>,
    card_visual_size: Res<CardVisualSize>,
) {
    let primary_window = windows.get_primary().expect("No primary window!");
    let (camera, camera_transform) = camera_query.single();

    if let Some(mouse_window_pos) = primary_window.cursor_position() {
        let mouse_world_pos =
            window_pos_to_world_pos(camera, camera_transform, primary_window, mouse_window_pos);

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
    targetable_card_query: Query<(Entity, &Card, &GlobalTransform), Without<NextCardInStack>>,
    card_visual_size: Res<CardVisualSize>,
    mut card_dropped_reader: EventReader<CardDroppedEvent>,
) {
    for CardDroppedEvent(dropped_entity, dropped_global_transform) in card_dropped_reader.iter() {
        let mut closest_drop_target = None;

        // Find which card we are overlapping the most.
        // TODO (Wybe 2022-05-14): This should also check if the card we are overlapping is
        //   a valid target to stack with.
        for (entity, card, global_transform) in targetable_card_query.iter() {
            if entity == *dropped_entity {
                // Cannot drop onto self.
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
                if let Some((_, shortest_distance)) = closest_drop_target {
                    if distance < shortest_distance {
                        closest_drop_target = Some((entity, distance));
                    }
                } else {
                    closest_drop_target = Some((entity, distance));
                }
            }
        }

        // If we have a target. We need to add this card on top of it.
        // TODO (Wybe 2022-05-14): Should also handle stacking multiple stacks.
        //    maybe we should chance the concept of what the user is dragging to stacks,
        //    instead of referencing individual cards.
        if let Some((drop_target, _)) = closest_drop_target {
            info!(
                "Dropped card {:?} is stacking with {:?}",
                dropped_entity, drop_target
            );

            add_card_to_stack(&mut commands, *dropped_entity, drop_target);
        }
    }
}

/// Adds a card to the stack of which the `bottom_entity` is the bottom card.
/// Effects are applied via Commands, which means it is visible next update.
pub fn add_card_to_stack(
    commands: &mut Commands,
    card_entity: Entity,
    bottom_card_of_stack: Entity,
) {
    // Put this card in front of the parent.
    let new_transform = Transform::from_xyz(0., -CARD_STACK_Y_SPACING, DELTA_Z);

    commands
        .entity(bottom_card_of_stack)
        .add_child(card_entity)
        .insert(NextCardInStack(card_entity));
    commands
        .entity(card_entity)
        .remove::<CardStackRoot>()
        .insert(new_transform);
}

/// Slowly nudges cards that are not dragged, until they don't overlap.
pub fn card_overlap_nudging_system(
    time: Res<Time>,
    mut undragged_stacks: Query<
        (&GlobalTransform, &mut Transform),
        (
            With<Card>,
            With<CardStackRoot>,
            Without<CardRelativeDragPosition>,
        ),
    >,
    card_visual_size: Res<CardVisualSize>,
) {
    let mut combinations = undragged_stacks.iter_combinations_mut();
    while let Some([(global_transform1, mut transform1), (global_transform2, mut transform2)]) =
        combinations.fetch_next()
    {
        let card_wanted_space = card_visual_size.0 + CARD_OVERLAP_SPACING;

        // TODO (Wybe 2022-05-14): Should we account for scaling and rotation?
        if let Some(total_movement) = get_movement_to_no_longer_overlap(
            global_transform1.translation.truncate(),
            card_wanted_space,
            global_transform2.translation.truncate(),
            card_wanted_space,
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
