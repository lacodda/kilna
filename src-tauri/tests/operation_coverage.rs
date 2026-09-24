//! Holds every mutating command to the operations log.
//!
//! A command that changes the workspace and records no operation leaves a hole
//! in the log, and a log with a hole replays to a database that never existed —
//! silently, because nothing at the time of the missing write goes wrong. The
//! failure surfaces much later, in a merge or a repair, as data that disagrees
//! with itself.
//!
//! Read from the source rather than by running the commands, for the reason
//! `journal_keys.rs` gives: a command no test happens to call contributes
//! nothing, and that is exactly the command most likely to have been forgotten.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri always has a parent")
        .to_path_buf()
}

fn commands_source() -> String {
    std::fs::read_to_string(repo_root().join("src-tauri/src/commands.rs"))
        .expect("commands.rs is readable")
}

/// One `#[tauri::command]` function, as the scan sees it.
struct Command {
    name: String,
    body: String,
}

/// The source with every comment and every string and character literal
/// blanked to spaces, so that what is left is code: a brace inside a
/// `format!` does not open a block, and a word in a comment does not count as
/// a call. Lengths are kept, so an offset means the same place in both.
fn code_only(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for byte in &mut out[from..to] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let end = source[i..].find('\n').map_or(bytes.len(), |at| i + at);
                blank(&mut out, i, end);
                i = end;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let end = source[i + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |at| i + 2 + at + 2);
                blank(&mut out, i, end);
                i = end;
            }
            // A raw string: r"..." or r#"..."# with any number of hashes.
            b'r' if matches!(bytes.get(i + 1), Some(b'"') | Some(b'#'))
                && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_')) =>
            {
                let hashes = bytes[i + 1..]
                    .iter()
                    .take_while(|byte| **byte == b'#')
                    .count();
                if bytes.get(i + 1 + hashes) != Some(&b'"') {
                    i += 1;
                    continue;
                }
                let close = format!("\"{}", "#".repeat(hashes));
                let start = i + 2 + hashes;
                let end = source[start..]
                    .find(&close)
                    .map_or(bytes.len(), |at| start + at + close.len());
                blank(&mut out, i, end);
                i = end;
            }
            b'"' => {
                let mut end = i + 1;
                while end < bytes.len() && bytes[end] != b'"' {
                    end += if bytes[end] == b'\\' { 2 } else { 1 };
                }
                blank(&mut out, i, (end + 1).min(bytes.len()));
                i = end + 1;
            }
            // A character literal ('{', '\n', '\''); a lifetime has no
            // closing quote within the next few bytes and is left alone.
            b'\'' => {
                let end = if bytes.get(i + 1) == Some(&b'\\') {
                    source[i + 2..].find('\'').map(|at| i + 2 + at)
                } else if bytes.get(i + 2) == Some(&b'\'') {
                    Some(i + 2)
                } else {
                    None
                };
                match end {
                    Some(end) if end - i <= 10 => {
                        blank(&mut out, i, end + 1);
                        i = end + 1;
                    }
                    _ => i += 1,
                }
            }
            _ => i += 1,
        }
    }
    String::from_utf8(out).expect("blanking keeps the text valid")
}

/// Every command in `commands.rs`, with the text of its body.
///
/// The body is the function's own block, from its opening brace to the brace
/// that closes it, read in code with comments and literals blanked. It used to
/// run to the next `#[tauri::command]`, which took in whatever helpers sat
/// between: `update_profile_config` was followed by the `recording` helper and
/// `set_current_version` by `discard_and_record`, so both "recorded an
/// operation" while writing past the log - and a doc comment mentioning a
/// write counted as one.
fn commands() -> Vec<Command> {
    let source = commands_source();
    let code = code_only(&source);
    // Both spellings: `#[tauri::command]` and `#[tauri::command(async)]`,
    // which runs a command off the main thread.
    let marker = "#[tauri::command";

    let mut found = Vec::new();
    for (start, _) in code.match_indices(marker) {
        let Some(at) = code[start..].find("pub fn ").map(|at| start + at) else {
            continue;
        };
        let name_start = at + "pub fn ".len();
        let Some(open_paren) = code[name_start..].find('(').map(|at| name_start + at) else {
            continue;
        };
        let name = code[name_start..open_paren].trim().to_owned();

        // The first brace after the signature opens the body; count to the
        // one that closes it.
        let Some(open) = code[open_paren..].find('{').map(|at| open_paren + at) else {
            continue;
        };
        let mut depth = 0usize;
        let mut close = code.len();
        for (offset, byte) in code[open..].bytes().enumerate() {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        close = open + offset + 1;
                        break;
                    }
                }
                _ => {}
            }
        }

        found.push(Command {
            name,
            body: code[open..close].to_owned(),
        });
    }

    assert!(
        !found.is_empty(),
        "no `#[tauri::command]` functions found — this test has stopped testing anything"
    );
    found
}

