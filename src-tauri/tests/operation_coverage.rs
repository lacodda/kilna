//! Holds every write the window can cause to the operations log.
//!
//! A write that records no operation leaves a hole in the log, and a log with
//! a hole replays to a database that never existed — silently, because nothing
//! at the time of the missing write goes wrong. The failure surfaces much
//! later, in a merge or a repair, as data that disagrees with itself.
//!
//! Since v0.83 the rule has two halves (ADR 0040). A command writes nothing
//! itself: it hands a write to an action. And an action writes only inside a
//! gesture, which records its operation by construction - the gesture records
//! after its closure returns, so the only way to record nothing is to say so.
//! This gate reads both halves from the source rather than running them, for
//! the reason `journal_keys.rs` gives: a command no test happens to call
//! contributes nothing, and that is exactly the command most likely to have
//! been forgotten.

mod common;

use std::collections::BTreeSet;

use common::{Source, backend, gestures, matching_brace, the_function};

/// One `#[tauri::command]` function, as the scan sees it.
struct Command {
    name: String,
    body: String,
}

/// Every command in the backend, with the text of its body.
///
/// Found by the attribute, in whichever file it stands: the commands were one
/// file until v0.83, and a gate that opened that file by name would have read
/// nothing - and passed - the day they were split by domain.
///
/// The body is the function's own block, from its opening brace to the brace
/// that closes it, read in code with comments and literals blanked. It used to
/// run to the next `#[tauri::command]`, which took in whatever helpers sat
/// between, so two commands "recorded an operation" while writing past the
/// log - and a doc comment mentioning a write counted as one.
fn commands_in(sources: &[Source]) -> Vec<Command> {
    // Both spellings: `#[tauri::command]` and `#[tauri::command(async)]`,
    // which runs a command off the main thread.
    let marker = "#[tauri::command";

    let mut found = Vec::new();
    for file in sources {
        let code = &file.code;
        for (start, _) in code.match_indices(marker) {
            let Some(at) = code[start..].find("pub fn ").map(|at| start + at) else {
                continue;
            };
            let name_start = at + "pub fn ".len();
            let Some(open_paren) = code[name_start..].find('(').map(|at| name_start + at) else {
                continue;
            };
            let name = code[name_start..open_paren].trim().to_owned();

            // The first brace after the signature opens the body.
            let Some(open) = code[open_paren..].find('{').map(|at| open_paren + at) else {
                continue;
            };
            let close = matching_brace(code, open);
            found.push(Command {
                name,
                body: code[open..close].to_owned(),
            });
        }
    }

    assert!(
        !found.is_empty(),
        "no `#[tauri::command]` functions found — this test has stopped testing anything"
    );
    found
}

fn commands() -> Vec<Command> {
    commands_in(&backend())
}

/// Every command the application registers, from `generate_handler!`.
///
/// The one invocation in the backend, wherever it is; each entry by its last
/// segment, so `commands::works::create_work` names `create_work`.
fn registered() -> BTreeSet<String> {
    let sources = backend();
    let places: Vec<(&Source, usize)> = sources
        .iter()
        .flat_map(|file| {
            file.code
                .match_indices("generate_handler![")
                .map(move |(at, _)| (file, at))
        })
        .collect();
    assert_eq!(
        places.len(),
        1,
        "expected the commands to be registered in exactly one `generate_handler!`, found {}",
        places.len()
    );
    let (file, at) = places[0];
    let start = at + "generate_handler![".len();
    let end = file.code[start..]
        .find(']')
        .map_or(file.code.len(), |offset| start + offset);
    file.code[start..end]
        .split(',')
        .filter_map(|entry| entry.trim().rsplit("::").next().map(str::to_owned))
        .filter(|name| !name.is_empty())
        .collect()
}

/// The scanner's own watchdog: it has to find exactly the commands the
/// application registers. A scan that finds fewer passes everything it missed
/// without a word - the way a source scanner goes blind.
#[test]
fn the_scan_finds_every_registered_command() {
    let scanned: BTreeSet<String> = commands().into_iter().map(|command| command.name).collect();
    let registered = registered();
    assert!(
        registered.len() >= 150,
        "read only {} registered commands",
        registered.len()
    );
    assert_eq!(
        scanned, registered,
        "the commands found by their attribute and those registered in `generate_handler!` disagree"
    );
}

