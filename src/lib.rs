use bevy::math::const_vec2;
use bevy::prelude::*;

const CARD_SIZE: Vec2 = const_vec2!([100.0, 130.0]);

pub struct TheStacksPlugin;

impl Plugin for TheStacksPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(card_mouse_drag_system);
    }
}

#[derive(Component)]
pub struct Card;

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    spawn_card(&mut commands);
    spawn_card(&mut commands);
}

fn spawn_card(commands: &mut Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                ..default()
            },
            transform: Transform::from_scale(CARD_SIZE.extend(1.0)),
            ..default()
        })
        .insert(Card);
}

fn card_mouse_drag_system(
    mouse_button: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    card_query: Query<&GlobalTransform, With<Card>>,
) {
    let primary_window = windows.get_primary().expect("No primary window!");
    let (camera, camera_transform) = camera_query.single();

    if let Some(mouse_window_pos) = primary_window.cursor_position() {
        let mouse_world_pos =
            window_pos_to_world_pos(camera, camera_transform, primary_window, mouse_window_pos);

        if mouse_button.just_pressed(MouseButton::Left) {
            info!("{}", mouse_world_pos);
            for transform in card_query.iter() {
                // TODO (Wybe 2022-05-14): Transform mouse position to local coordinates.
                // Assumes sprite size is 1x1.
                if in_bounds(transform, &mouse_world_pos) {
                    info!("In bounds!");
                }
            }
        }
        if mouse_button.just_released(MouseButton::Left) {
            info!("Left mouse button released");
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

fn in_bounds(transform: &GlobalTransform, position: &Vec2) -> bool {
    // TODO (Wybe 2022-05-14): Take into account rotation.
    let half_size = transform.scale / 2.0;
    let translation = transform.translation;

    position.x >= translation.x - half_size.x
        && position.x <= translation.x + half_size.x
        && position.y >= translation.y - half_size.y
        && position.y <= translation.y + half_size.y
}
