//! Getting data out and in: markdown export, workspace backups, a work as a
//! parcel that leaves one space and lands in another, works coming in from
//! files and the clipboard, and importing a slice of the author's previous
//! system.
//!
//! The export exists to make the "you are not locked in" promise checkable
//! rather than stated. See ADR 0002.

/// The space a migration from the command line works on: the one `--space`
/// names, else the last opened. Writing to a guess is refused once the
/// application holds more than one space - a run aimed at "the last opened"
/// met a channel space made minutes before instead of the one meant
/// (v0.164); a plan only reads, and may still take the last opened.
pub fn target_space(
    app: &rusqlite::Connection,
    app_dir: &std::path::Path,
    space: Option<&str>,
    apply_it: bool,
) -> crate::error::Result<crate::space::Space> {
    use crate::space::registry;
    if let Some(name) = space {
        return registry::by_name(app, app_dir, name);
    }
    let live: Vec<crate::space::Space> = registry::list(app, app_dir, None)?
        .into_iter()
        .filter(|one| one.trashed_at.is_none())
        .collect();
    if apply_it && live.len() > 1 {
        let names: Vec<String> = live.iter().map(|one| one.name.clone()).collect();
        return Err(crate::error::Error::refused("space.nameIt").param("names", names.join(", ")));
    }
    registry::newest(app, app_dir)?.ok_or_else(|| crate::error::Error::refused("space.none"))
}

pub mod backup;
pub mod channel;
pub mod chat;
pub mod convert;
pub mod export;
pub mod ideas;
pub mod import;
pub mod intake;
pub mod material;
pub mod package;
pub mod parcel;
pub mod rehome;
pub mod settle;
pub mod songs;
