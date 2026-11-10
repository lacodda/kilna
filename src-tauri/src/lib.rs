pub mod actions;
pub mod activity;
pub mod asset;
pub mod assistant;
pub mod board;
pub mod canon;
pub mod card;
pub mod clock;
pub mod clone;
pub mod collection;
pub mod commands;
pub mod comment;
pub mod conception;
pub mod cover;
pub mod cut;
pub mod db;
pub mod device;
pub mod donor_map;
pub mod error;
pub mod exchange;
#[doc(hidden)]
pub mod fixtures;
pub mod focus;
pub mod folder;
pub mod journal;
pub mod lab;
pub mod layout;
pub mod library;
pub mod lines;
pub mod link;
pub mod log;
pub mod mcp;
pub mod minted;
pub mod note;
pub mod operation;
pub mod ordinal;
pub mod parts;
pub mod phrase;
pub mod plan_import;
pub mod plugin;
pub mod profile;
pub mod publication;
pub mod readiness;
pub mod register;
pub mod release;
pub mod release_meta;
pub mod replay;
pub mod report;
pub mod reversal;
pub mod scene;
pub mod scene_frame;
pub mod scene_note;
pub mod score;
pub mod search;
pub mod shelf;
pub mod space;
pub mod state;
pub mod style_brick;
pub mod style_set;
pub mod time;
pub mod time_off;
pub mod tombstone;
pub mod trash;
pub mod tray;
pub mod undo;
pub mod words;
pub mod work;

pub use error::{Error, Result};

use tauri::Manager;

/// Build and run the desktop application on the usual workspace.
pub fn run() {
    run_in(None, None)
}

/// The event the window hears something to open by: a `space.kilna` double-
/// clicked or a `kilna://` link followed while kilna is already open.
pub const OPEN_EVENT: &str = "app:open";

