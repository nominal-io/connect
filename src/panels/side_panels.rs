use bevy_egui::egui;
use egui_commonmark::CommonMarkViewer;
use egui_extras::{Column, TableBuilder};
use egui_plot::{Line, Plot, PlotPoints};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::executors::streaming::StreamManager;
use crate::has_streaming_scripts;
use crate::types::*;

/// Displays the left panel of the application UI if enabled in the config.
pub fn show_left_panel(
    ui_state: &mut UiState,
    app_state: &mut AppState,
    config: &Config,
    window_width: f32,
    stream_manager: &StreamManager,
    markdown_cache: &mut MarkdownCache,
    ctx: &mut bevy_egui::egui::Context,
) {
    if !config.layout.left_panel.enabled {
        return;
    }

    egui::SidePanel::left("left_panel")
        .default_width(window_width * config.layout.left_panel.default_width)
        .resizable(true)
        .show(ctx, |ui| {
            show_tab_bar(
                ui,
                &mut ui_state.left_selected_tab,
                &config.layout.left_panel.tabs,
            );
            ui.separator();
            show_tab_content(
                ui,
                &ui_state.left_selected_tab,
                app_state,
                config,
                stream_manager,
                markdown_cache,
            );
        });
}

/// Displays the right panel of the application UI if enabled in the config.
pub fn show_right_panel(
    ui_state: &mut UiState,
    app_state: &mut AppState,
    config: &Config,
    window_width: f32,
    stream_manager: &StreamManager,
    markdown_cache: &mut MarkdownCache,
    ctx: &mut bevy_egui::egui::Context,
) {
    if !config.layout.right_panel.enabled {
        return;
    }

    egui::SidePanel::right("right_panel")
        .default_width(window_width * config.layout.right_panel.default_width)
        .resizable(true)
        .show(ctx, |ui| {
            show_tab_bar(
                ui,
                &mut ui_state.right_selected_tab,
                &config.layout.right_panel.tabs,
            );
            ui.separator();
            show_tab_content(
                ui,
                &ui_state.right_selected_tab,
                app_state,
                config,
                stream_manager,
                markdown_cache,
            );
        });
}

/// Renders the horizontal tab bar at the top of a panel
fn show_tab_bar(ui: &mut egui::Ui, selected_tab: &mut String, tabs: &[TabConfig]) {
    ui.horizontal(|ui| {
        for tab in tabs {
            let selected = *selected_tab == tab.id;
            if ui.selectable_label(selected, &tab.label).clicked() {
                *selected_tab = tab.id.clone();
            }
        }
    });
}

/// Displays the content for the currently selected tab.
/// Routes to specific view handlers based on the selected tab ID.
fn show_tab_content(
    ui: &mut egui::Ui,
    selected_tab: &str,
    app_state: &mut AppState,
    config: &Config,
    stream_manager: &StreamManager,
    markdown_cache: &mut MarkdownCache,
) {
    match selected_tab {
        "table_view" => show_table_view(ui, app_state),
        tab_id => show_other_tab_content(
            ui,
            tab_id,
            app_state,
            config,
            stream_manager,
            markdown_cache,
        ),
    }
}

/// Renders the table view tab content, showing script execution results in tabular format.
/// Includes debug logging and handles empty state display.
fn show_table_view(ui: &mut egui::Ui, app_state: &mut AppState) {
    let now = Instant::now();
    let debug_interval = Duration::from_secs(1);

    ui.push_id("table_view_container", |ui| {
        // Overall debug message
        if should_debug(
            app_state.table_display_state.last_debug,
            now,
            debug_interval,
        ) {
            println!(
                "Table view tab selected. Tables count: {}",
                app_state.script_tables.len()
            );
            app_state.table_display_state.last_debug = Some(now);
        }

        if app_state.script_tables.is_empty() {
            ui.push_id("empty_table_message", |ui| {
                ui.label("No table data available");
            });
            return;
        }

        show_tables_scroll_area(ui, app_state, now, debug_interval);
    });
}

