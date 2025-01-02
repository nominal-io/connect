use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use serde::{Deserialize, Deserializer, Serialize};
use bevy::prelude::*;
use egui_commonmark::CommonMarkCache;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum AppSet {
    Main,
}

#[derive(Deserialize, Debug)]
pub struct FunctionConfig {
    pub name: String,
    pub display: String,
}

#[derive(Deserialize, Debug)]
pub struct ScriptConfig {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub script_type: String,
    #[serde(default)]
    pub functions: Vec<FunctionConfig>,
}

#[derive(Deserialize, Debug, Default)]
pub struct InputFieldConfig {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub tab: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct PlotConfig {
    #[serde(default)]
    pub tab: String,
}

#[derive(Deserialize, Debug)]
pub struct SliderConfig {
    pub id: String,
    pub label: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub tab: String,
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

#[derive(Deserialize, Debug, Default)]
pub struct DebugConfig {
    #[serde(default)]
    pub streaming: bool,
}

#[derive(Deserialize, Debug, Default)]
pub struct Config {
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub debug: DebugConfig,
    #[serde(default)]
    pub scripts: Vec<ScriptConfig>,
}

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct LayoutConfig {
    #[serde(default)]
    pub show_3d_scene: bool,
    pub title: Option<String>,
    #[serde(default)]
    pub left_panel: PanelConfig,
    #[serde(default)]
    pub right_panel: RightPanelConfig,
    #[serde(default)]
    pub docs: DocsConfig,
    #[serde(default)]
    pub plot: PlotConfig,
    #[serde(default)]
    pub input_fields: Vec<InputFieldConfig>,
    #[serde(default)]
    pub sliders: Vec<SliderConfig>,
    #[serde(default)]
    pub table: TableConfig,
}

pub fn default_slider_min() -> f32 { -10.0 }
pub fn default_slider_max() -> f32 { 10.0 }
pub fn default_slider_value() -> f32 { 0.0 }

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct TableData {
    pub columns: Vec<String>,
    #[serde(deserialize_with = "deserialize_string_array")]
    pub data: Vec<Vec<String>>,
    #[serde(default)]
    pub error: Option<String>,
}

pub fn deserialize_string_array<'de, D>(deserializer: D) -> Result<Vec<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_data: Vec<Vec<Option<String>>> = Vec::deserialize(deserializer)?;
    Ok(raw_data
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell| cell.unwrap_or_default())
                .collect()
        })
        .collect())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableDisplayState {
    #[serde(skip)]
    pub last_debug: Option<Instant>,
    #[serde(skip)]
    pub table_debugs: HashMap<String, Instant>,
}

impl Default for TableDisplayState {
    fn default() -> Self {
        Self {
            last_debug: None,
            table_debugs: HashMap::new(),
        }
    }
}

#[derive(Resource, Default, Debug, Serialize)]
pub struct AppState {
    pub input_values: HashMap<String, String>,
    pub script_results: HashMap<String, String>,
    pub slider_values: HashMap<String, f32>,
    pub opened_file: Option<PathBuf>,
    pub script_tables: HashMap<String, TableData>,
    pub table_display_state: TableDisplayState,
}

impl AppState {
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}

#[derive(Resource, Default)]
pub struct ScriptOutputs {
    pub results: Vec<String>,
}

#[derive(Resource, Default)]
pub struct MarkdownCache {
    pub cache: CommonMarkCache,
}

#[derive(Resource, Default)]
pub struct UiState {
    pub left_selected_tab: String,
    pub right_selected_tab: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct RightPanelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_panel_width")]
    pub default_width: f32,
    #[serde(default)]
    pub tabs: Vec<TabConfig>,
}

pub fn default_panel_width() -> f32 { 0.3 }

#[derive(Deserialize, Debug, Default)]
pub struct TabConfig {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct DocsConfig {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub tab: String,
}

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct TableStyleConfig {
    #[serde(default)]
    pub striped: bool,
    #[serde(default)]
    pub borders: bool,
    pub header_background: Option<String>,
    #[serde(default = "default_row_height")]
    pub row_height: f32,
}

pub fn default_row_height() -> f32 { 30.0 }

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct TableConfig {
    #[serde(default)]
    pub tab: String,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub data: Vec<Vec<String>>,
    #[serde(default)]
    pub style: TableStyleConfig,
}

#[derive(Deserialize, Debug, Default)]
pub struct PanelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_panel_width")]
    pub default_width: f32,
    #[serde(default)]
    pub tabs: Vec<TabConfig>,
}
