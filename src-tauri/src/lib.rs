pub mod asset;
pub mod assistant;
pub mod clock;
pub mod clone;
pub mod collection;
pub mod commands;
pub mod cut;
pub mod db;
pub mod device;
pub mod doors;
pub mod error;
pub mod exchange;
pub mod focus;
pub mod journal;
pub mod layout;
pub mod link;
pub mod mcp;
pub mod minted;
pub mod note;
pub mod operation;
pub mod plugin;
pub mod profile;
pub mod readiness;
pub mod release;
pub mod release_meta;
pub mod replay;
pub mod reversal;
pub mod scene;
pub mod scene_frame;
pub mod scene_note;
pub mod score;
pub mod search;
pub mod state;
pub mod style_brick;
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
        // Size, position and whether it was maximised come back on the next
        // start. Visibility is left out on purpose: the window is created
        // hidden and shown by the page once it has painted, and a restored
        // "visible" would show it a moment early, white.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all()
                        - tauri_plugin_window_state::StateFlags::VISIBLE,
                )
                .build(),
        )
        .setup(move |app| {
            // Per-user application data, resolved by Tauri for the current
            // platform — unless the command line named a directory.
            let data_dir = match &workspace {
                Some(dir) => dir.clone(),
                None => app.path().app_data_dir()?,
            };
            let state = state::AppState::open(&db::default_path(&data_dir))?;

            // The window may show the files the workspace holds, and only
            // those. The scope is granted here rather than in the config
            // because the directory is not known until now: `--workspace`
            // moves it, and a path written into `tauri.conf.json` would be
            // right for one workspace and wrong for the next.
            match state.media_dir() {
                Ok(media) => {
                    if let Err(cause) = app.asset_protocol_scope().allow_directory(&media, true) {
                        eprintln!(
                            "assets: the window may not read {}: {cause}",
                            media.display()
                        );
                    }
                }
                // Said, not fatal: everything but the pictures still works,
                // and refusing to start over a directory would be worse.
                Err(cause) => eprintln!("assets: no directory for files: {cause}"),
            }

            app.manage(state);

            // The page shows the window as soon as it has painted. Should it
            // never get that far - a broken bundle, a webview that failed to
            // load - the window must still appear, or the app would be
            // running with nothing to close. Three seconds is longer than any
            // healthy start and shorter than a person's patience.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                if let Some(window) = handle.get_webview_window("main") {
                    if !window.is_visible().unwrap_or(true) {
                        let _ = window.show();
                    }
                }
            });
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
            commands::resolve_links,
            commands::list_style_bricks,
            commands::style_brick_counts,
            commands::get_style_brick,
            commands::style_brick_references,
            commands::create_style_brick,
            commands::update_style_brick,
            commands::paste_style_reference,
            commands::describe_style_brick,
            commands::preview_style_task,
            commands::start_style_task,
            commands::delete_style_brick,
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
            commands::release_fields,
            commands::set_release_fields,
            commands::preview_release_fields,
            commands::generate_release_fields,
            commands::generate_release_fields_batch,
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
            commands::list_links,
            commands::create_link,
            commands::delete_link,
            commands::derive_work,
            commands::list_scenes,
            commands::create_scene,
            commands::update_scene,
            commands::delete_scene,
            commands::time_scenes,
            commands::renumber_scenes,
            commands::frame_scenes,
            commands::list_scene_notes,
            commands::attach_scene_note,
            commands::detach_scene_note,
            commands::list_cuts,
            commands::list_cuts_from,
            commands::create_cut,
            commands::update_cut,
            commands::reorder_cuts,
            commands::delete_cut,
            commands::cut_shot_list,
            commands::attach_asset,
            commands::list_work_assets,
            commands::list_release_assets,
            commands::list_covers,
            commands::detach_asset,
            commands::list_scene_frames,
            commands::attach_scene_frame,
            commands::paste_scene_frame,
            commands::detach_scene_frame,
            commands::select_scene_frame,
            commands::clear_scene_frame,
            commands::reorder_scene_frames,
            commands::search,
            commands::works_matching,
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
            commands::apply_proposal,
            commands::apply_pending_proposals,
            commands::pending_proposals,
            commands::dismiss_proposal,
            commands::ask_assistant,
            commands::start_run,
            commands::cancel_run,
            commands::list_runs,
            commands::active_runs,
            commands::start_task,
            commands::preview_task,
            commands::start_tasks,
            commands::active_tasks,
            commands::task_queue,
            commands::clear_task_queue,
            commands::waiting_chats,
            commands::clear_waiting,
            commands::render_prompt,
            commands::clone_work,
            commands::write_text_file,
            commands::export_markdown,
            commands::export_package,
            commands::can_export_package,
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