/// Helper function to determine if debug information should be logged
/// based on the time elapsed since the last debug message.
fn should_debug(last_debug: Option<Instant>, now: Instant, interval: Duration) -> bool {
    last_debug.map_or(true, |last| now.duration_since(last) > interval)
}

/// Creates a vertical scrollable area to display all tables.
/// Handles both debugging and rendering of table content.
fn show_tables_scroll_area(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    now: Instant,
    debug_interval: Duration,
) {
    egui::ScrollArea::vertical()
        .id_salt("table_scroll_area")
        .show(ui, |ui| {
            debug_tables(app_state, now, debug_interval);
            render_tables(ui, app_state);
        });
}

/// Logs debug information about tables being displayed,
/// including the number of columns and rows for each table.
fn debug_tables(app_state: &mut AppState, now: Instant, debug_interval: Duration) {
    let debug_tables: Vec<_> = app_state
        .script_tables
        .iter()
        .filter(|(script_name, _)| {
            should_debug(
                app_state
                    .table_display_state
                    .table_debugs
                    .get(*script_name)
                    .copied(),
                now,
                debug_interval,
            )
        })
        .map(|(name, data)| (name.clone(), data.columns.len(), data.data.len()))
        .collect();

    for (script_name, cols, rows) in debug_tables {
        println!(
            "Displaying table for {}: {} columns, {} rows",
            script_name, cols, rows
        );
        app_state
            .table_display_state
            .table_debugs
            .insert(script_name, now);
    }
}

/// Renders all available tables in the UI, including their headers and data.
/// Each table is displayed with its script name as a heading.
fn render_tables(ui: &mut egui::Ui, app_state: &AppState) {
    for (script_name, table_data) in &app_state.script_tables {
        ui.push_id(format!("table_container_{}", script_name), |ui| {
            ui.push_id(format!("table_heading_{}", script_name), |ui| {
                ui.heading(script_name);
            });

            show_table_grid(ui, script_name, table_data);

            ui.push_id(format!("table_spacing_{}", script_name), |ui| {
                ui.add_space(20.0);
            });
        });
    }
}

/// Creates and configures the grid layout for a single table.
/// Sets up the table builder with appropriate styling and layout options.
fn show_table_grid(ui: &mut egui::Ui, script_name: &str, table_data: &TableData) {
    ui.push_id(format!("table_grid_{}", script_name), |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .columns(
                Column::auto().at_least(80.0).resizable(true),
                table_data.columns.len(),
            )
            .header(30.0, |mut header| {
                show_table_header(&mut header, script_name, table_data);
            })
            .body(|mut body| {
                show_table_body(&mut body, script_name, table_data);
            });
    });
}

/// Renders the header row of a table with column names.
/// Each column header is displayed in bold text.
fn show_table_header(
    header: &mut egui_extras::TableRow,
    script_name: &str,
    table_data: &TableData,
) {
    for (col_idx, col_name) in table_data.columns.iter().enumerate() {
        header.col(|ui| {
            ui.push_id(format!("header_{}_{}", script_name, col_idx), |ui| {
                ui.strong(col_name);
            });
        });
    }
}

/// Renders the data rows of a table.
/// Displays each cell's content in a formatted grid layout.
fn show_table_body(body: &mut egui_extras::TableBody, script_name: &str, table_data: &TableData) {
    for (row_idx, row_data) in table_data.data.iter().enumerate() {
        body.row(25.0, |mut row| {
            for (col_idx, cell) in row_data.iter().enumerate() {
                row.col(|ui| {
                    ui.push_id(
                        format!("cell_{}_{}_{}", script_name, row_idx, col_idx),
                        |ui| {
                            ui.label(cell);
                        },
                    );
                });
            }
        });
    }
}

/// Handles the display of non-table tab content including plots, documentation,
/// input fields, and sliders based on the configuration.
fn show_other_tab_content(
    ui: &mut egui::Ui,
    tab_id: &str,
    app_state: &mut AppState,
    config: &Config,
    stream_manager: &StreamManager,
    markdown_cache: &mut MarkdownCache,
) {
    ui.push_id(format!("tab_content_{}", tab_id), |ui| {
        show_plot_if_configured(ui, tab_id, config, stream_manager);
        show_docs_if_configured(ui, tab_id, app_state, config, markdown_cache);
        show_input_fields(ui, tab_id, app_state, config);
        show_sliders(ui, tab_id, app_state, config);
    });
}

