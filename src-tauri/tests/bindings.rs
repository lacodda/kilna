//! The window's IPC types, generated from the Rust ones that cross it.
//!
//! `src/lib/api/types.ts` used to hand-write every shape the backend sends
//! and accepts, and nothing held the two sides together (ADR 0003) - they
//! drifted before. Here, every argument and return type of every
//! `#[tauri::command]`, and every event payload, is exported with
//! [`ts_rs`] together with everything it depends on, into a temporary
//! directory, and compared file by file against the committed
//! `src/lib/api/generated/`. A mismatch fails the test with the list of what
//! changed and this instruction:
//!
//!     KILNA_BLESS=1 cargo test --test bindings
//!
//! which rewrites `src/lib/api/generated/` to match - deleting it first, so a
//! type that stopped crossing the wire is actually removed rather than left
//! behind stale.
//!
//! `#[ts(export)]` is deliberately not used on the types themselves: it would
//! make every `cargo test` anywhere in the crate write into the tree. This is
//! the one place that exports, and only when asked to.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ts_rs::{Config, TS};

/// Where the frontend keeps its copy.
fn generated_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri always has a parent")
        .join("src/lib/api/generated")
}

/// Export every root type, and everything it depends on, into `dir`.
///
/// The list is every command's argument and return type
/// (`src-tauri/src/commands/*.rs`) and every event payload
/// (`assistant::run::Emission`, `commands::assistant::TaskQueue`) - not their
/// dependencies, which `TS::export_all` follows on its own.
fn export_all(dir: &Path) -> Result<(), ts_rs::ExportError> {
    let cfg = Config::from_env().with_out_dir(dir);

    macro_rules! export {
        ($($ty:ty),* $(,)?) => {
            $(<$ty as TS>::export_all(&cfg)?;)*
        };
    }

    export! {
        // workspace.rs
        kilna_lib::profile::Workspace,
        kilna_lib::profile::Profile,
        kilna_lib::profile::config::ProfileConfig,
        // works.rs
        kilna_lib::work::WorkFilter,
        kilna_lib::work::Work,
        kilna_lib::work::NewWork,
        kilna_lib::work::WorkPatch,
        kilna_lib::work::status::Change,
        kilna_lib::actions::work::Discarded,
        kilna_lib::actions::work::Made,
        kilna_lib::cover::read::CoverView,
        kilna_lib::cover::read::FrameView,
        // ideas.rs
        kilna_lib::cover::idea::CoverBoard,
        kilna_lib::cover::idea::CoverIdea,
        kilna_lib::cover::idea::Verdict,
        kilna_lib::cover::idea::IdeaRequest,
        kilna_lib::actions::BulkOutcome,
        kilna_lib::score::ScoredWork,
        kilna_lib::card::Counts,
        kilna_lib::clone::Cloned,
        // versions.rs
        kilna_lib::work::version::VersionSummary,
        kilna_lib::work::version::Version,
        kilna_lib::work::version::NewVersion,
        // notes.rs
        kilna_lib::note::NewNote,
        kilna_lib::note::Note,
        kilna_lib::note::NotePatch,
        kilna_lib::note::NoteFilter,
        kilna_lib::note::Promotion,
        kilna_lib::note::Promoted,
        kilna_lib::commands::notes::ResolvedLink,
        // register.rs
        kilna_lib::register::RegisterEntry,
        kilna_lib::register::Term,
        kilna_lib::register::NewTerm,
        kilna_lib::register::TermPatch,
        kilna_lib::register::TermUse,
        kilna_lib::register::check::TextCheck,
        // canon.rs
        kilna_lib::canon::view::CardFilter,
        kilna_lib::canon::view::CardSummary,
        kilna_lib::canon::view::CardView,
        kilna_lib::canon::view::Dated,
        kilna_lib::canon::Fact,
        kilna_lib::canon::NewFact,
        kilna_lib::canon::FactPatch,
        kilna_lib::canon::CanonLink,
        kilna_lib::canon::NewCanonLink,
        kilna_lib::canon::CanonLinkPatch,
        kilna_lib::canon::proposal::FactReview,
        kilna_lib::assistant::apply::CanonProposal,
        kilna_lib::profile::config::Lens,
        // comments.rs
        kilna_lib::comment::Comment,
        kilna_lib::comment::CommentFilter,
        kilna_lib::comment::CommentPatch,
        kilna_lib::comment::NewComment,
        // styles.rs
        kilna_lib::style_brick::StyleBrickFilter,
        kilna_lib::style_brick::StyleBrick,
        kilna_lib::style_brick::NewStyleBrick,
        kilna_lib::style_brick::StyleBrickPatch,
        kilna_lib::asset::Asset,
        // focus.rs
        kilna_lib::focus::Dismissal,
        kilna_lib::focus::DismissalKey,
        kilna_lib::focus::FocusNote,
        kilna_lib::focus::NewFocusNote,
        kilna_lib::focus::FocusNotePatch,
        // scores.rs
        kilna_lib::score::NewScore,
        kilna_lib::score::Score,
        kilna_lib::profile::config::KindVerdict,
        // releases.rs
        kilna_lib::release::NewRelease,
        kilna_lib::release::Release,
        kilna_lib::release::ReleasePatch,
        kilna_lib::release_meta::Field,
        kilna_lib::release_meta::Generated,
        kilna_lib::release_meta::ReleaseProposal,
        kilna_lib::actions::release::GeneratedBatch,
        kilna_lib::release::SlotPreview,
        kilna_lib::release::Scheduling,
        kilna_lib::release::ScheduledRelease,
        kilna_lib::layout::Placement,
        // collections.rs
        kilna_lib::collection::Collection,
        kilna_lib::collection::NewCollection,
        kilna_lib::collection::CollectionPatch,
        // links.rs
        kilna_lib::link::Links,
        kilna_lib::link::NewLink,
        kilna_lib::link::Link,
        kilna_lib::publication::Publications,
        // scenes.rs
        kilna_lib::scene::Scene,
        kilna_lib::scene::NewScene,
        kilna_lib::scene::ScenePatch,
        kilna_lib::scene_note::SceneNote,
        kilna_lib::scene_frame::SceneFrame,
        kilna_lib::cover::read::SceneFrameView,
        // cuts.rs
        kilna_lib::cut::Cut,
        kilna_lib::cut::NewCut,
        kilna_lib::cut::CutPatch,
        kilna_lib::cut::Shot,
        // assets.rs
        kilna_lib::asset::NewAsset,
        // search.rs
        kilna_lib::search::Hit,
        // journal.rs
        kilna_lib::journal::Entry,
        // trash.rs
        kilna_lib::trash::Deletion,
        kilna_lib::undo::Undoable,
        // assistant.rs
        kilna_lib::assistant::cli::Availability,
        kilna_lib::assistant::ChatSummary,
        kilna_lib::assistant::NewChat,
        kilna_lib::assistant::Chat,
        kilna_lib::assistant::Transcript,
        kilna_lib::actions::proposal::Overrides,
        kilna_lib::actions::proposal::Outcome,
        kilna_lib::assistant::apply::Pending,
        kilna_lib::assistant::apply::CommentProposal,
        kilna_lib::assistant::run::Run,
        kilna_lib::assistant::run::Emission,
        kilna_lib::assistant::task::Composed,
        kilna_lib::commands::assistant::TaskAbout,
        kilna_lib::commands::assistant::StartedTask,
        kilna_lib::commands::assistant::TaskQueue,
        kilna_lib::commands::assistant::StartedBatch,
        kilna_lib::actions::Skipped,
        // data.rs
        kilna_lib::exchange::export::ExportReport,
        kilna_lib::exchange::package::PackageReport,
        kilna_lib::exchange::import::ImportReport,
        // plugins.rs
        kilna_lib::plugin::manifest::Plugin,
        kilna_lib::plugin::manifest::Target,
        // error.rs - carried inside `Skipped` and every refusal reason
        kilna_lib::error::Reason,
    };

    Ok(())
}

