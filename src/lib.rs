mod camera;
mod card;
mod card_types;

use crate::camera::OrthographicCameraPlugin;
use crate::card::CardPlugin;
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
            .insert_resource(ClearColor(Color::BLACK))
            .add_state(GameState::AssetLoading)
            .add_plugin(CardPlugin)
            .add_plugin(OrthographicCameraPlugin);
    }
}
