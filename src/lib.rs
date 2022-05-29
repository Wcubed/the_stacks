#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]

mod camera;
mod card_packs;
mod card_types;
mod recipe;
mod recipe_defines;
mod stack;
mod stack_utils;
mod ui;

use crate::camera::OrthographicCameraPlugin;
use crate::card_packs::CardPackPlugin;
use crate::recipe::RecipePlugin;
use crate::stack::StackPlugin;
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
            .insert_resource(ClearColor(Color::rgb(0.1, 0.2, 0.1)))
            .insert_resource(TimeSpeed {
                running: true,
                speed: Speed::NORMAL,
            })
            .add_state(GameState::AssetLoading)
            .add_plugin(StackPlugin)
            .add_plugin(CardPackPlugin)
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
/// Influences things like the progress of crafting recipes.
pub struct TimeSpeed {
    running: bool,
    speed: Speed,
}

impl TimeSpeed {
    /// Does not take into account being paused.
    fn speed_as_factor(&self) -> f32 {
        match self.speed {
            Speed::NORMAL => 1.0,
            Speed::DOUBLE => 2.0,
            Speed::TRIPLE => 3.0,
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum Speed {
    NORMAL,
    DOUBLE,
    TRIPLE,
}

fn game_speed_change_system(keys: Res<Input<KeyCode>>, mut speed: ResMut<TimeSpeed>) {
    if keys.just_pressed(KeyCode::Space) {
        speed.running = !speed.running;
    }
    if keys.just_pressed(KeyCode::Key1) {
        speed.running = true;
        speed.speed = Speed::NORMAL;
    }
    if keys.just_pressed(KeyCode::Key2) {
        speed.running = true;
        speed.speed = Speed::DOUBLE;
    }
    if keys.just_pressed(KeyCode::Key3) {
        speed.running = true;
        speed.speed = Speed::TRIPLE;
    }
}

/// Conditional that can be used in [SystemSet::with_run_criteria](bevy::prelude::SystemSet::with_run_criteria) statements.
pub fn is_time_running(speed: Res<TimeSpeed>) -> ShouldRun {
    if speed.running {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}
