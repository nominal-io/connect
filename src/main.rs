mod executors;
mod gym3d;
mod types;
mod panels;

use executors::streaming::{StreamManager, update_streams};
use executors::discrete::execute_script;
use gym3d::camera::orbit_camera;
use gym3d::scene::initialize_scene_with_camera;
use gym3d::scene::InfiniteGridMaterial;
use gym3d::scene::update_infinite_plane;
use types::*;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use std::fs;
use bevy::window::{WindowMode, Window};
use std::path::PathBuf;
use crate::panels::side_panels::show_right_panel;
use crate::gym3d::camera::setup_isometric_camera;

fn main() {
    let default_config = PathBuf::from("test_apps/1_kitchen_sink/config.toml");
    let content = fs::read_to_string(&default_config).unwrap_or_default();
    let config: Config = toml::from_str(&content).unwrap_or_default();
    
    // Initialize Bevy app
    let mut app = bevy::prelude::App::new();

    // Get the initial tab from config
    let initial_tab = config.layout.right_panel.tabs
        .first()
        .map(|tab| tab.id.clone())
        .unwrap_or_default();    
    
    // Configure default plugins based on whether 3D scene is enabled
    if config.layout.show_3d_scene {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: config.layout.title.unwrap_or("Connect".to_string()).into(),
                mode: WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, initialize_scene_with_camera)
        .add_systems(Update, (
            orbit_camera,
            setup_isometric_camera,
            update_infinite_plane,
        ).in_set(AppSet::Main));
    } else {
        // Use a minimal set of plugins when 3D scene is disabled
        app.add_plugins(DefaultPlugins.build().disable::<bevy::render::RenderPlugin>());
    }

    app.add_plugins(EguiPlugin)
        .insert_resource(StreamManager::new(config.debug.streaming))
        .insert_resource(ScriptOutputs::default())
        .insert_resource(AppState {
            opened_file: Some(default_config),
            ..default()
        })
        .insert_resource(UiState {
            selected_tab: initial_tab,
        })
        .insert_resource(MarkdownCache::default())
        .add_plugins(MaterialPlugin::<InfiniteGridMaterial>::default())
        .configure_sets(Update, AppSet::Main);

    app.add_systems(Update, (
        egui_system,
        update_streams,
    ).in_set(AppSet::Main))
    .run();
}

// First, let's add a helper function to check for streaming scripts
fn has_streaming_scripts(scripts: &[ScriptConfig]) -> bool {
    scripts.iter().any(|script| script.script_type == "streaming")
}

fn egui_system(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut script_outputs: ResMut<ScriptOutputs>,
    mut stream_manager: ResMut<StreamManager>,
    mut app_state: ResMut<AppState>,
    mut ui_state: ResMut<UiState>,
    windows: Query<&Window>,
    mut markdown_cache: ResMut<MarkdownCache>,
    camera_query: Query<Entity, With<Camera3d>>,
    light_query: Query<Entity, With<PointLight>>,
    mesh_query: Query<Entity, With<Mesh3d>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create a longer-lived binding for the default path
    let default_path = PathBuf::from("config.toml");
    let config_path = app_state.opened_file.as_ref().unwrap_or(&default_path);
    let content = fs::read_to_string(config_path).unwrap_or_default();
    let config: Config = toml::from_str(&content).unwrap_or_default();
    
    // Show help text only if 3D scene is enabled
    if config.layout.show_3d_scene {
        egui::Area::new("help_text".into())
            .fixed_pos(egui::pos2(10.0, 40.0))
            .show(contexts.ctx_mut(), |ui| {
                ui.vertical(|ui| {
                    ui.label("Shift-click to pan");
                    ui.label("Cmd-click to drag");
                });
            });
    }

    // Get the window width
    let window_width = windows.single().width();

    // Simplified title panel
    if let Some(title) = &config.layout.title {
        egui::TopBottomPanel::top("title_bar").show(contexts.ctx_mut(), |ui| {
            ui.heading(title);
        });
    }

    // Python scripts bottom panel
    egui::TopBottomPanel::bottom("scripts_panel").show(contexts.ctx_mut(), |ui| {
        panels::scripts_panel::show_scripts_panel(
            ui,
            &mut commands,
            &mut app_state,
            &mut script_outputs,
            &mut stream_manager,
            &mut ui_state,
            &config,
            &camera_query,
            &light_query,
            &mesh_query,
            &asset_server,
            &mut meshes,
            &mut materials,
        );
    });

    // Replace the entire right panel implementation with:
    show_right_panel(
        &mut ui_state,
        &mut app_state,
        &config,
        window_width,
        &stream_manager,
        &mut markdown_cache,
        contexts.ctx_mut(),
    );
}



