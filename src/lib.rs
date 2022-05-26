mod camera;
mod card;
mod card_types;
mod recipe;
mod stack_utils;

use crate::camera::OrthographicCameraPlugin;
use crate::card::CardPlugin;
use crate::recipe::RecipePlugin;
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
            .add_state(GameState::AssetLoading)
            .add_plugin(CardPlugin)
            .add_plugin(RecipePlugin)
            .add_plugin(OrthographicCameraPlugin);
    }
}