/// Build and run the desktop application, on `workspace` - an application
/// directory - when one is given, opening `target` - a space's file or a link -
/// once it stands.
///
/// The override exists for the same reason `--mcp --workspace` does: a live
/// run on a *copy* of a real workspace, before a change that migrates it is
/// let near the original.
pub fn run_in(workspace: Option<std::path::PathBuf>, target: Option<String>) {
    let mut builder = tauri::Builder::default();
    // kilna started again while it is open - or waiting behind the tray -
    // brings that window forward instead of opening a second one on the same
    // workspace (v0.107), and opens the space or the link it was started on.
    // First, as the plugin asks: the second process ends inside it. Not for a
    // run on another directory (`--workspace`): that is a rehearsal on a
    // copy, and must stand beside the real one.
    if workspace.is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            tray::show_main(app);
            // A link arrives through the deep-link plugin, which the single
            // instance hands it to; a space's file arrives here.
            if let Some(target) = commands::spaces::target_in(&args)
                && commands::spaces::parse_link(&target).is_none()
            {
                use tauri::Emitter;
                let _ = app.emit(OPEN_EVENT, target);
            }
        }));
    }
    builder
        // `kilna://<space>/work/<id>` opens a work from anywhere a link can
        // be followed (#15); the scheme is registered by the installer.
        .plugin(tauri_plugin_deep_link::init())
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
        // A task that ended, a release due tomorrow, a week over its budget,
        // said outside the window when the window is not in front (v0.107).
        .plugin(tauri_plugin_notification::init())
        .manage(tray::Words::default())
        // Closed with something going, the window hides behind the tray.
        .on_window_event(tray::close_requested)
        .setup(move |app| {
            // The application's own directory - beside the executable for a
            // portable copy, per-user data otherwise - unless the command
            // line named one.
            let data_dir = match &workspace {
                Some(dir) => dir.clone(),
                None => space::app_dir()?,
            };
            // First, so that anything the opening says lands in the file.
            log::init(&data_dir, log::APP_FILE);

            // A space's file the start was handed opens straight away; a link
            // waits for the window, which goes where it points.
            let mut link = target
                .clone()
                .filter(|target| commands::spaces::parse_link(target).is_some());
            let requested = target
                .as_deref()
                .filter(|target| commands::spaces::parse_link(target).is_none())
                .map(std::path::PathBuf::from);
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if link.is_none()
                    && let Ok(Some(urls)) = app.deep_link().get_current()
                {
                    link = urls.first().map(ToString::to_string);
                }
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    use tauri::Emitter;
                    tray::show_main(&handle);
                    for url in event.urls() {
                        let _ = handle.emit(OPEN_EVENT, url.to_string());
                    }
                });
            }
            app.manage(commands::spaces::Launch(std::sync::Mutex::new(link)));

            let state = match requested {
                Some(file) => state::AppState::start(&data_dir, Some(&file)).or_else(|cause| {
                    log::error(
                        "space",
                        &format!("could not open {}: {cause}", file.display()),
                    );
                    state::AppState::start(&data_dir, None)
                })?,
                None => state::AppState::start(&data_dir, None)?,
            };

            // The window may show the files the space holds, and only those -
            // and the media folders this machine keeps for it: a work's folder
            // on disk is looked at where it lies (ADR 0057). Granted here and
            // on every switch, because which folders those are is known only
            // once a space is open.
            commands::spaces::grant(app.handle(), &state);

            app.manage(state);
            // The runs and the queue say when they change, and the window
            // hears the count: its close button, its badge and the tray's.
            commands::activity::watch(app.handle());
            // A click on a notification brings the window back; what it then
            // opens is the page's, which hears the same click.
            {
                use tauri_plugin_notification::NotificationExt;
                let handle = app.handle().clone();
                let _ = app.notification().on_action(move |performed| {
                    if performed.action_id() == "tap" {
                        tray::show_main(&handle);
                    }
                });
            }

            // The window says, once a minute, that it still holds its space
            // (`kilna.lock`): another machine finding the hold fresh opens the
            // space for reading only.
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    loop {
                        std::thread::sleep(space::lock::BEAT_EVERY);
                        handle.state::<state::AppState>().beat();
                    }
                });
            }

            // The page shows the window as soon as it has painted. Should it
            // never get that far - a broken bundle, a webview that failed to
            // load - the window must still appear, or the app would be
            // running with nothing to close. Three seconds is longer than any
            // healthy start and shorter than a person's patience. A window
            // closed behind the tray in those seconds stays there (wish 6659).
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                if let Some(window) = handle.get_webview_window("main")
                    && tray::watchdog_shows(window.is_visible().unwrap_or(true), tray::put_away())
                {
                    let _ = window.show();
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::workspace::get_workspace,
            commands::spaces::list_spaces,
            commands::spaces::current_space,
            commands::spaces::create_space,
            commands::spaces::open_space,
            commands::spaces::rename_space,
            commands::spaces::restyle_space,
            commands::spaces::trash_space,
            commands::spaces::restore_space,
            commands::spaces::forget_space,
            commands::spaces::take_launch_target,
            commands::spaces::open_target,
            commands::workspace::update_profile_config,
            commands::workspace::workspace_path,
            commands::workspace::log_path,
            commands::workspace::log_window_error,
            commands::activity::activity,
            tray::set_tray_words,
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
            commands::versions::set_main_version,
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
            commands::canon::settle_section,
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
            commands::assistant::preview_lab_task,
            commands::assistant::start_lab_task,
            commands::assistant::preview_board_task,
            commands::assistant::start_board_task,
            commands::assistant::stop_task,
            commands::notes::list_notes,
            commands::notes::list_lines,
            commands::notes::keep_lines,
            commands::notes::list_shelves,
            commands::notes::create_shelf,
            commands::notes::rename_shelf,
            commands::notes::move_shelf,
            commands::notes::delete_shelf,
            commands::notes::add_to_shelf,
            commands::notes::remove_from_shelf,
            commands::notes::retag_notes,
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
            commands::boards::work_boards,
            commands::boards::add_own_card,
            commands::boards::update_card,
            commands::boards::judge_card,
            commands::boards::move_card,
            commands::boards::retopic_cards,
            commands::boards::delete_card,
            commands::boards::chip_to_phrase,
            commands::boards::entry_worked_in,
            commands::boards::conception_order,
            commands::trials::trial_board,
            commands::trials::create_trial,
            commands::trials::update_trial,
            commands::trials::judge_trial,
            commands::trials::vary_trial,
            commands::trials::trial_from_version,
            commands::trials::delete_trial,
            commands::trials::harvest_trial,
            commands::trials::harvest_trial_phrase,
            commands::trials::harvest_trial_work,
            commands::trials::make_experiment,
            commands::styles::list_style_bricks,
            commands::styles::style_brick_counts,
            commands::styles::style_brick_references,
            commands::styles::create_style_brick,
            commands::styles::update_style_brick,
            commands::styles::paste_style_reference,
            commands::styles::delete_style_brick,
            commands::styles::restore_style_brick,
            commands::styles::style_slot_values,
            commands::styles::house_styles,
            commands::styles::compose_text,
            commands::styles::phrases_from_texts,
            commands::focus::dismissed_findings,
            commands::focus::dismiss_finding,
            commands::focus::restore_finding,
            commands::focus::list_focus_notes,
            commands::focus::week_report,
            commands::focus::since_visit,
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
            commands::releases::read_plan_import,
            commands::releases::apply_layout,
            commands::releases::plan_shift,
            commands::releases::apply_shift,
            commands::releases::pin_range,
            commands::time_off::list_time_off,
            commands::time_off::create_time_off,
            commands::time_off::update_time_off,
            commands::time_off::delete_time_off,
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
            commands::cuts::cut_parts,
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
            commands::assistant::assistant_settings,
            commands::assistant::set_parallel_runs,
            commands::assistant::set_run_model,
            commands::assistant::set_run_sees,
            commands::assistant::run_sight,
            commands::assistant::waiting_scores,
            commands::assistant::mcp_registration,
            commands::assistant::list_chat_summaries,
            commands::assistant::create_chat,
            commands::assistant::rename_chat,
            commands::assistant::get_transcript,
            commands::assistant::delete_chat,
            commands::assistant::star_answer,
            commands::assistant::starred_answers,
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
            commands::assistant::preview_phrases_task,
            commands::assistant::start_phrases_task,
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
            commands::assistant::assistant_spend,
            commands::assistant::check_spend,
            commands::assistant::set_weekly_budget,
            commands::data::write_text_file,
            commands::data::export_markdown,
            commands::data::chat_markdown,
            commands::data::export_package,
            commands::data::can_export_package,
            commands::data::backup_workspace,
            commands::data::suggested_backup_name,
            commands::data::import_legacy,
            commands::plugins::list_plugins,
            commands::plugins::plugins_folder,
            commands::plugins::set_plugin_enabled,
            commands::plugins::set_plugin_setting,
            commands::plugins::run_plugin,
            commands::plugins::cancel_plugin,
        ])
        .build(tauri::generate_context!())
        .expect("failed to start kilna")
        .run(|app, event| {
            // Assistant runs are separate processes, and they outlive the
            // window unless they are stopped: a run left going answers into a
            // workspace nobody is reading, on the user's tokens. Exit is the
            // last chance to end them.
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                // The space is let go, so another machine need not wait for
                // the hold to go stale.
                app.state::<state::AppState>().release();
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
