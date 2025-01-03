use crate::executors::streaming::{ProcessStatus, StreamManager};

use bevy::prelude::*;
use bevy_egui::egui;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tinyfiledialogs::open_file_dialog;

use crate::{
    execute_script, has_streaming_scripts, AppState, Config, ScriptConfig, ScriptOutputs, UiState,
};

use crate::gym3d::scene::handle_3d_scene_update;

pub fn show_scripts_panel(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    app_state: &mut AppState,
    script_outputs: &mut ScriptOutputs,
    stream_manager: &mut StreamManager,
    ui_state: &mut UiState,
    config: &Config,
    camera_query: &Query<Entity, With<Camera3d>>,
    light_query: &Query<Entity, With<PointLight>>,
    mesh_query: &Query<Entity, With<Mesh3d>>,
    _asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    ui.horizontal(|ui| {
        show_script_controls(ui, app_state, script_outputs, stream_manager, config);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            show_file_controls(
                ui,
                commands,
                app_state,
                ui_state,
                camera_query,
                light_query,
                mesh_query,
                _asset_server,
                meshes,
                materials,
            );
        });
    });

    show_scripts_grid(ui, app_state, script_outputs, stream_manager, config);
}

/// Displays file-related controls including open file button and current file display
fn show_file_controls(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    app_state: &mut AppState,
    ui_state: &mut UiState,
    camera_query: &Query<Entity, With<Camera3d>>,
    light_query: &Query<Entity, With<PointLight>>,
    mesh_query: &Query<Entity, With<Mesh3d>>,
    _asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    if ui.button("Open Config File").clicked() {
        if let Some(path_str) = open_file_dialog("Open File", "~", None) {
            handle_file_selection(
                path_str,
                commands,
                app_state,
                ui_state,
                camera_query,
                light_query,
                mesh_query,
                _asset_server,
                meshes,
                materials,
            );
        }
    }

    if let Some(path) = &app_state.opened_file {
        ui.label(format!("Selected: {}", path.display()));
    }
}

/// Handles the file selection process, loads config, and updates application state
fn handle_file_selection(
    path_str: String,
    commands: &mut Commands,
    app_state: &mut AppState,
    ui_state: &mut UiState,
    camera_query: &Query<Entity, With<Camera3d>>,
    light_query: &Query<Entity, With<PointLight>>,
    mesh_query: &Query<Entity, With<Mesh3d>>,
    _asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    debug!("Selected file: {}", path_str);
    let new_path = PathBuf::from(path_str);

    match fs::read_to_string(&new_path) {
        Ok(new_content) => {
            match toml::from_str::<Config>(&new_content) {
                Ok(new_config) => {
                    info!("Config loaded successfully: {:?}", new_config);

                    // Clear existing state
                    app_state.input_values.clear();
                    app_state.script_results.clear();
                    app_state.slider_values.clear();

                    handle_3d_scene_update(
                        &new_config,
                        commands,
                        camera_query,
                        light_query,
                        mesh_query,
                        _asset_server,
                        meshes,
                        materials,
                    );

                    // Initialize sliders
                    for slider in &new_config.layout.sliders {
                        app_state
                            .slider_values
                            .insert(slider.id.clone(), slider.default);
                    }

                    app_state.opened_file = Some(new_path);

                    // Update selected tab
                    if let Some(first_tab) = new_config.layout.right_panel.tabs.first() {
                        ui_state.right_selected_tab = first_tab.id.clone();
                    }
                }
                Err(e) => error!("Failed to parse config: {}", e),
            }
        }
        Err(e) => error!("Failed to read file: {}", e),
    }
}

/// Displays script execution controls including execute all and stream control buttons
fn show_script_controls(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    script_outputs: &mut ScriptOutputs,
    stream_manager: &mut StreamManager,
    config: &Config,
) {
    if ui
        .button(
            egui::RichText::new("Execute All Scripts").color(egui::Color32::from_rgb(0, 255, 0)),
        )
        .clicked()
    {
        handle_execute_all(app_state, script_outputs, stream_manager, config);
    }

    if has_streaming_scripts(&config.scripts) && ui.button("Stop Streaming").clicked() {
        stream_manager.stop_streaming();
    }
}

