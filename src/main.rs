mod streaming;
mod camera;
mod scene;

use streaming::{StreamManager, update_streams};
use camera::orbit_camera;
use scene::setup;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use serde_json;
use bevy::window::{WindowMode, Window};
use std::path::{PathBuf, Path};
use tinyfiledialogs::open_file_dialog;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::io::{BufRead, BufReader};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum AppSet {
    Main,
}

#[derive(Deserialize, Debug)]
struct FunctionConfig {
    name: String,      // Function name in the Python script
    display: String,   // Display name in the UI
}

#[derive(Deserialize, Debug)]
struct ScriptConfig {
    name: String,
    path: String,
    #[serde(rename = "type")]
    script_type: String,
    #[serde(default)]
    functions: Vec<FunctionConfig>,
}

#[derive(Deserialize, Debug, Default)]
struct InputFieldConfig {
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    tab: String,
}

#[derive(Deserialize, Debug, Default)]
struct PlotConfig {
    #[serde(default)]
    tab: String,
}

#[derive(Deserialize, Debug)]
struct SliderConfig {
    id: String,
    label: String,
    min: f32,
    max: f32,
    default: f32,
    tab: String,
}

impl Default for SliderConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            min: default_slider_min(),
            max: default_slider_max(),
            default: default_slider_value(),
            tab: String::new(),
        }
    }
}

// Add back the DebugConfig struct
#[derive(Deserialize, Debug, Default)]
struct DebugConfig {
    #[serde(default)]
    streaming: bool,
}

// The rest of the Config and LayoutConfig structs remain the same
#[derive(Deserialize, Debug, Default)]
struct Config {
    #[serde(default)]
    layout: LayoutConfig,
    #[serde(default)]
    debug: DebugConfig,
    #[serde(default)]
    scripts: Vec<ScriptConfig>,
}

#[derive(Deserialize, Debug, Default)]
struct LayoutConfig {
    #[serde(default)]
    show_3d_scene: bool,
    title: Option<String>,
    logo_path: Option<String>,
    #[serde(default)]
    right_panel: RightPanelConfig,
    #[serde(default)]
    docs: DocsConfig,
    #[serde(default)]
    plot: PlotConfig,
    #[serde(default)]
    input_fields: Vec<InputFieldConfig>,
    #[serde(default)]
    sliders: Vec<SliderConfig>,
}

fn default_slider_min() -> f32 { -10.0 }
fn default_slider_max() -> f32 { 10.0 }
fn default_slider_value() -> f32 { 0.0 }

#[derive(Resource, Default, Debug, Serialize)]
struct AppState {
    input_values: HashMap<String, String>,
    script_results: HashMap<String, String>,
    slider_values: HashMap<String, f32>,
    opened_file: Option<PathBuf>,
}

impl AppState {
    fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}

#[derive(Resource, Default)]
struct ScriptOutputs {
    results: Vec<String>,
}

#[derive(Resource, Default)]
struct MarkdownCache {
    cache: CommonMarkCache,
}

#[derive(Resource, Default)]
struct UiState {
    selected_tab: String,
}

#[derive(Deserialize, Debug, Default)]
struct RightPanelConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_panel_width")]
    default_width: f32,
    #[serde(default)]
    tabs: Vec<TabConfig>,
}

fn default_panel_width() -> f32 { 0.3 }

#[derive(Deserialize, Debug, Default)]
struct TabConfig {
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: String,
}

#[derive(Deserialize, Debug, Default)]
struct DocsConfig {
    #[serde(default)]
    path: String,
    #[serde(default)]
    tab: String,
}

