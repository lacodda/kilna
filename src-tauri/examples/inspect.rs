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

    Ok(())
}