/// Every command the application registers, from `generate_handler!`.
fn registered() -> BTreeSet<String> {
    let source = std::fs::read_to_string(repo_root().join("src-tauri/src/lib.rs"))
        .expect("lib.rs is readable");
    let start = source
        .find("generate_handler![")
        .expect("lib.rs registers its commands with generate_handler!")
        + "generate_handler![".len();
    let end = source[start..]
        .find(']')
        .map_or(source.len(), |at| start + at);
    source[start..end]
        .split(',')
        .filter_map(|entry| entry.trim().strip_prefix("commands::"))
        .map(|name| name.trim().to_owned())
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
        "the commands scanned in commands.rs and those registered in lib.rs disagree"
    );
}

/// Commands that change the workspace and so must record an operation.
///
/// Recognised by what they call, not by their names: a command mutates if its
/// body reaches a domain function that writes. Naming them by hand would make
/// this gate a list someone has to remember to extend, which is the failure it
/// exists to prevent.
fn writes_to_the_workspace(body: &str) -> bool {
    // A domain call that writes. Deliberately not `recording(` — that is how a
    // command records, and a gate whose test for "writes" is the same string as
    // its test for "records" can never catch anything. Matched on `::name(` so
    // that a local helper of
    // the same name does not count, and listed rather than inferred because
    // "writes" is not visible in a name: `score::catalogue` reads, `work::pin_tier`
    // writes.
    const WRITERS: [&str; 46] = [
        "::create(",
        "::create_minted(",
        "::update(",
        "::update_at(",
        "::delete(",
        "::discard(",
        "::discard_works_batch(",
        // Taking something back changes the workspace as surely as doing it.
        "::undo(",
        "::restore(",
        "::purge(",
        "::empty(",
        "::activate(",
        "::update_config(",
        "::add_note(",
        "::add_note_minted(",
        "::update_note(",
        "::update_note_at(",
        "::delete_note(",
        "::reorder(",
        "::reorder_notes(",
        "::set_contents(",
        "::set_contents_at(",
        "::set_current(",
        "::resync(",
        "::resync_at(",
        "::apply(",
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
        // The shared closure every non-self-transacting command routes its
        // change and its log entry through together. A command whose body
        // reaches here writes, even when the domain call it wraps has no
        // other name on this list — `recording` is the write.
    ];

    // Deletions all go through one helper, which records the operation once for
    // all six entities. A command that calls it is covered by that write, not by
    // one of its own — so it counts as both writing and recording.
    if body.contains("discard_and_record(") {
        return false;
    }

    WRITERS.iter().any(|writer| body.contains(writer))
}

/// Whether a command writes an operation, by either route.
///
/// Most go through `recording`, which pairs the change and the log entry in one
/// transaction. A domain function that opens its own transaction takes the
/// operation as an argument instead, and the command builds it — so an
/// `Intent::new` in the body counts too.
fn records_an_operation(body: &str) -> bool {
    body.contains("operation::record")
        || body.contains("operation::Intent::new")
        || body.contains("recording(")
}