/// Executes all scripts in the config, handling both discrete and streaming types
fn handle_execute_all(
    app_state: &mut AppState,
    script_outputs: &mut ScriptOutputs,
    stream_manager: &mut StreamManager,
    config: &Config,
) {
    script_outputs.results.clear();

    if has_streaming_scripts(&config.scripts) {
        stream_manager.start_streaming();
    }

    for script in &config.scripts {
        match script.script_type.as_str() {
            "discrete" => handle_discrete_script(script, app_state, script_outputs),
            "streaming" => handle_streaming_script(script, app_state, stream_manager),
            _ => error!("Unknown script type: {}", script.script_type),
        }
    }
}

/// Executes a discrete script, handling both single and multi-function scripts
fn handle_discrete_script(
    script: &ScriptConfig,
    app_state: &mut AppState,
    script_outputs: &mut ScriptOutputs,
) {
    if script.functions.is_empty() {
        execute_script(script, None, app_state, script_outputs);
    } else {
        for func in &script.functions {
            execute_script(script, Some(&func.name), app_state, script_outputs);
        }
    }
}

/// Launches a streaming script process and sets up input/output handling
fn handle_streaming_script(
    script: &ScriptConfig,
    app_state: &mut AppState,
    stream_manager: &mut StreamManager,
) {
    let config_dir = app_state
        .opened_file
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or_else(|| Path::new("."));

    let script_path = config_dir.join(&script.path);
    info!(
        "Launching streaming script: {} ({})",
        script.name,
        script_path.display()
    );

    let state_json = app_state.to_json();
    let mut child = Command::new("python3")
        .arg(&script_path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to spawn streaming script");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(state_json.as_bytes())
            .expect("Failed to write to stdin");
    }

    stream_manager.add_streaming_process(child);
}

/// Displays the main grid containing all script information and controls
fn show_scripts_grid(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    script_outputs: &mut ScriptOutputs,
    stream_manager: &StreamManager,
    config: &Config,
) {
    egui::Grid::new("scripts_grid")
        .num_columns(6)
        .spacing([40.0, 4.0])
        .striped(true)
        .min_col_width(100.0)
        .show(ui, |ui| {
            show_grid_headers(ui);
            show_discrete_scripts(ui, app_state, script_outputs, config);
            show_streaming_scripts(ui, stream_manager, config);
        });
}

/// Renders the header row for the scripts grid
fn show_grid_headers(ui: &mut egui::Ui) {
    ui.label("");
    ui.label("Script");
    ui.with_layout(
        egui::Layout::left_to_right(egui::Align::LEFT).with_cross_justify(true),
        |ui| {
            ui.set_min_width(200.0);
            ui.label("Actions");
        },
    );
    ui.label("Output");
    ui.label("Pass/Fail");
    ui.label("");
    ui.end_row();
}

/// Displays all discrete scripts in the grid
fn show_discrete_scripts(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    script_outputs: &mut ScriptOutputs,
    config: &Config,
) {
    let mut row_count = 1;

    for script in &config.scripts {
        if script.script_type == "discrete" {
            if script.functions.is_empty() {
                show_single_script_row(ui, script, None, app_state, script_outputs, row_count);
                row_count += 1;
            } else {
                show_script_with_functions(ui, script, app_state, script_outputs, &mut row_count);
            }
        }
    }
}