fn main() {
    let default_config = PathBuf::from("test_apps/1_kitchen_sink/config.toml");
    let content = fs::read_to_string(&default_config).unwrap_or_default();
    let config: Config = toml::from_str(&content).unwrap_or_default();
    
    // Get the initial tab from config
    let initial_tab = config.layout.right_panel.tabs
        .first()
        .map(|tab| tab.id.clone())
        .unwrap_or_default();

    let mut app = bevy::prelude::App::new();
    
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
        .add_systems(Startup, setup)
        .add_systems(Update, (
            orbit_camera,
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
        .configure_sets(Update, AppSet::Main);

    app.add_systems(Update, (
        egui_system,
        update_streams,
    ).in_set(AppSet::Main))
    .run();
}

fn execute_script(
    script: &ScriptConfig,
    function_name: Option<&str>,
    app_state: &mut AppState,
    script_outputs: &mut ScriptOutputs,
) {
    let config_dir = app_state.opened_file
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or_else(|| Path::new("."));
    
    let script_path = config_dir.join(&script.path);
    println!("Executing script: {} ({})", script.name, script_path.display());
    
    let simplified_state = serde_json::json!({
        "input_values": &app_state.input_values,
        "slider_values": &app_state.slider_values,
    });
    
    let mut command = Command::new("python3");
    command.arg(&script_path)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
    
    if let Some(func_name) = function_name {
        if !script.functions.is_empty() {
            println!("Executing function '{}' in script", func_name);
            command.arg("--function").arg(func_name);
        }
    }
    
    if let Ok(mut child) = command.spawn() {
        // Write state to stdin if needed
        if !script.functions.is_empty() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(simplified_state.to_string().as_bytes());
            }
        }

        // Process stdout in real-time
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(output) = line {
                    let trimmed_output = output.trim().to_string();
                    
                    // Create the result key based on whether we're executing a specific function
                    let result_key = if let Some(func_name) = function_name {
                        format!("{}_{}", script.name, func_name)
                    } else {
                        script.name.clone()
                    };
                    app_state.script_results.insert(result_key, trimmed_output.clone());
                    
                    println!("Script output: {}", trimmed_output);  // Debug print
                    script_outputs.results.push(trimmed_output);
                }
            }
        }
    }
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
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
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

    // Scripts panel
    egui::TopBottomPanel::bottom("scripts_panel").show(contexts.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            if ui.button("Open File").clicked() {
                if let Some(path_str) = open_file_dialog(
                    "Open File",
                    "~",
                    None,
                ) {
                    println!("Selected file: {}", path_str);
                    let new_path = PathBuf::from(path_str);
                    // Reload the app state when a new config is selected
                    match fs::read_to_string(&new_path) {
                        Ok(new_content) => {
                            match toml::from_str::<Config>(&new_content) {
                                Ok(new_config) => {
                                    println!("Config loaded successfully: {:?}", new_config);
                                    
                                    // Clear existing state
                                    app_state.input_values.clear();
                                    app_state.script_results.clear();
                                    app_state.slider_values.clear();
                                    
                                    // Clear 3D scene if show_3d_scene is false
                                    if !new_config.layout.show_3d_scene {
                                        for camera_entity in camera_query.iter() {
                                            commands.entity(camera_entity).despawn_recursive();
                                        }
                                        for light_entity in light_query.iter() {
                                            commands.entity(light_entity).despawn_recursive();
                                        }
                                        for entity in mesh_query.iter() {
                                            commands.entity(entity).despawn_recursive();
                                        }
                                    } else {
                                        // Reinitialize the 3D scene if show_3d_scene is true
                                        setup(commands, asset_server, meshes, materials);
                                    }
                                    
                                    // Initialize sliders with default values from new config
                                    for slider in &new_config.layout.sliders {
                                        app_state.slider_values.insert(slider.id.clone(), slider.default);
                                    }
                                    
                                    // Update the file path
                                    app_state.opened_file = Some(new_path);
                                    
                                    // Update the selected tab to the first available tab
                                    if let Some(first_tab) = new_config.layout.right_panel.tabs.first() {
                                        ui_state.selected_tab = first_tab.id.clone();
                                    }
                                    
                                    // Stop any running streams
                                    stream_manager.stop_streaming();
                                }
                                Err(e) => {
                                    println!("Failed to parse config: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Failed to read file: {}", e);
                        }
                    }
                }
            }
            
            if ui.button(egui::RichText::new("Execute All Scripts").color(egui::Color32::from_rgb(0, 255, 0)))
                .clicked() 
            {
                println!("Button clicked!");
                script_outputs.results.clear();
                
                // Only start streaming if there are streaming scripts
                if has_streaming_scripts(&config.scripts) {
                    stream_manager.start_streaming();
                }
                
                // Execute discrete scripts
                for script in &config.scripts {
                    if script.script_type == "discrete" {
                        if script.functions.is_empty() {
                            // Execute entire script
                            execute_script(&script, None, &mut app_state, &mut script_outputs);
                        } else {
                            // Execute each function in the script
                            for func in &script.functions {
                                execute_script(&script, Some(&func.name), &mut app_state, &mut script_outputs);
                            }
                        }
                    } else if script.script_type == "streaming" {
                        // Get the directory of the current config file
                        let config_dir = app_state.opened_file
                            .as_ref()
                            .and_then(|p| p.parent())
                            .unwrap_or_else(|| Path::new("."));
                        
                        // Combine the config directory with the script path
                        let script_path = config_dir.join(&script.path);
                        println!("Launching streaming script: {} ({})", script.name, script_path.display());

                        // Pass app state to streaming script
                        let state_json = app_state.to_json();
                        let mut child = Command::new("python3")
                            .arg(&script_path)  // Use the resolved path
                            .stdin(Stdio::piped())
                            .spawn()
                            .expect("Failed to spawn streaming script");

                        if let Some(mut stdin) = child.stdin.take() {
                            stdin.write_all(state_json.as_bytes()).expect("Failed to write to stdin");
                        }

                        stream_manager.add_streaming_process(child);
                    }
                }
            }

            // Only show stop streaming button if there are streaming scripts
            if has_streaming_scripts(&config.scripts) {
                if ui.button("Stop Streaming").clicked() {
                    stream_manager.stop_streaming();
                }
            }

            // Add the file path display
            if let Some(path) = &app_state.opened_file {
                ui.label(format!("Selected: {}", path.display()));
            }
        });

        egui::Grid::new("scripts_grid")
            .num_columns(6)
            .spacing([40.0, 4.0])
            .striped(true)
            .min_col_width(100.0)
            .show(ui, |ui| {
                // Add headers with custom widths
                ui.label("");
                ui.label("Script");
                ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT).with_cross_justify(true), |ui| {
                    ui.set_min_width(200.0);
                    ui.label("Actions");
                });
                ui.label("Output");
                ui.label("Pass/Fail");
                ui.label("");
                ui.end_row();

                let mut row_count = 1;

                // Display discrete scripts and their outputs
                for script in &config.scripts {
                    if script.script_type == "discrete" {
                        if script.functions.is_empty() {
                            // Single row for scripts without functions
                            ui.label(row_count.to_string());
                            ui.label(&script.path);
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT).with_cross_justify(true), |ui| {
                                ui.set_min_width(200.0);
                                if ui.button("Run").clicked() {
                                    execute_script(&script, None, &mut app_state, &mut script_outputs);
                                }
                            });
                            let output = app_state.script_results
                                .get(&script.name)
                                .map_or("", |s| s.as_str())
                                .trim();
                            ui.label(if output.is_empty() { "No output" } else { output });
                            
                            // Add pass/fail indicator
                            if output.to_lowercase() == "pass" {
                                ui.colored_label(egui::Color32::GREEN, "■");
                            } else if output.to_lowercase() == "fail" {
                                ui.colored_label(egui::Color32::RED, "■");
                            } else if output.to_lowercase() == "neutral" {
                                ui.colored_label(egui::Color32::YELLOW, "■");                                
                            } else {
                                ui.label("");
                            }
                            
                            ui.label("");
                            ui.end_row();
                            row_count += 1;
                        } else {
                            // Multiple rows for scripts with functions
                            for (idx, func) in script.functions.iter().enumerate() {
                                ui.label(row_count.to_string());
                                if idx == 0 {
                                    ui.label(&script.path);
                                } else {
                                    ui.label("");
                                }
                                
                                ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT).with_cross_justify(true), |ui| {
                                    ui.set_min_width(200.0);
                                    if ui.button(&func.display).clicked() {
                                        execute_script(&script, Some(&func.name), &mut app_state, &mut script_outputs);
                                    }
                                });

                                let output_key = format!("{}_{}", script.name, func.name);
                                let output = app_state.script_results
                                    .get(&output_key)
                                    .map_or("", |s| s.as_str())
                                    .trim();
                                ui.label(if output.is_empty() { "No output" } else { output });
                                
                                // Add pass/fail indicator
                                if output.to_lowercase() == "pass" {
                                    ui.colored_label(egui::Color32::GREEN, "■");
                                } else if output.to_lowercase() == "fail" {
                                    ui.colored_label(egui::Color32::RED, "■");
                                } else if output.to_lowercase() == "neutral" {
                                    ui.colored_label(egui::Color32::YELLOW, "■");                                
                                } else {
                                    ui.label("");
                                }
                                
                                ui.label("");
                                ui.end_row();
                                row_count += 1;
                            }
                        }
                    }
                }

                // Only show streaming scripts section if there are streaming scripts
                if has_streaming_scripts(&config.scripts) {
                    ui.label("Streaming Scripts");
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT).with_cross_justify(true), |ui| {
                        ui.set_min_width(200.0);
                        ui.label("Status");
                    });
                    ui.label("");
                    ui.label("");
                    ui.end_row();

                    for script in &config.scripts {
                        if script.script_type == "streaming" {
                            ui.label(&script.path);
                            ui.label(if stream_manager.is_running() { "Running" } else { "Stopped" });
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                        }
                    }
                }
            });
    });

    // Update the right panel code:
    if config.layout.right_panel.enabled {
        egui::SidePanel::right("right_panel")
            .default_width(window_width * config.layout.right_panel.default_width)
            .resizable(true)
            .show(contexts.ctx_mut(), |ui| {
                // Tab Bar
                ui.horizontal(|ui| {
                    for tab in &config.layout.right_panel.tabs {
                        let selected = ui_state.selected_tab == tab.id;
                        if ui.selectable_label(selected, &tab.label).clicked() {
                            ui_state.selected_tab = tab.id.clone();
                        }
                    }
                });

            ui.separator();

                // Tab Content
                match ui_state.selected_tab.as_str() {
                    tab_id => {
                        // Documentation
                        if config.layout.docs.tab == tab_id {
                            if let Some(opened_file) = &app_state.opened_file {
                                let full_docs_path = opened_file.parent()
                                    .unwrap_or_else(|| Path::new(""))
                                    .join(&config.layout.docs.path);
                                
                                match fs::read_to_string(&full_docs_path) {
                                    Ok(content) => {
                                        egui::ScrollArea::vertical().show(ui, |ui| {
                                            CommonMarkViewer::new()
                                                .show(ui, &mut markdown_cache.cache, &content);
                                        });
                                    },
                                    Err(e) => {
                                        ui.label(format!("Could not load documentation: {}", e));
                                    }
                                }
                            }
                        }

                        // Plot
                        if config.layout.plot.tab == tab_id {
                            let plot = Plot::new("streaming_plot")
                                .view_aspect(2.0);
                            plot.show(ui, |plot_ui| {
                                if has_streaming_scripts(&config.scripts) {
                                    if let Ok(streams) = stream_manager.as_ref().streams.lock() {
                                        for (stream_id, points) in streams.iter() {
                                            if !points.is_empty() {
                                                let line = Line::new(PlotPoints::new(points.clone()))
                                                    .name(stream_id)
                                                    .width(2.0);
                                                plot_ui.line(line);
                                            }
                                        }
                                    }
                                }
                            });
                        }

                        // Input Fields
                        let tab_input_fields: Vec<_> = config.layout.input_fields.iter()
                            .filter(|field| field.tab == tab_id)
                            .collect();
                        
                        if !tab_input_fields.is_empty() {
                            ui.vertical(|ui| {
                                for field in tab_input_fields {
                                    ui.horizontal(|ui| {
                                        ui.label(&field.label);
                                        let value = app_state.input_values
                                            .entry(field.id.clone())
                                            .or_insert_with(String::new);
                                        ui.text_edit_singleline(value);
                                    });
                                }
                            });
                        }

                        // Sliders
                        let tab_sliders: Vec<_> = config.layout.sliders.iter()
                            .filter(|slider| slider.tab == tab_id)
                            .collect();
                        
                        if !tab_sliders.is_empty() {
                            ui.vertical(|ui| {
                                for slider in tab_sliders {
                                    ui.horizontal(|ui| {
                                        ui.label(&slider.label);
                                        let value = app_state.slider_values
                                            .entry(slider.id.clone())
                                            .or_insert(slider.default);
                                        let _ = ui.add(egui::Slider::new(
                                            value, 
                                            slider.min..=slider.max
                                        ));
                                    });
                                }
                            });
                        }
                    }
                }
            });
    }
}



