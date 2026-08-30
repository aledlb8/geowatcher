//! `GeoWatcher`: watches Geometry Dash streams and alerts on real runs.

pub mod capture;
pub mod detect;
pub mod run;
pub mod twitch;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

/// Builds and runs the Tauri application.
///
/// # Panics
///
/// Panics if the webview or the platform event loop cannot be initialised.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
