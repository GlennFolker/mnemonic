pub mod content;
pub mod editor;
pub mod map;
pub mod obj;

use avian3d::prelude::*;
use bevy::{prelude::*, window::PresentMode};
use bevy_asset_loader::prelude::*;
use bevy_mod_picking::prelude::*;
use content::{TileTexture, Tiles};
use editor::EditorPlugin;
use iyes_progress::prelude::*;
use map::MapPlugin;
use obj::ObjPlugin;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    Loading,
    Menu,
    Editor,
}

#[inline]
pub fn run() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()).set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                title: "Mnemonic".into(),
                ..default()
            }),
            ..default()
        }),
        PhysicsPlugins::default().with_length_unit(2.0),
        #[cfg(feature = "dev")]
        PhysicsDebugPlugin::default(),
        DefaultPickingPlugins,
        MapPlugin,
        ObjPlugin,
        EditorPlugin,
    ))
    .init_state::<GameState>()
    .add_plugins(ProgressPlugin::new(GameState::Loading).continue_to(GameState::Editor))
    .add_loading_state(
        LoadingState::new(GameState::Loading)
            .load_collection::<Tiles>()
            .init_resource::<TileTexture>(),
    );

    #[cfg(feature = "dev")]
    {
        fn toggle_debug(mut mode: ResMut<DebugPickingMode>) {
            *mode = match *mode {
                DebugPickingMode::Disabled => DebugPickingMode::Noisy,
                _ => DebugPickingMode::Disabled,
            }
        }

        app.insert_resource(DebugPickingMode::Disabled).add_systems(
            PreUpdate,
            toggle_debug.run_if(bevy::input::common_conditions::input_just_pressed(KeyCode::F5)),
        );
    }

    app.run();
}
