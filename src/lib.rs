use bevy::math::const_vec2;
use bevy::prelude::*;

const CARD_SIZE: Vec2 = const_vec2!([100.0, 130.0]);
const CARD_COLOR: Color = Color::rgb(0.25, 0.25, 0.75);
/// TODO (Wybe 2022-05-14): Convert this into an overlay somehow, instead of changing the card sprite color.
const CARD_DRAG_COLOR: Color = Color::rgb(0.30, 0.30, 0.80);

pub struct TheStacksPlugin;

impl Plugin for TheStacksPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(card_mouse_drag_system);
    }
}

#[derive(Component, Default)]
pub struct Card {
    relative_drag_position: Option<Vec2>,
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    for _ in 0..10 {
        spawn_card(&mut commands);
    }
}

fn spawn_card(commands: &mut Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: CARD_COLOR,
                ..default()
            },
            transform: Transform::from_scale(CARD_SIZE.extend(1.0)),
            ..default()
        })
        .insert(Card::default());
}

fn card_mouse_drag_system(
    mouse_button: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    camera_query: Query<(&Camera, &GlobalTransform), Without<Card>>,
    mut card_query: Query<(&mut GlobalTransform, &mut Sprite, &mut Card)>,
) {
    let primary_window = windows.get_primary().expect("No primary window!");
    let (camera, camera_transform) = camera_query.single();

    if let Some(mouse_window_pos) = primary_window.cursor_position() {
        let mouse_world_pos =
            window_pos_to_world_pos(camera, camera_transform, primary_window, mouse_window_pos);

        if mouse_button.just_pressed(MouseButton::Left) {
            for (transform, mut sprite, mut card) in card_query.iter_mut() {
                // Assumes sprite size is 1x1, and that the transform.scale provides the actual size.
                if let Some(pos) = in_bounds(&transform, mouse_world_pos) {
                    card.relative_drag_position = Some(pos);
                    sprite.color = CARD_DRAG_COLOR;
                    // Can only drag one card at a time.
                    break;
                }
            }
        }
        if mouse_button.just_released(MouseButton::Left) {
            for (_, mut sprite, mut card) in card_query.iter_mut() {
                card.relative_drag_position = None;
                sprite.color = CARD_COLOR;
            }
        }

        for (mut transform, _, card) in card_query.iter_mut() {
            if let Some(pos) = card.relative_drag_position {
                transform.translation = (mouse_world_pos - pos).extend(1.0);
            }
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
fn in_bounds(transform: &GlobalTransform, position: Vec2) -> Option<Vec2> {
    // TODO (Wybe 2022-05-14): Take into account rotation.
    let half_size = transform.scale.truncate() / 2.0;

    let pos_in_bounds = position - transform.translation.truncate();

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
