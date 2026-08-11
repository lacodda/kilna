//! Writes a workspace out as markdown.
//!
//!   cargo run --example export -- <kilna.db> <directory>

use kilna_lib::exchange::export;
use kilna_lib::{db, profile};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let source = args.next().ok_or("usage: export <kilna.db> <directory>")?;
    let directory = args.next().ok_or("usage: export <kilna.db> <directory>")?;

    let conn = db::open(std::path::Path::new(&source))?;
    profile::seed(&conn)?;

    let report = export::to_markdown(&conn, std::path::Path::new(&directory))?;

    println!("wrote {} files for {} works", report.files, report.works);
    println!("into {}", report.directory);

    Ok(())
}
