mod camera;
mod card;

use crate::camera::{camera_drag_system, camera_zoom_system};
use crate::card::{
    card_mouse_drag_system, card_overlap_nudging_system, spawn_card, CardImages, CardVisualSize,
};
use bevy::prelude::*;
use bevy_asset_loader::AssetLoader;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    AssetLoading,
    Run,
}

pub struct TheStacksPlugin;

impl Plugin for TheStacksPlugin {
    fn build(&self, app: &mut App) {
        AssetLoader::new(GameState::AssetLoading)
            .continue_to_state(GameState::Run)
            .with_collection::<CardImages>()
            .build(app);

        app.insert_resource(Msaa { samples: 4 })
            .add_state(GameState::AssetLoading)
            .add_system_set(
                SystemSet::on_exit(GameState::AssetLoading).with_system(card::on_assets_loaded),
            )
            .add_system_set(SystemSet::on_enter(GameState::Run).with_system(world_setup))
            .add_system_set(
                SystemSet::on_update(GameState::Run)
                    .with_system(camera_zoom_system)
                    .with_system(camera_drag_system)
                    .with_system(card_mouse_drag_system)
                    .with_system(card_overlap_nudging_system),
            );
    }
}

fn world_setup(mut commands: Commands, card_images: Res<CardImages>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    for _ in 0..10 {
        spawn_card(&mut commands, &card_images);
    }
}
