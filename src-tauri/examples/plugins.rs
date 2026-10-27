//! Lists installed plugins and runs one against a work.
//!
//!   cargo run --example plugins -- <kilna.db> [work title]
//!
//! Checks the whole path a plugin takes: discovery by name, the manifest it
//! reports, the invocation, and the metadata merged back into the work.

use kilna_lib::plugin::{self, Invocation, manifest::Target};
use kilna_lib::work::{self, WorkFilter, WorkPatch};
use kilna_lib::{db, profile};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("usage: plugins <kilna.db> [title]")?;
    let wanted = args.next();

    let path = std::path::PathBuf::from(path);
    let directory = path.parent().ok_or("the database has no directory")?;

    let found = plugin::discover(directory);
    println!("plugins found: {}", found.len());
    for entry in &found {
        match (&entry.manifest, entry.usable) {
            (Some(manifest), true) => println!(
                "  {} {} — {} command(s)",
                entry.executable,
                manifest.version,
                manifest.commands.len()
            ),
            _ => println!(
                "  {} — unusable: {}",
                entry.executable,
                entry.reason.as_deref().unwrap_or("no reason given")
            ),
        }
    }

    let Some(usable) = found.iter().find(|entry| entry.usable) else {
        println!("\nnothing usable to try");
        return Ok(());
    };
    let manifest = usable.manifest.as_ref().expect("a usable plugin has one");
    let Some(command) = manifest
        .commands
        .iter()
        .find(|command| command.target == Target::Work)
    else {
        println!("\nno command targets a work");
        return Ok(());
    };

    let conn = db::open(&path)?;
    profile::seed(&conn)?;
    let profile_id = profile::active(&conn)?.ok_or("no active profile")?.id;

    let works = work::list(
        &conn,
        &profile_id,
        &WorkFilter {
            search: wanted,
            ..Default::default()
        },
    )?;
    let target = works
        .iter()
        .find(|work| work.current_version_id.is_some())
        .ok_or("no work with a draft to measure")?;

    println!("\nrunning `{}` on {}", command.label, target.title);

    // The same subject the command builds: the row, every role's body, the
    // folders, the finished files and the shot list.
    let subject = kilna_lib::actions::plugin::subject(&conn, Target::Work, &target.id)?;

    let cancel = std::sync::atomic::AtomicBool::new(false);
    let outcome = plugin::invoke(
        std::path::Path::new(&usable.path),
        &Invocation {
            command: &command.key,
            target: Target::Work,
            subject: subject.clone(),
        },
        plugin::Run {
            limit: command.allowance(),
            cancel: &cancel,
            on_progress: &mut |step| println!("progress: {step:?}"),
        },
    )?;

    if let Some(message) = &outcome.message {
        println!("said: {message}");
    }

    if !outcome.meta.is_empty() {
        let existing = target.meta.clone();
        let merged = plugin::merge_meta(&existing, &outcome.meta);
        let updated = work::update(
            &conn,
            &target.id,
            WorkPatch {
                meta: Some(merged),
                ..Default::default()
            },
        )?;
        println!("meta now: {}", serde_json::to_string(&updated.meta)?);
    }

    Ok(())
}