/// Every file under `dir`, relative to it, with its contents. `\r\n` is
/// folded to `\n` so a Windows checkout of the committed copy compares equal
/// to what this process just wrote.
fn read_tree(dir: &Path) -> std::collections::BTreeMap<PathBuf, String> {
    let mut out = std::collections::BTreeMap::new();
    if !dir.exists() {
        return out;
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).expect("the directory is readable") {
            let entry = entry.expect("a directory entry reads");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let relative = path.strip_prefix(dir).expect("under dir").to_path_buf();
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_default()
                    .replace("\r\n", "\n");
                out.insert(relative, text);
            }
        }
    }
    out
}

#[test]
fn the_generated_bindings_match_the_types_that_cross_ipc() {
    let bless = std::env::var_os("KILNA_BLESS").is_some();
    let committed_dir = generated_dir();

    if bless {
        // Deleted first, so a type that stopped crossing the wire actually
        // leaves - `export_all` only ever adds files, it never removes one.
        if committed_dir.exists() {
            std::fs::remove_dir_all(&committed_dir)
                .expect("the generated directory can be removed");
        }
        std::fs::create_dir_all(&committed_dir).expect("the generated directory can be made");
        export_all(&committed_dir).expect("the types export");
        return;
    }

    let fresh = tempfile::tempdir().expect("a temporary directory is available");
    export_all(fresh.path()).expect("the types export");

    let expected = read_tree(fresh.path());
    let committed = read_tree(&committed_dir);

    let expected_paths: BTreeSet<&PathBuf> = expected.keys().collect();
    let committed_paths: BTreeSet<&PathBuf> = committed.keys().collect();

    let missing: Vec<&PathBuf> = expected_paths
        .difference(&committed_paths)
        .copied()
        .collect();
    let extra: Vec<&PathBuf> = committed_paths
        .difference(&expected_paths)
        .copied()
        .collect();
    let different: Vec<&PathBuf> = expected_paths
        .intersection(&committed_paths)
        .copied()
        .filter(|path| expected[*path] != committed[*path])
        .collect();

    assert!(
        missing.is_empty() && extra.is_empty() && different.is_empty(),
        "src/lib/api/generated/ no longer matches the types that cross IPC.\n\
         missing: {missing:?}\nextra: {extra:?}\ndifferent: {different:?}\n\
         Write it again with `KILNA_BLESS=1 cargo test --test bindings`."
    );
}