/// Renders a single row in the scripts grid for a script/function
fn show_single_script_row(
    ui: &mut egui::Ui,
    script: &ScriptConfig,
    function_name: Option<&str>,
    app_state: &mut AppState,
    script_outputs: &mut ScriptOutputs,
    row_count: i32,
) {
    ui.label(row_count.to_string());
    ui.label(&script.path);

    ui.with_layout(
        egui::Layout::left_to_right(egui::Align::LEFT).with_cross_justify(true),
        |ui| {
            ui.set_min_width(200.0);
            if ui.button(function_name.unwrap_or("Run")).clicked() {
                execute_script(script, function_name, app_state, script_outputs);
            }
        },
    );

    let output_key = if let Some(func_name) = function_name {
        format!("{}_{}", script.name, func_name)
    } else {
        script.name.clone()
    };

    let output = app_state
        .script_results
        .get(&output_key)
        .map_or("", |s| s.as_str())
        .trim();

    ui.label(if output.is_empty() {
        "No output"
    } else {
        output
    });
    show_status_indicator(ui, output);
    ui.label("");
    ui.end_row();
}

/// Displays a script with multiple functions, showing each function as a separate row
fn show_script_with_functions(
    ui: &mut egui::Ui,
    script: &ScriptConfig,
    app_state: &mut AppState,
    script_outputs: &mut ScriptOutputs,
    row_count: &mut i32,
) {
    for (idx, func) in script.functions.iter().enumerate() {
        ui.label(row_count.to_string());
        if idx == 0 {
            ui.label(&script.path);
        } else {
            ui.label("");
        }

        ui.with_layout(
            egui::Layout::left_to_right(egui::Align::LEFT).with_cross_justify(true),
            |ui| {
                ui.set_min_width(200.0);
                if ui.button(&func.display).clicked() {
                    execute_script(script, Some(&func.name), app_state, script_outputs);
                }
            },
        );

        let output_key = format!("{}_{}", script.name, func.name);
        let output = app_state
            .script_results
            .get(&output_key)
            .map_or("", |s| s.as_str())
            .trim();

        ui.label(if output.is_empty() {
            "No output"
        } else {
            output
        });
        show_status_indicator(ui, output);
        ui.label("");
        ui.end_row();
        *row_count += 1;
    }
}

/// Displays a colored status indicator based on script output
fn show_status_indicator(ui: &mut egui::Ui, output: &str) {
    match output.to_lowercase().as_str() {
        "pass" => ui.colored_label(egui::Color32::GREEN, "■"),
        "fail" => ui.colored_label(egui::Color32::RED, "■"),
        "neutral" => ui.colored_label(egui::Color32::YELLOW, "■"),
        _ => ui.label(""),
    };
}

/// Displays information about currently running streaming scripts
fn show_streaming_scripts(ui: &mut egui::Ui, stream_manager: &StreamManager, config: &Config) {
    if has_streaming_scripts(&config.scripts) {
        ui.label("Streaming Scripts");
        ui.with_layout(
            egui::Layout::left_to_right(egui::Align::LEFT).with_cross_justify(true),
            |ui| {
                ui.set_min_width(200.0);
                ui.label("Status");
            },
        );
        ui.label("");
        ui.label("");
        ui.end_row();

        if let Ok(statuses) = stream_manager.process_statuses.lock() {
            for (i, script) in config.scripts.iter().enumerate() {
                if script.script_type == "streaming" {
                    ui.label(&script.path);

                    // Get the status for this script's process
                    let status = statuses.get(i).cloned().unwrap_or(ProcessStatus::Finished);
                    match status {
                        ProcessStatus::Running => {
                            ui.label("Running");
                        }
                        ProcessStatus::Failed(Some(code)) => {
                            ui.colored_label(
                                egui::Color32::RED,
                                format!("Terminated (code: {})", code),
                            );
                        }
                        ProcessStatus::Failed(None) => {
                            ui.colored_label(egui::Color32::RED, "Terminated (unknown)");
                        }
                        ProcessStatus::Finished => {
                            ui.label("Stopped");
                        }
                    }

                    ui.label("");
                    ui.label("");
                    ui.end_row();
                }
            }
        }
    }
}
