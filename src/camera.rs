use crate::stack::StackRelativeDragPosition;
use crate::GameState;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::camera::Camera2d;

/// Mouse wheels are less precise than touchpads, so we scale the zoom when using a scroll wheel.
const MOUSE_WHEEL_ZOOM_FACTOR: f32 = 0.1;
const MAX_ZOOMED_OUT_SCALE: f32 = 10.0;
const MAX_ZOOMED_IN_SCALE: f32 = 2.0;

const START_ZOOM: f32 = 2.0;

pub struct OrthographicCameraPlugin;

impl Plugin for OrthographicCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(GameState::Run).with_system(camera_setup))
            .add_system_set(
                SystemSet::on_update(GameState::Run)
                    .with_system(camera_zoom_system)
                    .with_system(camera_drag_system),
            );
    }
}

pub fn camera_setup(mut commands: Commands) {
    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.transform.scale = Vec3::splat(START_ZOOM);
    commands.spawn_bundle(camera_bundle);
}

pub fn camera_zoom_system(
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    mut mouse_wheel: EventReader<MouseWheel>,
) {
    let mut camera = camera_query.single_mut();
    for event in mouse_wheel.iter() {
        let scroll_amount = match event.unit {
            MouseScrollUnit::Line => event.y * MOUSE_WHEEL_ZOOM_FACTOR,
            MouseScrollUnit::Pixel => event.y,
        } * camera.scale.x;

        camera.scale.x -= scroll_amount;
        camera.scale.x = camera
            .scale
            .x
            .clamp(MAX_ZOOMED_IN_SCALE, MAX_ZOOMED_OUT_SCALE);

        // Zoom is always equal on x an y axis.
        camera.scale.y = camera.scale.x;
    }
}

pub fn camera_drag_system(
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<Camera2d>>,
    windows: Res<Windows>,
    mouse_button: Res<Input<MouseButton>>,
    mut last_pos: Local<Option<Vec2>>,
    dragged_card_query: Query<&StackRelativeDragPosition>,
) {
    if !dragged_card_query.is_empty() {
        // The user is dragging cards, so we shouldn't be dragging the camera, otherwise that
        // might mess up the dragging.
        return;
    }
    let window = windows.get_primary().expect("No primary window!");
    let current_pos = match window.cursor_position() {
        Some(current_pos) => current_pos,
        None => return,
    };
    let delta = current_pos - last_pos.unwrap_or(current_pos);

    if mouse_button.pressed(MouseButton::Left) || mouse_button.pressed(MouseButton::Right) {
        let (mut camera_transform, projection) = camera_query.single_mut();

        let scaling = Vec2::new(
            window.width() / (projection.right - projection.left),
            window.height() / (projection.top - projection.bottom),
        ) * projection.scale
            * camera_transform.scale.truncate();

        camera_transform.translation -= (delta * scaling).extend(0.);
    }

    *last_pos = Some(current_pos);
}