/// Renders a plot if configured for the current tab.
/// Displays streaming data points in a line graph format.
fn show_plot_if_configured(
    ui: &mut egui::Ui,
    tab_id: &str,
    config: &Config,
    stream_manager: &StreamManager,
) {
    if config.layout.plot.tab == tab_id {
        ui.push_id("plot_container", |ui| {
            let plot = Plot::new("streaming_plot").view_aspect(2.0);
            plot.show(ui, |plot_ui| {
                if has_streaming_scripts(&config.scripts) {
                    if let Ok(streams) = stream_manager.streams.lock() {
                        // Changed from "sine_wave" to "single_scalar_channel"
                        if let Some(points) = streams.get("single_scalar_channel") {
                            if !points.is_empty() {
                                let plot_points: Vec<[f64; 2]> = points
                                    .iter()
                                    .filter_map(|point| point.as_plot2d())
                                    .collect();
                                let line = Line::new(PlotPoints::new(plot_points));
                                plot_ui.line(line);
                            }
                        }
                    }
                }
            });
        });
        ui.add_space(20.0);
    }
}

/// Displays markdown documentation if configured for the current tab.
/// Reads and renders documentation from the specified file path.
fn show_docs_if_configured(
    ui: &mut egui::Ui,
    tab_id: &str,
    app_state: &AppState,
    config: &Config,
    markdown_cache: &mut MarkdownCache,
) {
    if config.layout.docs.tab == tab_id {
        ui.push_id("instructions_container", |ui| {
            if let Some(opened_file) = &app_state.opened_file {
                let full_docs_path = opened_file
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(&config.layout.docs.path);

                match fs::read_to_string(&full_docs_path) {
                    Ok(content) => {
                        egui::ScrollArea::vertical()
                            .id_salt("instructions_scroll")
                            .show(ui, |ui| {
                                CommonMarkViewer::new().show(
                                    ui,
                                    &mut markdown_cache.cache,
                                    &content,
                                );
                            });
                    }
                    Err(e) => {
                        ui.label(format!("Could not load documentation: {}", e));
                    }
                }
            }
        });
    }
}

/// Renders input fields configured for the current tab.
/// Each field includes a label and a single-line text input.
fn show_input_fields(ui: &mut egui::Ui, tab_id: &str, app_state: &mut AppState, config: &Config) {
    let tab_input_fields: Vec<_> = config
        .layout
        .input_fields
        .iter()
        .filter(|field| field.tab == tab_id)
        .collect();

    if !tab_input_fields.is_empty() {
        ui.push_id("input_fields_section", |ui| {
            ui.vertical(|ui| {
                for field in tab_input_fields {
                    ui.push_id(&field.id, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(&field.label);
                            let value = app_state
                                .input_values
                                .entry(field.id.clone())
                                .or_default();
                            ui.text_edit_singleline(value);
                        });
                    });
                }
            });
        });
        ui.add_space(10.0);
    }
}

/// Displays slider controls configured for the current tab.
/// Each slider includes a label and allows value adjustment within defined bounds.
fn show_sliders(ui: &mut egui::Ui, tab_id: &str, app_state: &mut AppState, config: &Config) {
    let tab_sliders: Vec<_> = config
        .layout
        .sliders
        .iter()
        .filter(|slider| slider.tab == tab_id)
        .collect();

    if !tab_sliders.is_empty() {
        ui.push_id("sliders_section", |ui| {
            ui.vertical(|ui| {
                for slider in tab_sliders {
                    ui.push_id(&slider.id, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(&slider.label);
                            let value = app_state
                                .slider_values
                                .entry(slider.id.clone())
                                .or_insert(slider.default);
                            let _ = ui.add(egui::Slider::new(value, slider.min..=slider.max));
                        });
                    });
                }
            });
        });
    }
}
