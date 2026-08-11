pub mod commands;
pub mod db;
pub mod error;
pub mod profile;
pub mod state;
pub mod time;

pub use error::{Error, Result};

use tauri::Manager;

/// Build and run the desktop application.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Per-user application data, resolved by Tauri for the current platform.
            let data_dir = app.path().app_data_dir()?;
            let state = state::AppState::open(&db::default_path(&data_dir))?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_workspace,
            commands::list_profiles,
            commands::activate_profile,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start kilna");
}
