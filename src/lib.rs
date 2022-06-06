#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]
// Allow elided lifetimes for now. Because it marks bevy's `Command` and `Query` everywhere.
// Which makes system arguments even more verbose than they already are.
// Once the elided lifetimes are no longer allowed, bevy will probably have a solution for it.
#![allow(elided_lifetimes_in_paths)]

mod camera;
mod card_packs;
mod card_types;
pub mod procedural;
mod recipe;
mod stack;
mod ui;

use crate::camera::*;
use crate::card_packs::CardPackPlugin;
use crate::procedural::ProceduralPlugin;
use crate::recipe::RecipePlugin;
use crate::stack::StackPlugin;
use crate::ui::UiPlugin;
use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;

pub struct TheStacksPlugin;

impl Plugin for TheStacksPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa { samples: 4 })
            .insert_resource(ClearColor(Color::rgb(0.1, 0.2, 0.1)))
            .insert_resource(TimeSpeed {
                running: true,
                speed: Speed::NORMAL,
            })
            .insert_resource(TimeOfDay {
                time_of_day: 0.,
                day: 1,
            })
            .insert_resource(LengthOfDay(100.))
            .add_state(GameState::AssetLoading)
            .add_stage_after(
                CoreStage::Update,
                UpdateStage::SystemsThatDeleteCards.as_str(),
                SystemStage::parallel(),
            )
            .add_plugin(ProceduralPlugin)
            .add_plugin(StackPlugin)
            .add_plugin(CardPackPlugin)
            .add_plugin(RecipePlugin)
            .add_plugin(OrthographicCameraPlugin)
            .add_plugin(UiPlugin)
            .add_system_set(
                SystemSet::on_update(GameState::Run)
                    .with_system(game_speed_change_system)
                    .with_system(time_of_day_progress_system),
            );
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    AssetLoading,
    Run,
}

enum UpdateStage {
    /// All systems that are allowed to delete stacks / cards should go in here.
    /// This to prevent "cannot add component to entity because it has already been deleted" errors.
    SystemsThatDeleteCards,
}

impl UpdateStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            UpdateStage::SystemsThatDeleteCards => "SystemsThatDeleteCards",
        }
    }
}

/// Resource that keeps track of which day it is, and how far along the day we are.
pub struct TimeOfDay {
    day: u32,
    /// 0 to 1
    time_of_day: f32,
}

/// Resource that holds the configured length of a single day, in in-game seconds.
pub struct LengthOfDay(f32);

/// There are separate `running` and `speed` fields so that the game can remember the desired speed
/// while paused.
/// Influences things like the progress of crafting recipes.
pub struct TimeSpeed {
    running: bool,
    speed: Speed,
}

impl TimeSpeed {
    /// Does not take into account being paused.
    fn as_factor(&self) -> f32 {
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

fn time_of_day_progress_system(
    mut days: ResMut<TimeOfDay>,
    speed: ResMut<TimeSpeed>,
    length_of_day: Res<LengthOfDay>,
    time: Res<Time>,
) {
    days.time_of_day += (time.delta_seconds() * speed.as_factor()) / length_of_day.0;
    if days.time_of_day >= 1.0 {
        days.time_of_day -= 1.0;
        days.day += 1;
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
