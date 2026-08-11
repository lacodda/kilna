//! Prints what a workspace database contains.
//!
//!   cargo run --example inspect -- <path-to-kilna.db>
//!
//! Useful when a bug report comes with a database but no reproduction.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args()
        .nth(1)
        .ok_or("usage: inspect <path-to-kilna.db>")?
        .into();

    let conn = kilna_lib::db::open(&path)?;
    let workspace = kilna_lib::profile::workspace(&conn)?;

    println!("path            {}", path.display());
    println!("schema version  {}", workspace.schema_version);
    println!("works           {}", workspace.works);
    println!("releases        {}", workspace.releases);

    match workspace.profile {
        Some(profile) => {
            println!("active profile  {} ({})", profile.name, profile.key);
            println!("axes            {}", profile.config.axes.len());
            for axis in &profile.config.axes {
                println!("  {:<12} weight {:>4}", axis.key, axis.weight);
            }
            println!("tiers");
            for tier in &profile.config.tiers {
                println!("  {:<12} from {:>5}", tier.key, tier.min);
            }
        }
        None => println!("active profile  none"),
    }

    let profile_id = kilna_lib::profile::active(&conn)?.map(|profile| profile.id);
    if let Some(profile_id) = profile_id {
        let works = kilna_lib::work::list(&conn, &profile_id, &Default::default())?;
        println!("\nworks");
        for work in &works {
            println!("  {:<20} {:<10} {}", work.title, work.status, work.kind);
            for version in kilna_lib::work::version::list(&conn, &work.id)? {
                println!(
                    "      {:<8} r{}  {:>5} chars{}",
                    version.role,
                    version.revision,
                    version.length,
                    if version.is_current {
                        "  ← current"
                    } else {
                        ""
                    }
                );
            }
            let notes = kilna_lib::note::list(
                &conn,
                &profile_id,
                &kilna_lib::note::NoteFilter {
                    work_id: Some(work.id.clone()),
                    ..Default::default()
                },
            )?;
            for note in notes {
                println!("      note: {} [{}]", note.body, note.tags.join(", "));
            }
        }
    }

    Ok(())
}
