//! Opens a workspace the way the window does, and says what the opening did.
//!
//!   cargo run --example open -- <copy-of-kilna.db>
//!
//! For rehearsing an upgrade on a copy of a real workspace before a tag: the
//! schema migrations, the profile's carry-forward and every move that runs on
//! open (the doors of v0.74, the publications of v0.86) happen exactly as
//! they would on the owner's machine, and the lines they wrote in the history
//! are printed. Never point it at the live file - it writes.

use kilna_lib::{journal, profile, state};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: open <copy-of-kilna.db>")?;
    let opened = state::AppState::open(std::path::Path::new(&path))?;
    let conn = opened.conn();
    let profile_id = profile::active(&conn)?.ok_or("no active profile")?.id;

    for entry in journal::list(&conn, &profile_id)?
        .into_iter()
        .filter(|entry| entry.action.starts_with("upgrade."))
    {
        println!(
            "{}  {}  {}",
            entry.created_at,
            entry.action,
            serde_json::to_string(&entry.params)?
        );
    }
    Ok(())
}