/// A domain call that writes, matched on `::name(` so that a local helper of
/// the same name does not count. Listed rather than inferred because "writes"
/// is not visible in a name: `score::catalogue` reads, `work::pin_tier_at`
/// writes.
const WRITERS: [&str; 66] = [
    "::create(",
    "::create_minted(",
    "::update(",
    "::update_at(",
    "::update_body_at(",
    "::delete(",
    "::discard(",
    "::discard_minted(",
    "::discard_batch(",
    // Taking something back changes the workspace as surely as doing it.
    "::undo(",
    "::restore(",
    "::purge(",
    "::empty(",
    "::activate(",
    "::update_config(",
    "::update_config_at(",
    "::add_note(",
    "::add_note_minted(",
    "::update_note(",
    "::update_note_at(",
    "::delete_note(",
    "::reorder(",
    "::reorder_notes(",
    "::set_contents(",
    // A machine's own record, written past the log on purpose - and seen
    // here so that the exemption saying so is held to a write.
    "::set_root(",
    "::set_contents_at(",
    "::set_current(",
    "::resync(",
    "::resync_at(",
    "::apply(",
    "::apply_at(",
    "::mark_released(",
    "::mark_released_at(",
    "::unmark_released(",
    "::unmark_released_at(",
    "::schedule(",
    "::schedule_at(",
    "::unschedule(",
    "::unschedule_at(",
    "::dismiss(",
    "::dismiss_at(",
    "::unpin(",
    "::unpin_at(",
    "::pin_tier(",
    "::pin_tier_at(",
    "::unpin_tier(",
    "::unpin_tier_at(",
    "::set_slot_pin(",
    "::set_slot_pin_at(",
    "::set_status(",
    "::run(",
    "::promote(",
    "::attach(",
    "::attach_minted(",
    "::attach_bytes(",
    "::detach(",
    "::select(",
    "::clear_selection(",
    "::set_role(",
    "::frame_from_text(",
    "::time_board_at(",
    "::renumber(",
    "::clone_work_minted(",
    "::from_legacy(",
    // A chat's own rows: written, but by design not into the log.
    "::rename(",
    "::clear_waiting(",
];

/// Where each writer is called in a piece of code, with the path it is called
/// through: `actions::work::create` for `actions::work::create(`.
fn writer_calls(code: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for writer in WRITERS {
        for (at, _) in code.match_indices(writer) {
            // The whole path the call is made through, read backwards over
            // the path's characters.
            let start = code[..at]
                .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
                .map_or(0, |i| i + 1);
            let path = &code[start..at + writer.len() - 1];
            found.push((at, path.to_owned()));
        }
    }
    found
}

/// Whether a call goes through the actions: from a command, `actions::…`;
/// from inside an action, a neighbouring action through `super::…`.
fn through_an_action(path: &str) -> bool {
    path.starts_with("actions::")
        || path.starts_with("crate::actions::")
        || path.starts_with("super::")
}

/// Commands that write past the actions, on purpose.
///
/// Each entry is a promise that replaying the log without its write still
/// produces the right database, and says why. Anything not on this list that
/// writes has to hand the write to an action.
const WRITES_PAST_THE_ACTIONS: [(&str, &str); 9] = [
    (
        "activate_profile",
        "which profile is open is a fact about this machine; ADR 0012 keeps it off the wire \
         for the same reason",
    ),
    (
        "create_chat",
        "a chat is this device's conversation with the assistant, not part of the workspace \
         a replay rebuilds",
    ),
    ("rename_chat", "as above"),
    ("delete_chat", "as above"),
    ("clear_waiting", "as above"),
    (
        "dismiss_proposal",
        "marks one chat message as turned down and writes nothing else; a chat is this \
         device's conversation",
    ),
    (
        "set_media_root",
        "where a workspace keeps its media is a place on this machine, kept off the wire as          the open profile is (ADR 0057): on the next device it would name nothing",
    ),
    (
        "create_work_folder",
        "makes a directory under the person's media folder and writes no row: the folder is          found by its name, never recorded (ADR 0057), so there is nothing in the workspace          for an operation to describe or a replay to rebuild",
    ),
    (
        "import_legacy",
        "reads a predecessor's database once into this one, many rows at a time; the log \
         starts after it, the way it starts after the seed - a replay rebuilds on top of an \
         import rather than repeating it from a file that may no longer exist",
    ),
];

#[test]
fn no_command_writes_past_the_actions() {
    let exempt: BTreeSet<&str> = WRITES_PAST_THE_ACTIONS
        .iter()
        .map(|(name, _)| *name)
        .collect();

    let direct: Vec<String> = commands()
        .into_iter()
        .filter(|command| !exempt.contains(command.name.as_str()))
        .flat_map(|command| {
            writer_calls(&command.body)
                .into_iter()
                .filter(|(_, path)| !through_an_action(path))
                .map(move |(_, path)| format!("{} calls {path}", command.name))
        })
        .collect();

    assert!(
        direct.is_empty(),
        "these commands write to the workspace themselves instead of through an action, so \
         nothing records an operation for the write: {direct:?}\n\
         Move the write into `src/actions/` inside a gesture, or - if the change genuinely \
         does not belong in the log - add the command to WRITES_PAST_THE_ACTIONS with the reason."
    );
}

