//! Shows what the kind of a space actually changes.
//!
//!   cargo run --example profiles
//!
//! The point of ADR 0001 is that a craft is configuration, not schema. This
//! makes a space of each shipped kind in memory (ADR 0077), prints its
//! vocabulary and puts one work in it, so the claim can be checked rather than
//! believed.

use kilna_lib::work::NewWork;
use kilna_lib::{db, profile, work};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for shipped in profile::builtin()? {
        let conn = db::open_in_memory()?;
        profile::seed(&conn, &shipped.key)?;
        let entry = profile::active(&conn)?.ok_or("nothing active")?;
        let config = &entry.config;
        println!("=== {} ({}) ===", entry.name, entry.key);
        println!(
            "  work kinds    {}",
            config
                .work_kinds
                .iter()
                .map(|k| k.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "  roles         {}",
            config.work_kinds[0]
                .version_roles
                .iter()
                .map(|r| r.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "  axes          {}",
            config.work_kinds[0]
                .axes
                .iter()
                .map(|a| format!("{} ×{}", a.label, a.weight))
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "  tiers         {}",
            config.work_kinds[0]
                .tiers
                .iter()
                .map(|t| format!("{} ≥{}", t.label, t.min))
                .collect::<Vec<_>>()
                .join(", ")
        );

        // One work per space, in that space's own vocabulary.
        let created = work::create(
            &conn,
            &entry.id,
            NewWork {
                kind: config.work_kinds[0].key.clone(),
                title: format!(
                    "Sample {}",
                    config.work_kinds[0].label.as_str().to_lowercase()
                ),
                ..Default::default()
            },
        )?;
        println!("  added         {} ({})", created.title, created.status);
    }
    Ok(())
}