/// The root list above is written by hand, so it is held to the commands: a
/// type named in any command's signature that has no generated file is a
/// type the window would have to write by hand again - the drift this file
/// exists against, come back through a new command.
#[test]
fn every_type_a_command_names_is_generated() {
    // While the other test is rewriting the folder it reads a half-written
    // one; the plain run that follows a bless is the one that counts.
    if std::env::var_os("KILNA_BLESS").is_some() {
        return;
    }
    // Names in signatures that are not the wire's: Tauri's and std's.
    // `Response` is Tauri's raw answer - bytes, which arrive in the window as
    // an `ArrayBuffer` and have no shape to generate.
    const NOT_ON_THE_WIRE: [&str; 10] = [
        "State",
        "AppState",
        "AppHandle",
        "Result",
        "Option",
        "Vec",
        "String",
        "BTreeMap",
        "Value",
        "Response",
    ];
    let commands_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
    let generated: BTreeSet<String> = read_tree(&generated_dir())
        .keys()
        .filter_map(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .collect();

    let mut named = BTreeSet::new();
    for entry in std::fs::read_dir(&commands_dir).expect("the commands are readable") {
        let text =
            std::fs::read_to_string(entry.expect("an entry").path()).expect("a command file reads");
        for (at, _) in text.match_indices("#[tauri::command") {
            let Some(open) = text[at..].find("pub fn ").map(|offset| at + offset) else {
                continue;
            };
            let Some(body) = text[open..].find('{').map(|offset| open + offset) else {
                continue;
            };
            let signature = &text[open..body];
            for word in signature.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
                if word.starts_with(|c: char| c.is_ascii_uppercase())
                    && !NOT_ON_THE_WIRE.contains(&word)
                {
                    named.insert(word.to_owned());
                }
            }
        }
    }
    // The scan's watchdog: commands name dozens of types.
    assert!(
        named.len() >= 40,
        "read only {} type names from the commands",
        named.len()
    );

    let missing: Vec<&String> = named.difference(&generated).collect();
    assert!(
        missing.is_empty(),
        "these types cross IPC in a command's signature but are not generated: {missing:?}
         Derive `ts_rs::TS` on each, add it to `export_all`, and bless."
    );
}
