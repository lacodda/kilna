//! Opens a workspace, migrates it, and asks the search index a question.
//!
//!   cargo run --example search_probe -- <kilna.db> <query>
//!
//! Exists so full-text search can be tried against a real catalogue rather
//! than against invented rows: a tokenizer that looks right on three English
//! fixtures is exactly the thing that fails on a real Russian one.

use kilna_lib::db;
use kilna_lib::search;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: search_probe <kilna.db> <query>")?;
    let query = args.collect::<Vec<_>>().join(" ");

    let conn = db::open(std::path::Path::new(&path))?;

    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let indexed: i64 = conn.query_row("SELECT count(*) FROM search_index", [], |row| row.get(0))?;
    let staged: i64 = conn.query_row(
        "SELECT count(*) FROM work WHERE stage IS NOT NULL",
        [],
        |row| row.get(0),
    )?;
    println!("schema {version} · indexed {indexed} · staged {staged}");

    if query.trim().is_empty() {
        return Ok(());
    }

    let profile_id = kilna_lib::profile::active(&conn)?
        .ok_or("no active profile")?
        .id;
    let hits = search::find(&conn, &profile_id, &query)?;
    println!("\n{} hit(s) for {query:?}", hits.len());
    for hit in &hits {
        println!("  [{:?}] {} — {}", hit.kind, hit.work_title, hit.title);
    }

    Ok(())
}
