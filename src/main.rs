mod executors;
mod gym3d;
mod panels;
mod types;

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::window::{Window, WindowMode};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use std::fs;
use std::path::PathBuf;

use executors::{
    discrete::execute_script,
    streaming::{update_streams, StreamManager},
};
use gym3d::{
    camera::{orbit_camera, setup_isometric_camera},
    scene::{
        initialize_scene_with_camera, update_cube_position, update_infinite_plane,
        InfiniteGridMaterial,
    },
};
use panels::side_panels::{show_left_panel, show_right_panel};

use types::*;

const DEFAULT_CONFIG_PATH: &str = "test_apps/4_flight_replay/config.toml";

fn main() {
    // Get the current executable's directory
    let exe_dir = std::env::current_exe()
        .map(|path| {
            path.parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf()
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Try multiple possible locations for the config file
    let possible_paths = [
        std::path::PathBuf::from(DEFAULT_CONFIG_PATH), // Try relative to working directory
        exe_dir.join(DEFAULT_CONFIG_PATH),             // Try relative to executable
    ];

    let (content, config_path) = possible_paths
        .iter()
        .find_map(|path| {
            fs::read_to_string(path)
                .ok()
                .map(|content| (content, path.clone()))
        })
        .unwrap_or_else(|| {
            (
                String::default(),
                std::path::PathBuf::from(DEFAULT_CONFIG_PATH),
            )
        });

    let config: Config = toml::from_str(&content).unwrap_or_default();

    // Initialize Bevy app
    let mut app = bevy::prelude::App::new();

    // Configure default plugins based on whether 3D scene is enabled
    if config.layout.show_3d_scene {
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: config.layout.title.clone().unwrap_or("Connect".to_string()),
                        mode: WindowMode::Windowed,
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    filter: "connect=info,wgpu=error".to_string(),
                    level: bevy::log::Level::INFO,
                    ..default()
                }),
        )
        .add_systems(Startup, initialize_scene_with_camera)
        .add_systems(
            Update,
            (orbit_camera, setup_isometric_camera, update_infinite_plane).in_set(AppSet::Main),
        );
    } else {
        // Use a minimal set of plugins when 3D scene is disabled
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<bevy::render::RenderPlugin>()
                .set(LogPlugin {
                    filter: "connect=info,wgpu=error".to_string(),
                    level: bevy::log::Level::INFO,
                    ..default()
                }),
        );
    }

    app.add_plugins(EguiPlugin)
        .insert_resource(StreamManager::new(config.debug.streaming, &config))
        .insert_resource(ScriptOutputs::default())
        .insert_resource(AppState {
            opened_file: Some(config_path),
            ..default()
        })
        .insert_resource(UiState {
            left_selected_tab: config
                .layout
                .left_panel
                .tabs
                .first()
                .map(|tab| tab.id.clone())
                .unwrap_or_default(),
            right_selected_tab: config
                .layout
                .right_panel
                .tabs
                .first()
                .map(|tab| tab.id.clone())
                .unwrap_or_default(),
        })
        .insert_resource(MarkdownCache::default())
        .add_plugins(MaterialPlugin::<InfiniteGridMaterial>::default())
        .configure_sets(Update, AppSet::Main);

    app.add_systems(
        Update,
        (egui_system, update_streams, update_cube_position).in_set(AppSet::Main),
    )
    .run();
}

// First, let's add a helper function to check for streaming scripts
fn has_streaming_scripts(scripts: &[ScriptConfig]) -> bool {
    scripts
        .iter()
        .any(|script| script.script_type == "streaming")
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
    mut grid_materials: ResMut<Assets<InfiniteGridMaterial>>,
) {
    // Create a longer-lived binding for the default path
    let default_path = PathBuf::from("config.toml");
    let config_path = app_state.opened_file.as_ref().unwrap_or(&default_path);
    let content = fs::read_to_string(config_path).unwrap_or_default();
    let config: Config = toml::from_str(&content).unwrap_or_default();

    // Get the window width
    let window_width = windows.single().width();

    // Show help text only if 3D scene is enabled
    if config.layout.show_3d_scene {
        let has_left_panel = !config.layout.left_panel.tabs.is_empty();
        let has_right_panel = !config.layout.right_panel.tabs.is_empty();

        // Calculate x position based on panels
        let x_pos = if has_left_panel && has_right_panel {
            // Center between panels
            window_width / 2.0
        } else {
            // Default left position
            10.0
        };

        egui::Area::new("help_text".into())
            .fixed_pos(egui::pos2(x_pos, 40.0))
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
            &mut grid_materials,
        );
    });

    // Show both panels
    show_left_panel(
        &mut ui_state,
        &mut app_state,
        &config,
        window_width,
        &stream_manager,
        &mut markdown_cache,
        contexts.ctx_mut(),
    );

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
