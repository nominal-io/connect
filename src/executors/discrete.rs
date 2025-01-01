use crate::types::*;
use std::process::{Command, Stdio};
use std::path::Path;
use std::io::{Write, BufRead, BufReader};
use serde_json;

pub fn execute_script(
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
    
    if let Some(output) = spawn_and_run_script(script, function_name, &simplified_state, &script_path) {
        process_script_output(output, script, function_name, app_state, script_outputs);
    }
}

fn spawn_and_run_script(
    script: &ScriptConfig,
    function_name: Option<&str>,
    state: &serde_json::Value,
    script_path: &Path,
) -> Option<String> {
    let mut command = Command::new("python3");
    command.arg(script_path)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
    
    if let Some(func_name) = function_name {
        if !script.functions.is_empty() {
            println!("Executing function '{}' in script", func_name);
            command.arg("--function").arg(func_name);
        }
    }
    
    let mut child = command.spawn().ok()?;
    
    // Write state to stdin if needed
    if !script.functions.is_empty() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(state.to_string().as_bytes());
        }
    }

    // Handle stderr in real-time
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(error_line) = line {
                eprintln!("Script stderr: {}", error_line);
            }
        }
    }

    // Collect stdout
    let mut output = String::new();
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                output = line.trim().to_string();
            }
        }
    }

    // Wait for completion
    match child.wait() {
        Ok(status) if !status.success() => {
            println!("Script failed with status: {}", status);
            None
        },
        Err(e) => {
            println!("Failed to wait for script: {}", e);
            None
        },
        Ok(_) => Some(output),
    }
}

fn process_script_output(
    output: String,
    script: &ScriptConfig,
    function_name: Option<&str>,
    app_state: &mut AppState,
    script_outputs: &mut ScriptOutputs,
) {
    println!("Script output: {}", output);
    
    let result_key = if let Some(func_name) = function_name {
        format!("{}_{}", script.name, func_name)
    } else {
        script.name.clone()
    };

    println!("Attempting to parse as table data for key: {}", result_key);
    
    match serde_json::from_str::<TableData>(&output) {
        Ok(mut table_data) => {
            println!("Successfully parsed table data: {} columns, {} rows", 
                table_data.columns.len(), 
                table_data.data.len()
            );
            
            // Handle error if present
            let has_error = table_data.error.is_some();
            if let Some(error) = table_data.error.take() {
                println!("Table data contained error: {}", error);
                app_state.script_results.insert(result_key.clone(), error);
            }
            
            // Store table data if no error
            if !has_error {
                app_state.script_tables.insert(result_key, table_data);
                println!("Stored table data");
            }
        },
        Err(e) => {
            println!("Failed to parse as table data: {}", e);
            app_state.script_results.insert(result_key, output.clone());
        }
    }
    
    script_outputs.results.push(output);
}
