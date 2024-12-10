mod streaming;
mod camera;
mod scene;

use streaming::{StreamManager, update_streams};
use camera::{orbit_camera, camera_ui};
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
    id: String,
    label: String,
}

#[derive(Deserialize, Debug, Default)]
struct PlotConfig {
    // We can add fields back when we need them
}

#[derive(Deserialize, Debug)]
struct SliderConfig {
    id: String,
    label: String,
    #[serde(default = "default_slider_min")]
    min: f32,
    #[serde(default = "default_slider_max")]
    max: f32,
    #[serde(default = "default_slider_value")]
    default: f32,
}

impl Default for SliderConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            min: default_slider_min(),
            max: default_slider_max(),
            default: default_slider_value(),
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
    #[serde(default = "default_show_3d")]
    show_3d_scene: bool,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    input_fields: Vec<InputFieldConfig>,
    #[serde(default)]
    plot: Option<PlotConfig>,
    #[serde(default)]
    sliders: Vec<SliderConfig>,
}

fn default_show_3d() -> bool { false }
fn default_slider_min() -> f32 { -10.0 }
fn default_slider_max() -> f32 { 10.0 }
fn default_slider_value() -> f32 { 0.0 }

#[derive(Resource, Default, Debug, Serialize)]
struct AppState {
    input_values: HashMap<String, String>,
    script_results: HashMap<String, String>,
    slider_values: HashMap<String, f32>,
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

fn main() {
    let content = fs::read_to_string("scripts.toml").unwrap_or_default();
    let config: Config = toml::from_str(&content).unwrap_or_default();
    
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: config.layout.title.unwrap_or("Connect".to_string()).into(),
                mode: WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .insert_resource(StreamManager::new(config.debug.streaming))
        .insert_resource(ScriptOutputs::default())
        .insert_resource(AppState::default())
        .configure_sets(Update, AppSet::Main);

    // Only add 3D scene systems if enabled
    if config.layout.show_3d_scene {
        app.add_systems(Startup, setup)
           .add_systems(Update, (
               orbit_camera,
               camera_ui,
           ).in_set(AppSet::Main));
    }

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
    let full_path = format!("src/{}", script.path);
    println!("Executing script: {} ({})", script.name, full_path);
    
    // Create a simplified state object that includes both input values and slider values
    let simplified_state = serde_json::json!({
        "input_values": &app_state.input_values,
        "slider_values": &app_state.slider_values,
    });
    
    // Create command with script path
    let mut command = Command::new("python3");
    command.arg(&full_path)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
    
    if let Some(func_name) = function_name {
        if !script.functions.is_empty() {
            println!("Executing function '{}' in script", func_name);
            command.arg("--function").arg(func_name);
        }
    }
    
    // Spawn process and handle IO
    if let Ok(mut child) = command.spawn() {
        // Only write to stdin if we're expecting the script to read it
        if !script.functions.is_empty() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(simplified_state.to_string().as_bytes());
            }
        }

        let output = child.wait_with_output()
            .map(|output| {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    app_state.script_results.insert(script.name.clone(), stdout.clone());
                    stdout
                } else {
                    String::from_utf8_lossy(&output.stderr).to_string()
                }
            })
            .unwrap_or_else(|e| format!("Error executing script: {}", e));
        
        println!("Script output: {}", output);  // Debug print
        script_outputs.results.push(output);
    }
}