/// Commands that change something, but nothing the log is about.
///
/// Each entry is a promise that replaying the log without it still produces the
/// right database, and each says why. Anything not on this list that writes has
/// to be logged.
const NOT_IN_THE_LOG: [(&str, &str); 19] = [
    (
        "undo_last",
        "records its operation one level down, inside `undo::undo`'s own \
         transaction — held by `the_undo_path_records_an_operation` below",
    ),
    (
        "import_legacy",
        "reads a predecessor's database once into this one, many rows at a time; \
         the log starts after it, the way it starts after the seed - a replay \
         rebuilds on top of an import rather than repeating it from a file that \
         may no longer exist",
    ),
    (
        "mark_journal_read",
        "the journal is a feed for a person, not part of the workspace a replay rebuilds",
    ),
    (
        "activate_profile",
        "which profile is open is a fact about this machine; ADR 0012 keeps it off the wire \
         for the same reason",
    ),
    (
        "ask_assistant",
        "an assistant run writes only chat rows, which are this device's conversation",
    ),
    ("start_run", "as above"),
    ("start_task", "as above"),
    ("start_tasks", "as above"),
    ("cancel_run", "as above"),
    ("clear_waiting", "as above"),
    ("clear_task_queue", "as above"),
    ("create_chat", "as above"),
    ("rename_chat", "as above"),
    ("delete_chat", "as above"),
    (
        "dismiss_proposal",
        "marks one chat message as turned down and writes nothing else; a chat is          this device's conversation, as the runs above are",
    ),
    (
        "apply_proposal",
        "records one operation per row it writes, inside `assistant::apply` — the \
         same intents the hand-driven commands record; held by \
         `a_package_in_a_chat_on_nothing_creates_the_whole_work_through_the_log` there",
    ),
    ("apply_pending_proposals", "as above, once per proposal"),
    (
        "start_comment_task",
        "opens a chat and starts a run, which are this device's conversation; the \
         reply it drafts is written only when the person keeps it, through `apply_proposal`",
    ),
    (
        "start_screenshot_task",
        "as above: the comment read off the picture is kept through `apply_proposal`, \
         and the picture itself goes to a temporary folder, not the workspace",
    ),
];

#[test]
fn every_mutating_command_records_an_operation() {
    let exempt: BTreeSet<&str> = NOT_IN_THE_LOG.iter().map(|(name, _)| *name).collect();

    let missing: Vec<String> = commands()
        .into_iter()
        .filter(|command| !exempt.contains(command.name.as_str()))
        .filter(|command| writes_to_the_workspace(&command.body))
        .filter(|command| !records_an_operation(&command.body))
        .map(|command| command.name)
        .collect();

    assert!(
        missing.is_empty(),
        "these commands change the workspace but record no operation, so the log replays \
         to a database that never existed: {missing:?}\n\
         Either call `operation::record` in each, or — if the change genuinely does not \
         belong in the log — add it to NOT_IN_THE_LOG with the reason."
    );
}

/// The exemption list has to stay a list of real commands.
///
/// A command renamed away leaves its exemption behind, and the exemption then
/// silently covers nothing while looking like it covers something.
#[test]
fn nothing_is_exempted_that_is_not_a_command() {
    let names: BTreeSet<String> = commands().into_iter().map(|command| command.name).collect();

    let stale: Vec<&str> = NOT_IN_THE_LOG
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !names.contains(*name))
        .collect();

    assert!(
        stale.is_empty(),
        "these names are exempted from the operations log but are not commands any more: {stale:?}"
    );
}

/// The shared deletion helper records for all six entities it serves.
///
/// `every_mutating_command_records_an_operation` passes over the commands that
/// call it, because the write happens one level down. If the helper stopped
/// recording, every deletion in the application would fall out of the log at
/// once and nothing above would notice.
#[test]
fn the_shared_deletion_helper_records_an_operation() {
    let source = commands_source();
    let at = source
        .find("fn discard_and_record(")
        .expect("the deletion helper is still called that");
    let end = source[at..]
        // The source is checked out with LF endings; a search for CRLF found
        // nothing and let this scan run to the end of the file, where any
        // later command's record satisfied it.
        .find("\n#[tauri::command")
        .map_or(source.len(), |offset| at + offset);

    assert!(
        source[at..end].contains("operation::record"),
        "`discard_and_record` records no operation, so every deletion in the application \
         is missing from the log"
    );
}

/// Taking something back is recorded like anything else that changes rows.
///
/// `every_mutating_command_records_an_operation` passes over `undo_last`,
/// because the write happens inside `undo::undo`. If that stopped recording,
/// every undo in the application would fall out of the log at once and the
/// command above would look innocent.
#[test]
fn the_undo_path_records_an_operation() {
    let source = std::fs::read_to_string(repo_root().join("src-tauri/src/undo.rs"))
        .expect("undo.rs is readable");

    assert!(
        source.contains("operation::record"),
        "`undo::undo` records no operation, so undoing something would leave the log \
         claiming the change is still in force"
    );
}

/// The gate itself has to be able to fail.
///
/// Without this, a change to `writes_to_the_workspace` that stops recognising
/// anything would leave the first test green and testing nothing — the shape of
/// false green this project has hit before.
#[test]
fn the_gate_recognises_the_commands_it_is_about() {
    let recognised = commands()
        .into_iter()
        .filter(|command| writes_to_the_workspace(&command.body))
        .count();

    assert!(
        recognised >= 20,
        "only {recognised} commands look like they write — the scan has stopped seeing \
         what it is supposed to check"
    );
}
