use bevy::math::const_vec2;
use bevy::prelude::*;

const CARD_SIZE: Vec2 = const_vec2!([100.0, 130.0]);

pub struct TheStacksPlugin;

impl Plugin for TheStacksPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
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
                custom_size: Some(CARD_SIZE),
                ..default()
            },
            ..default()
        })
        .insert(Card);
}