fn egui_system(
    mut contexts: EguiContexts,
    mut script_outputs: ResMut<ScriptOutputs>,
    mut stream_manager: ResMut<StreamManager>,
    mut app_state: ResMut<AppState>,
    windows: Query<&Window>,
) {
    let content = fs::read_to_string("scripts.toml").unwrap_or_default();
    let config: Config = toml::from_str(&content).unwrap_or_default();
    
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
            if ui.button(egui::RichText::new("Execute All Scripts").color(egui::Color32::from_rgb(0, 255, 0)))
                .clicked() 
            {
                println!("Button clicked!");
                script_outputs.results.clear();
                stream_manager.start_streaming();
                
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
                        let path = script.path.clone();
                        println!("Launching streaming script: {} ({})", script.name, path);
                        
                        // Pass app state to streaming script
                        let state_json = app_state.to_json();
                        let mut child = Command::new("python3")
                            .arg(&format!("src/{}", path))
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
            
            if ui.button("Stop Streaming").clicked() {
                stream_manager.stop_streaming();
            }
        });

        egui::Grid::new("scripts_grid")
            .num_columns(4)
            .spacing([40.0, 4.0])
            .striped(true)
            .min_col_width(100.0)
            .show(ui, |ui| {
                // Add headers with custom widths
                ui.label("Script");
                ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT).with_cross_justify(true), |ui| {
                    ui.set_min_width(200.0);  // Make Actions column wider
                    ui.label("Actions");
                });
                ui.label("Output");
                ui.label("");  // Empty column for alignment
                ui.end_row();

                // Display discrete scripts and their outputs
                for script in &config.scripts {
                    if script.script_type == "discrete" {
                        ui.label(&script.path);
                        
                        // Function buttons column with wider space
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT).with_cross_justify(true), |ui| {
                            ui.set_min_width(200.0);  // Match header width
                            ui.vertical(|ui| {
                                if script.functions.is_empty() {
                                    if ui.button("Run").clicked() {
                                        execute_script(&script, None, &mut app_state, &mut script_outputs);
                                    }
                                } else {
                                    for func in &script.functions {
                                        if ui.button(format!("Run {}", &func.display)).clicked() {
                                            execute_script(&script, Some(&func.name), &mut app_state, &mut script_outputs);
                                        }
                                    }
                                }
                            });
                        });

                        let output = app_state.script_results
                            .get(&script.name)
                            .map_or("", |s| s.as_str())
                            .trim();
                        ui.label(if output.is_empty() { "No output" } else { output });
                        ui.label("");
                        ui.end_row();
                    }
                }
                
                // Streaming scripts section
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
            });
    });

    // Plot panel with configurable settings
    egui::SidePanel::right("plot_panel")
        .default_width(window_width * 0.5)
        .resizable(true)
        .show(contexts.ctx_mut(), |ui| {
            if let Some(_plot_config) = &config.layout.plot {
                let plot = Plot::new("streaming_plot")
                    .view_aspect(2.0);
                
                plot.show(ui, |plot_ui| {
                    if let Ok(streams) = stream_manager.as_ref().streams.lock() {
                        for (stream_id, points) in streams.iter() {
                            if points.is_empty() {
                                if stream_manager.as_ref().debug {
                                    println!("Stream {} has no points to plot", stream_id);
                                }
                                continue;
                            }
                            
                            if stream_manager.as_ref().debug {
                                println!("Plotting {} points for stream {}", points.len(), stream_id);
                                println!("First point: {:?}, Last point: {:?}", 
                                    points.first(), points.last());
                                // Clone the HashMap for serialization
                                let streams_data = streams.clone();
                                println!("Current streams data: {}", serde_json::to_string(&streams_data).unwrap());
                            }
                            
                            let line = Line::new(PlotPoints::new(points.clone()))
                                .name(stream_id)
                                .width(2.0);
                            plot_ui.line(line);
                        }
                    } else if stream_manager.as_ref().debug {
                        println!("Failed to lock streams for plotting");
                    }
                });
            }

            // Render input fields
            if !config.layout.input_fields.is_empty() {
                ui.vertical(|ui| {
                    for field in &config.layout.input_fields {
                        ui.horizontal(|ui| {
                            ui.label(&field.label);
                            let value = app_state.input_values.entry(field.id.clone())
                                .or_insert_with(String::new);
                            ui.text_edit_singleline(value);
                        });
                    }
                });
            }

            // Update slider rendering section
            if !config.layout.sliders.is_empty() {
                ui.separator();
                ui.vertical(|ui| {
                    for slider in &config.layout.sliders {
                        ui.horizontal(|ui| {
                            ui.label(&slider.label);
                            let value = app_state.slider_values.entry(slider.id.clone())
                                .or_insert(slider.default);
                            let _ = ui.add(egui::Slider::new(value, slider.min..=slider.max));
                        });
                    }
                });
            }
        });
}



