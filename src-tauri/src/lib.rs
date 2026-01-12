mod obs;
mod sync;
mod network;
mod commands;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::connect_obs,
            commands::disconnect_obs,
            commands::get_obs_status,
            commands::set_app_mode,
            commands::get_app_mode,
            commands::start_master_server,
            commands::stop_master_server,
            commands::connect_to_master,
            commands::disconnect_from_master,
            commands::set_sync_targets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
