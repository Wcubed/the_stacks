use bevy::prelude::*;

const CARD_Z: f32 = 1.0;
const CARD_DRAG_Z: f32 = 2.0;

const CARD_COLOR: Color = Color::rgb(0.25, 0.25, 0.75);
/// TODO (Wybe 2022-05-14): Convert this into an overlay somehow, instead of changing the card sprite color.
const CARD_DRAG_COLOR: Color = Color::rgb(0.30, 0.30, 0.80);
const CARD_HOVER_COLOR: Color = Color::rgb(0.35, 0.35, 0.85);
const CARD_BORDER_COLOR: Color = Color::BLACK;

const ASSET_LOAD_STAGE: &str = "asset_load";
const WORLD_SETUP_STAGE: &str = "world_setup";

pub struct TheStacksPlugin;

impl Plugin for TheStacksPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa { samples: 4 })
            .add_startup_stage_after(
                StartupStage::Startup,
                ASSET_LOAD_STAGE,
                SystemStage::parallel(),
            )
            .add_startup_system_to_stage(ASSET_LOAD_STAGE, load_assets)
            .add_startup_stage_after(ASSET_LOAD_STAGE, WORLD_SETUP_STAGE, SystemStage::parallel())
            .add_startup_system_to_stage(WORLD_SETUP_STAGE, world_setup)
            .add_system(card_mouse_drag_system);
    }
}

#[derive(Component, Default)]
pub struct Card {
    relative_drag_position: Option<Vec2>,
}

pub struct CardImages {
    background: Handle<Image>,
    border: Handle<Image>,
}

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Enable hot reloading.
    asset_server.watch_for_changes().unwrap();

    let background = asset_server.load("vector_images/card_background.png");
    let border = asset_server.load("vector_images/card_border.png");

    commands.insert_resource(CardImages { background, border });
    info!("Here!")
}

fn world_setup(mut commands: Commands, card_images: Res<CardImages>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    for _ in 0..10 {
        spawn_card(&mut commands, &card_images);
    }
}

fn spawn_card(commands: &mut Commands, card_images: &Res<CardImages>) {
    commands
        .spawn_bundle(SpriteBundle {
            texture: card_images.background.clone(),
            sprite: Sprite {
                color: CARD_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Card::default())
        .with_children(|parent| {
            parent.spawn_bundle(SpriteBundle {
                texture: card_images.border.clone(),
                transform: Transform::from_xyz(0.0, 0.0, 1.0),
                sprite: Sprite {
                    color: CARD_BORDER_COLOR,
                    ..default()
                },
                ..default()
            });
        });
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

        for (mut transform, mut sprite, mut card) in card_query.iter_mut() {
            // Assumes sprite size is 1x1, and that the transform.scale provides the actual size.
            if let Some(pos) = in_bounds(&transform, mouse_world_pos) {
                if mouse_button.just_pressed(MouseButton::Left) {
                    card.relative_drag_position = Some(pos);
                    sprite.color = CARD_DRAG_COLOR;
                    // Can only drag one card at a time.
                    // TODO (Wybe 2022-05-14): Make this not break out of a loop that does more stuff.
                    break;
                } else if !mouse_button.pressed(MouseButton::Left) {
                    sprite.color = CARD_HOVER_COLOR;
                }
            } else if card.relative_drag_position.is_none() {
                sprite.color = CARD_COLOR;
            }

            if mouse_button.just_released(MouseButton::Left) {
                card.relative_drag_position = None;
                transform.translation.z = CARD_Z;
            }

            if let Some(pos) = card.relative_drag_position {
                transform.translation = (mouse_world_pos - pos).extend(CARD_DRAG_Z);
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
