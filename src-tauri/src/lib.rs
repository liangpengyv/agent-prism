mod billing;
mod commands;
mod data_source;
mod store;

use commands::{get_summary, get_threads, refresh};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_summary, get_threads, refresh])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
