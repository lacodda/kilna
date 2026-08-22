pub mod assistant;
pub mod collection;
pub mod commands;
pub mod db;
pub mod error;
pub mod exchange;
pub mod journal;
pub mod note;
pub mod plugin;
pub mod profile;
pub mod readiness;
pub mod release;
pub mod score;
pub mod search;
pub mod state;
pub mod time;
pub mod trash;
pub mod work;

pub use error::{Error, Result};

use tauri::Manager;

/// Build and run the desktop application.
pub fn run() {
    tauri::Builder::default()
        // Needed by the data screen to pick a directory or a file.
        .plugin(tauri_plugin_dialog::init())
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
            commands::update_profile_config,
            commands::list_works,
            commands::get_work,
            commands::create_work,
            commands::update_work,
            commands::status_drift,
            commands::resync_statuses,
            commands::unpin_status,
            commands::delete_work,
            commands::list_versions,
            commands::get_version,
            commands::create_version,
            commands::set_current_version,
            commands::delete_version,
            commands::list_notes,
            commands::create_note,
            commands::update_note,
            commands::delete_note,
            commands::list_tags,
            commands::score_work,
            commands::score_history,
            commands::latest_score,
            commands::delete_score,
            commands::catalogue,
            commands::create_release,
            commands::update_release,
            commands::delete_release,
            commands::preview_schedule,
            commands::warn_unready_releases,
            commands::schedule_release,
            commands::set_slot_pin,
            commands::unschedule_release,
            commands::mark_released,
            commands::calendar,
            commands::release_queue,
            commands::releases_for_work,
            commands::list_collections,
            commands::create_collection,
            commands::update_collection,
            commands::delete_collection,
            commands::set_collection_contents,
            commands::search,
            commands::list_journal,
            commands::journal_for_work,
            commands::unread_journal,
            commands::mark_journal_read,
            commands::list_deletions,
            commands::restore_deletion,
            commands::purge_deletion,
            commands::empty_trash,
            commands::assistant_status,
            commands::list_chats,
            commands::create_chat,
            commands::get_transcript,
            commands::delete_chat,
            commands::ask_assistant,
            commands::render_prompt,
            commands::export_markdown,
            commands::backup_workspace,
            commands::suggested_backup_name,
            commands::workspace_path,
            commands::import_legacy,
            commands::list_plugins,
            commands::run_plugin,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start kilna");
}
