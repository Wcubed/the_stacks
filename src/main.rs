use bevy::prelude::App;
use bevy::DefaultPlugins;
use the_stacks::TheStacksPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(TheStacksPlugin)
        .run();
}
