//! Shows what switching profiles actually changes.
//!
//!   cargo run --example profiles -- <kilna.db>
//!
//! The point of ADR 0001 is that a craft is configuration, not schema. This
//! prints each built-in profile's vocabulary and puts one work in each, so the
//! claim can be checked rather than believed.

use kilna_lib::work::{NewWork, WorkFilter};
use kilna_lib::{db, profile, work};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: profiles <kilna.db>")?;

    let mut conn = db::open(std::path::Path::new(&path))?;
    profile::seed(&conn)?;

    for entry in profile::list(&conn)? {
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
            config
                .version_roles
                .iter()
                .map(|r| r.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "  axes          {}",
            config
                .axes
                .iter()
                .map(|a| format!("{} ×{}", a.label, a.weight))
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "  tiers         {}",
            config
                .tiers
                .iter()
                .map(|t| format!("{} ≥{}", t.label, t.min))
                .collect::<Vec<_>>()
                .join(", ")
        );

        // One work per profile, in that profile's own vocabulary.
        let kind = config.work_kinds[0].key.clone();
        let title = format!("Sample {}", config.work_kinds[0].label.to_lowercase());
        let already = work::list(
            &conn,
            &entry.id,
            &WorkFilter {
                search: Some(title.clone()),
                ..Default::default()
            },
        )?;
        if already.is_empty() {
            let created = work::create(
                &conn,
                &entry.id,
                NewWork {
                    kind,
                    title,
                    ..Default::default()
                },
            )?;
            println!("  added         {} ({})", created.title, created.status);
        }

        let mine = work::list(&conn, &entry.id, &WorkFilter::default())?;
        println!("  works here    {}", mine.len());
    }

    // Switching is what a user does; the vocabulary follows the active profile.
    let first = profile::list(&conn)?
        .first()
        .cloned()
        .ok_or("no profiles")?;
    profile::activate(&mut conn, &first.id)?;
    let active = profile::active(&conn)?.ok_or("nothing active")?;
    println!(
        "\nactive now: {} — statuses: {}",
        active.name,
        active
            .config
            .statuses
            .iter()
            .map(|s| s.label.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(())
}
