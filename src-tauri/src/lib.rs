mod commands;
mod network;
mod obs;
mod sync;

use commands::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();

    // Initialize logging
    let log_dir = std::env::temp_dir().join("obs-sync-logs");
    std::fs::create_dir_all(&log_dir).ok();
    let log_file = log_dir.join(format!(
        "obs-sync-{}.log",
        chrono::Utc::now().format("%Y-%m-%d")
    ));

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .expect("Failed to open log file");

    tracing_subscriber::fmt()
        .with_writer(file)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(false)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(|app| {
            let handle = app.handle().clone();
            let state: tauri::State<AppState> = app.state();
            let state_inner = state.inner().clone();
            tauri::async_runtime::spawn(async move {
                state_inner.set_app_handle(handle).await;
            });
            Ok(())
        })
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
            commands::is_slave_connected,
            commands::set_sync_targets,
            commands::get_connected_clients_count,
            commands::get_connected_clients_info,
            commands::get_slave_statuses,
            commands::get_obs_sources,
            commands::get_slave_reconnection_status,
            commands::resync_all_slaves,
            commands::resync_specific_slave,
            commands::request_resync_from_master,
            commands::save_settings,
            commands::load_settings,
            commands::get_log_file_path,
            commands::open_log_file,
            commands::get_performance_metrics,
            commands::get_local_ip_address,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
