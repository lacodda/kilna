pub mod assistant;
pub mod clock;
pub mod collection;
pub mod commands;
pub mod db;
pub mod device;
pub mod error;
pub mod exchange;
pub mod focus;
pub mod journal;
pub mod layout;
pub mod mcp;
pub mod minted;
pub mod note;
pub mod operation;
pub mod plugin;
pub mod profile;
pub mod readiness;
pub mod release;
pub mod replay;
pub mod reversal;
pub mod score;
pub mod search;
pub mod state;
pub mod time;
pub mod tombstone;
pub mod trash;
pub mod undo;
pub mod work;

pub use error::{Error, Result};

use tauri::Manager;

/// Build and run the desktop application on the usual workspace.
pub fn run() {
    run_in(None)
}

/// Build and run the desktop application, on `workspace` when one is given.
///
/// The override exists for the same reason `--mcp --workspace` does: a live
/// run on a *copy* of a real workspace, before a change that migrates it is
/// let near the original.
pub fn run_in(workspace: Option<std::path::PathBuf>) {
    tauri::Builder::default()
        // Needed by the data screen to pick a directory or a file.
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            // Per-user application data, resolved by Tauri for the current
            // platform — unless the command line named a directory.
            let data_dir = match &workspace {
                Some(dir) => dir.clone(),
                None => app.path().app_data_dir()?,
            };
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
            commands::pin_tier,
            commands::unpin_tier,
            commands::delete_work,
            commands::delete_works,
            commands::set_works_status,
            commands::list_versions,
            commands::get_version,
            commands::create_version,
            commands::update_version_body,
            commands::set_current_version,
            commands::delete_version,
            commands::list_notes,
            commands::create_note,
            commands::update_note,
            commands::delete_note,
            commands::list_tags,
            commands::work_tags,
            commands::dismissed_findings,
            commands::dismiss_finding,
            commands::restore_finding,
            commands::list_focus_notes,
            commands::create_focus_note,
            commands::update_focus_note,
            commands::reorder_focus_notes,
            commands::delete_focus_note,
            commands::score_work,
            commands::score_history,
            commands::latest_score,
            commands::kind_verdicts,
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
            commands::unschedule_works,
            commands::mark_released,
            commands::unmark_released,
            commands::calendar,
            commands::plan_layout,
            commands::apply_layout,
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
            commands::last_undoable,
            commands::undo_last,
            commands::assistant_status,
            commands::mcp_registration,
            commands::list_chat_summaries,
            commands::create_chat,
            commands::rename_chat,
            commands::get_transcript,
            commands::delete_chat,
            commands::ask_assistant,
            commands::start_run,
            commands::cancel_run,
            commands::list_runs,
            commands::active_runs,
            commands::start_task,
            commands::start_tasks,
            commands::active_tasks,
            commands::task_queue,
            commands::clear_task_queue,
            commands::waiting_chats,
            commands::clear_waiting,
            commands::render_prompt,
            commands::export_markdown,
            commands::backup_workspace,
            commands::suggested_backup_name,
            commands::workspace_path,
            commands::import_legacy,
            commands::list_plugins,
            commands::run_plugin,
        ])
        .build(tauri::generate_context!())
        .expect("failed to start kilna")
        .run(|app, event| {
            // Assistant runs are separate processes, and they outlive the
            // window unless they are stopped: a run left going answers into a
            // workspace nobody is reading, on the user's tokens. Exit is the
            // last chance to end them.
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                let stopped = app.state::<state::AppState>().runs().stop_all();
                if stopped > 0 {
                    eprintln!("assistant: stopped {stopped} run(s) still going at exit");
                }
            }
        });
}
