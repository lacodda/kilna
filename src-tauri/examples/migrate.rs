//! Imports a slice of a predecessor workspace into a kilna one.
//!
//!   cargo run --example migrate -- <kilna.db> <legacy.db>
//!
//! Exists to try the data model against a real catalogue rather than against
//! invented demo rows. Existing works are matched by title and left alone, so
//! running it twice is safe.

use kilna_lib::exchange::import;
use kilna_lib::work::WorkFilter;
use kilna_lib::{db, profile, score, work};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let target = args.next().ok_or("usage: migrate <kilna.db> <legacy.db>")?;
    let source = args.next().ok_or("usage: migrate <kilna.db> <legacy.db>")?;

    let mut conn = db::open(std::path::Path::new(&target))?;
    profile::seed(&conn)?;
    let profile_id = profile::active(&conn)?.ok_or("no active profile")?.id;

    let report = import::from_legacy(&mut conn, std::path::Path::new(&source), &profile_id)?;

    println!("works     {}", report.works);
    println!("versions  {}", report.versions);
    println!("scores    {}", report.scores);
    println!("releases  {}", report.releases);
    println!("skipped   {}", report.skipped);

    let works = work::list(&conn, &profile_id, &WorkFilter::default())?;
    println!("\nin the workspace: {} works", works.len());

    // What the import means for the catalogue: the top of it, by this
    // profile's weights rather than the source's.
    println!("\ntop of the catalogue");
    for row in score::catalogue(&conn, &profile_id)?.into_iter().take(10) {
        println!(
            "  {:>5}  {:<10} {}",
            row.total
                .map(|total| format!("{total:.1}"))
                .unwrap_or_else(|| "-".into()),
            row.tier.as_deref().unwrap_or("-"),
            row.title
        );
    }

    Ok(())
}
