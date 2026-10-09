pub mod actions;
pub mod asset;
pub mod assistant;
pub mod canon;
pub mod card;
pub mod clock;
pub mod clone;
pub mod collection;
pub mod commands;
pub mod comment;
pub mod cover;
pub mod cut;
pub mod db;
pub mod device;
pub mod error;
pub mod exchange;
#[doc(hidden)]
pub mod fixtures;
pub mod focus;
pub mod folder;
pub mod journal;
pub mod layout;
pub mod link;
pub mod log;
pub mod mcp;
pub mod minted;
pub mod note;
pub mod operation;
pub mod plugin;
pub mod profile;
pub mod publication;
pub mod readiness;
pub mod register;
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
pub mod style_set;
pub mod time;
pub mod tombstone;
pub mod trash;
pub mod undo;
pub mod words;
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
            // First, so that anything the opening says lands in the file.
            log::init(&data_dir, log::APP_FILE);
            let state = state::AppState::open(&db::default_path(&data_dir))?;

            // The window may show the files the workspace holds, and only
            // those. The scope is granted here rather than in the config
            // because the directory is not known until now: `--workspace`
            // moves it, and a path written into `tauri.conf.json` would be
            // right for one workspace and wrong for the next.
            match state.media_dir() {
                Ok(media) => {
                    if let Err(cause) = app.asset_protocol_scope().allow_directory(&media, true) {
                        log::error(
                            "assets",
                            &format!("the window may not read {}: {cause}", media.display()),
                        );
                    }
                }
                // Said, not fatal: everything but the pictures still works,
                // and refusing to start over a directory would be worse.
                Err(cause) => log::error("assets", &format!("no directory for files: {cause}")),
            }
            // And the media folders this machine keeps for the workspace's
            // profiles: a work's folder on disk is looked at where it lies
            // (ADR 0057), so the window has to be let in there too.
            match folder::roots(&state.conn()) {
                Ok(roots) => {
                    for root in roots {
                        commands::folders::allow(app.handle(), &root);
                    }
                }
                Err(cause) => log::error("folder", &format!("no media folders read: {cause}")),
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
                if let Some(window) = handle.get_webview_window("main")
                    && !window.is_visible().unwrap_or(true)
                {
                    let _ = window.show();
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::workspace::get_workspace,
            commands::workspace::list_profiles,
            commands::workspace::activate_profile,
            commands::workspace::update_profile_config,
            commands::workspace::workspace_path,
            commands::workspace::log_path,
            commands::workspace::log_window_error,
            commands::works::list_works,
            commands::works::get_work,
            commands::works::create_work,
            commands::works::update_work,
            commands::works::cover_view,
            commands::works::frame_view,
            commands::works::status_drift,
            commands::works::resync_statuses,
            commands::works::unpin_status,
            commands::works::pin_tier,
            commands::works::unpin_tier,
            commands::works::delete_work,
            commands::works::delete_works,
            commands::works::set_works_status,
            commands::works::number_works,
            commands::works::work_tags,
            commands::works::catalogue,
            commands::works::card_counts,
            commands::works::work_timeline,
            commands::works::clone_work,
            commands::versions::list_versions,
            commands::versions::get_version,
            commands::versions::create_version,
            commands::versions::update_version_body,
            commands::versions::set_current_version,
            commands::versions::delete_version,
            commands::canon::list_cards,
            commands::canon::read_card,
            commands::canon::card_as_seen,
            commands::canon::canon_timeline,
            commands::canon::expand_card_references,
            commands::canon::create_card,
            commands::canon::describe_card,
            commands::canon::add_fact,
            commands::canon::update_fact,
            commands::canon::retire_fact,
            commands::canon::reorder_facts,
            commands::canon::delete_fact,
            commands::canon::relate_cards,
            commands::canon::update_relation,
            commands::canon::unrelate_cards,
            commands::canon::paste_card_picture,
            commands::canon::set_picture_role,
            commands::canon::pending_canon_proposals,
            commands::canon::review_canon_proposal,
            commands::assistant::preview_card_task,
            commands::assistant::start_card_task,
            commands::assistant::preview_cover_task,
            commands::assistant::start_cover_task,
            commands::assistant::stop_task,
            commands::notes::list_notes,
            commands::notes::create_note,
            commands::notes::update_note,
            commands::notes::delete_note,
            commands::notes::promote_note,
            commands::notes::list_tags,
            commands::notes::resolve_links,
            commands::register::list_terms,
            commands::register::term_uses,
            commands::register::preview_term,
            commands::register::list_term_topics,
            commands::register::create_term,
            commands::register::update_term,
            commands::register::delete_term,
            commands::register::link_term,
            commands::register::unlink_term,
            commands::register::check_text,
            commands::register::clean_text,
            commands::register::list_blocks,
            commands::register::create_block,
            commands::register::rename_block,
            commands::register::move_block,
            commands::register::delete_block,
            commands::register::add_to_block,
            commands::register::remove_from_block,
            commands::register::bank_words,
            commands::register::words_from_texts,
            commands::register::keep_words,
            commands::register::repeat_marks,
            commands::register::work_repeats,
            commands::register::keep_repeat,
            commands::register::unkeep_repeat,
            commands::comments::list_comments,
            commands::comments::comment_channels,
            commands::comments::create_comment,
            commands::comments::update_comment,
            commands::comments::delete_comment,
            commands::ideas::cover_board,
            commands::ideas::add_own_idea,
            commands::ideas::judge_idea,
            commands::ideas::judge_sibling_cover,
            commands::ideas::take_idea,
            commands::ideas::take_sibling_cover,
            commands::ideas::delete_idea,
            commands::styles::list_style_bricks,
            commands::styles::style_brick_counts,
            commands::styles::style_brick_references,
            commands::styles::create_style_brick,
            commands::styles::update_style_brick,
            commands::styles::paste_style_reference,
            commands::styles::delete_style_brick,
            commands::styles::restore_style_brick,
            commands::styles::style_slot_values,
            commands::focus::dismissed_findings,
            commands::focus::dismiss_finding,
            commands::focus::restore_finding,
            commands::focus::list_focus_notes,
            commands::focus::create_focus_note,
            commands::focus::update_focus_note,
            commands::focus::reorder_focus_notes,
            commands::focus::delete_focus_note,
            commands::scores::score_work,
            commands::scores::score_history,
            commands::scores::latest_score,
            commands::scores::kind_verdicts,
            commands::scores::delete_score,
            commands::releases::create_release,
            commands::releases::update_release,
            commands::releases::delete_release,
            commands::releases::release_fields,
            commands::releases::set_release_fields,
            commands::releases::preview_release_fields,
            commands::releases::generate_release_fields,
            commands::releases::generate_release_fields_batch,
            commands::releases::preview_schedule,
            commands::releases::warn_unready_releases,
            commands::releases::schedule_release,
            commands::releases::set_slot_pin,
            commands::releases::unschedule_release,
            commands::releases::unschedule_works,
            commands::releases::mark_released,
            commands::releases::unmark_released,
            commands::releases::calendar,
            commands::releases::plan_layout,
            commands::releases::apply_layout,
            commands::releases::release_queue,
            commands::releases::releases_for_work,
            commands::collections::list_collections,
            commands::collections::create_collection,
            commands::collections::update_collection,
            commands::collections::delete_collection,
            commands::collections::set_collection_contents,
            commands::collections::add_to_collection,
            commands::links::list_links,
            commands::links::create_link,
            commands::links::delete_link,
            commands::links::derive_work,
            commands::links::list_publications,
            commands::scenes::list_scenes,
            commands::scenes::create_scene,
            commands::scenes::update_scene,
            commands::scenes::delete_scene,
            commands::scenes::time_scenes,
            commands::scenes::renumber_scenes,
            commands::scenes::frame_scenes,
            commands::scenes::list_scene_notes,
            commands::scenes::attach_scene_note,
            commands::scenes::detach_scene_note,
            commands::scenes::list_scene_frames,
            commands::scenes::attach_scene_frame,
            commands::scenes::paste_scene_frame,
            commands::scenes::detach_scene_frame,
            commands::scenes::scene_frame_view,
            commands::scenes::select_scene_frame,
            commands::scenes::clear_scene_frame,
            commands::scenes::reorder_scene_frames,
            commands::cuts::list_cuts,
            commands::cuts::list_cuts_from,
            commands::cuts::create_cut,
            commands::cuts::update_cut,
            commands::cuts::reorder_cuts,
            commands::cuts::delete_cut,
            commands::cuts::cut_shot_list,
            commands::assets::attach_asset,
            commands::assets::list_work_assets,
            commands::assets::list_release_assets,
            commands::assets::list_covers,
            commands::assets::detach_asset,
            commands::assets::paste_asset,
            commands::assets::choose_cover,
            commands::assets::asset_bytes,
            commands::assets::save_picture,
            commands::folders::media_root,
            commands::folders::set_media_root,
            commands::folders::work_folder,
            commands::folders::create_work_folder,
            commands::folders::media_directory,
            commands::folders::open_media,
            commands::search::search,
            commands::search::works_matching,
            commands::journal::list_journal,
            commands::journal::unread_journal,
            commands::journal::mark_journal_read,
            commands::trash::list_deletions,
            commands::trash::restore_deletion,
            commands::trash::purge_deletion,
            commands::trash::empty_trash,
            commands::trash::last_undoable,
            commands::trash::undo_last,
            commands::assistant::assistant_status,
            commands::assistant::mcp_registration,
            commands::assistant::list_chat_summaries,
            commands::assistant::create_chat,
            commands::assistant::rename_chat,
            commands::assistant::get_transcript,
            commands::assistant::delete_chat,
            commands::assistant::apply_proposal,
            commands::assistant::apply_pending_proposals,
            commands::assistant::pending_proposals,
            commands::assistant::pending_comment_proposals,
            commands::assistant::dismiss_proposal,
            commands::assistant::start_run,
            commands::assistant::cancel_run,
            commands::assistant::list_runs,
            commands::assistant::active_runs,
            commands::assistant::start_task,
            commands::assistant::preview_task,
            commands::assistant::preview_style_task,
            commands::assistant::start_style_task,
            commands::assistant::preview_comment_task,
            commands::assistant::start_comment_task,
            commands::assistant::preview_release_task,
            commands::assistant::start_release_task,
            commands::assistant::release_proposals,
            commands::assistant::start_screenshot_task,
            commands::assistant::waiting_chats,
            commands::assistant::clear_waiting,
            commands::assistant::active_tasks,
            commands::assistant::start_tasks,
            commands::assistant::task_queue,
            commands::assistant::clear_task_queue,
            commands::assistant::render_prompt,
            commands::data::write_text_file,
            commands::data::export_markdown,
            commands::data::export_package,
            commands::data::can_export_package,
            commands::data::backup_workspace,
            commands::data::suggested_backup_name,
            commands::data::import_legacy,
            commands::plugins::list_plugins,
            commands::plugins::run_plugin,
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
                    log::info(
                        "assistant",
                        &format!("stopped {stopped} run(s) still going at exit"),
                    );
                }
            }
        });
}