/// The exemption list has to stay a list of commands that write.
///
/// A command renamed away, or moved onto an action, leaves its exemption
/// behind, and the exemption then silently covers nothing while looking like
/// it covers something.
#[test]
fn nothing_is_exempted_that_does_not_write() {
    let commands = commands();
    let stale: Vec<&str> = WRITES_PAST_THE_ACTIONS
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| {
            commands
                .iter()
                .find(|command| command.name == *name)
                .is_none_or(|command| {
                    writer_calls(&command.body)
                        .iter()
                        .all(|(_, path)| through_an_action(path))
                })
        })
        .collect();

    assert!(
        stale.is_empty(),
        "these are exempted from going through an action but are not commands that write \
         past one: {stale:?}"
    );
}

/// A function of the actions layer: where it is and where its body runs.
struct ActionFn<'a> {
    file: &'a Source,
    name: String,
    body: std::ops::Range<usize>,
}

fn action_functions(sources: &[Source]) -> Vec<ActionFn<'_>> {
    let mut found = Vec::new();
    for file in sources
        .iter()
        .filter(|file| file.path.starts_with("src-tauri/src/actions/"))
    {
        for (at, _) in file.code.match_indices("fn ") {
            let before = file.code[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let name_start = at + "fn ".len();
            let Some(name_end) = file.code[name_start..]
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .map(|end| name_start + end)
            else {
                continue;
            };
            let Some(open) = file.code[name_end..].find('{').map(|at| name_end + at) else {
                continue;
            };
            // A declaration with no body is not a function here; the first
            // brace after it would be someone else's.
            if file.code[name_end..open].contains(';') {
                continue;
            }
            found.push(ActionFn {
                file,
                name: file.code[name_start..name_end].to_owned(),
                body: open..matching_brace(&file.code, open),
            });
        }
    }
    found
}

/// Actions that write outside a gesture, on purpose, and why.
const WRITES_OUTSIDE_A_GESTURE: [(&str, &str); 1] = [(
    "undo",
    "an undo records its own operation - `undo.<kind>` - inside `crate::undo::undo`, beside \
     the reversal it makes; held by `take_back` in `undo_takes_back.rs`",
)];

#[test]
fn every_write_in_the_actions_is_inside_a_gesture() {
    let sources = backend();
    let functions = action_functions(&sources);
    assert!(
        functions.len() >= 60,
        "read only {} functions in src/actions - the scan has stopped seeing the actions layer",
        functions.len()
    );
    let exempt: BTreeSet<&str> = WRITES_OUTSIDE_A_GESTURE
        .iter()
        .map(|(name, _)| *name)
        .collect();

    let mut outside = Vec::new();
    for function in &functions {
        if exempt.contains(function.name.as_str()) {
            continue;
        }
        let gestures = gestures(function.file);
        let body = &function.file.code[function.body.clone()];
        for (offset, path) in writer_calls(body) {
            if through_an_action(&path) {
                continue;
            }
            let at = function.body.start + offset;
            if !gestures.iter().any(|(span, _)| span.contains(&at)) {
                outside.push(format!(
                    "{}::{} calls {path}",
                    function.file.path, function.name
                ));
            }
        }
    }

    assert!(
        outside.is_empty(),
        "these actions write outside a gesture, so no operation is recorded for the write: \
         {outside:?}\nWrap the write in `gesture(conn, \"kind\", |act| …)`."
    );
}

/// The gesture primitive itself records - the one place every action relies
/// on. Two links, both read in code: `gesture_in` records the act's intent,
/// and that writes the operation. If either stopped, every write in the
/// application would fall out of the log at once and nothing above would
/// notice.
#[test]
fn the_gesture_records_its_operation() {
    let sources = backend();
    let gesture_in = the_function(&sources, "gesture_in");
    assert!(
        gesture_in.code.contains(".record_the_intent()"),
        "`gesture_in` no longer records the act's intent, so no gesture records its operation"
    );
    let finish = the_function(&sources, "record_the_intent");
    assert!(
        finish.code.contains("operation::record("),
        "`Act::record_the_intent` no longer records the operation"
    );
}

/// The gate itself has to be able to fail: it must see writes where writes
/// are. Without this, a change to the writer list that stops recognising
/// anything would leave the gates above green and testing nothing.
#[test]
fn the_gate_recognises_the_writes_it_is_about() {
    let sources = backend();
    let writing = action_functions(&sources)
        .iter()
        .filter(|function| {
            writer_calls(&function.file.code[function.body.clone()])
                .iter()
                .any(|(_, path)| !through_an_action(path))
        })
        .count();
    assert!(
        writing >= 40,
        "only {writing} actions look like they write — the scan has stopped seeing what it is \
         supposed to check"
    );

    let through = commands()
        .iter()
        .filter(|command| {
            writer_calls(&command.body)
                .iter()
                .any(|(_, path)| through_an_action(path))
        })
        .count();
    assert!(
        through >= 50,
        "only {through} commands hand a write to an action — the scan has stopped seeing them"
    );
}
