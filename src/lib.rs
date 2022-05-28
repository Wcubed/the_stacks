mod camera;
mod card;
mod card_types;
mod recipe;
mod recipe_defines;
mod stack_utils;
mod ui;

use crate::camera::OrthographicCameraPlugin;
use crate::card::CardPlugin;
use crate::recipe::RecipePlugin;
use crate::ui::UiPlugin;
use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    AssetLoading,
    Run,
}

pub struct TheStacksPlugin;

impl Plugin for TheStacksPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa { samples: 4 })
            .insert_resource(ClearColor(Color::rgb(0.0, 0.1, 0.0)))
            .insert_resource(GameSpeed {
                running: true,
                speed: 1.0,
            })
            .add_state(GameState::AssetLoading)
            .add_plugin(CardPlugin)
            .add_plugin(RecipePlugin)
            .add_plugin(OrthographicCameraPlugin)
            .add_plugin(UiPlugin)
            .add_system_set(
                SystemSet::on_update(GameState::Run).with_system(game_speed_change_system),
            );
    }
}

/// There are separate `running` and `speed` fields so that the game can remember the desired speed
/// while paused.
pub struct GameSpeed {
    running: bool,
    speed: f32,
}

fn game_speed_change_system(keys: Res<Input<KeyCode>>, mut speed: ResMut<GameSpeed>) {
    if keys.just_pressed(KeyCode::Space) {
        speed.running = !speed.running;
    }
}

/// Conditional that can be used in [SystemSet::with_run_criteria](bevy::prelude::SystemSet::with_run_criteria) statements.
pub fn is_game_running(speed: Res<GameSpeed>) -> ShouldRun {
    if speed.running {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}
