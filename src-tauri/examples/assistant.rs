//! Runs one real turn through the installed Claude Code CLI.
//!
//!   cargo run --example assistant -- <path-to-kilna.db> [work title]
//!
//! Not a test: it costs money and needs a logged-in CLI. It exists to check the
//! integration by hand after changing how kilna talks to the CLI.

use kilna_lib::assistant::{self, NewChat, cli};
use kilna_lib::work::WorkFilter;
use kilna_lib::{db, profile, work};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("usage: assistant <db> [work title]")?;
    let wanted = args.next();

    let availability = cli::probe();
    println!(
        "cli: {}",
        if availability.available {
            availability.version.unwrap_or_default()
        } else {
            availability.reason.unwrap_or_default()
        }
    );
    if !availability.available {
        return Ok(());
    }

    let mut conn = db::open(std::path::Path::new(&path))?;
    // The app does this on startup; a workspace opened directly needs it too,
    // or prompts shipped after the database was created are missing.
    profile::seed(&conn)?;
    let profile = profile::active(&conn)?.ok_or("no active profile")?;
    let profile_id = profile.id.clone();

    let works = work::list(
        &conn,
        &profile_id,
        &WorkFilter {
            search: wanted,
            ..Default::default()
        },
    )?;
    let target = works.first().ok_or("no work to talk about")?;
    println!("work: {}", target.title);

    let template = profile
        .config
        .prompts
        .first()
        .ok_or("the profile defines no prompts")?;
    println!("action: {}", template.label);

    let prompt = assistant::prompt::for_work(&conn, &target.id, &template.template)?;
    println!("\n--- prompt ---\n{prompt}\n");

    let chat = assistant::create(
        &conn,
        &profile_id,
        NewChat {
            work_id: Some(target.id.clone()),
            title: Some(template.label.clone()),
        },
    )?;

    let reply = assistant::ask(&mut conn, &chat.id, &prompt)?;
    println!("--- reply ---\n{}\n", reply.body);

    if let Some(cost) = reply.meta.get("cost_usd") {
        println!("cost: {cost}");
    }

    // The session id is what makes a second turn a continuation.
    let reloaded = assistant::get(&conn, &chat.id)?.ok_or("the chat vanished")?;
    println!("session: {}", reloaded.session_id.unwrap_or_default());

    Ok(())
}
