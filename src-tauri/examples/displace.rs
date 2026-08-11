//! Demonstrates what happens when two releases want the same slot.
//!
//!   cargo run --example displace -- <path-to-kilna.db> <slot>
//!
//! The stronger work takes the date; the weaker one keeps its plan and returns
//! to the queue. Nothing is deleted by losing a slot.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("usage: displace <db> <slot>")?;
    let slot = args.next().ok_or("usage: displace <db> <slot>")?;

    let mut conn = kilna_lib::db::open(std::path::Path::new(&path))?;
    let profile_id = kilna_lib::profile::active(&conn)?
        .ok_or("no active profile")?
        .id;

    let queued = kilna_lib::release::queue(&conn, &profile_id)?;
    let Some(challenger) = queued.first() else {
        println!("the queue is empty — nothing to schedule");
        return Ok(());
    };

    println!(
        "challenger: {} ({})",
        challenger.work_title,
        challenger
            .total
            .map(|total| format!("{total:.1}"))
            .unwrap_or_else(|| "unscored".into())
    );

    match kilna_lib::release::schedule(&mut conn, &challenger.release.id, &slot) {
        Ok(result) => match result.displaced {
            Some(displaced) => println!(
                "took {slot}, displacing `{}` — it is back in the queue",
                displaced.title.as_deref().unwrap_or(&displaced.work_id)
            ),
            None => println!("took {slot}, the slot was free"),
        },
        Err(error) => println!("refused: {error}"),
    }

    println!("\ncalendar");
    for entry in kilna_lib::release::calendar(&conn, &profile_id)? {
        println!(
            "  {}  {:<20} {}",
            entry.release.scheduled_at.as_deref().unwrap_or("-"),
            entry.work_title,
            entry
                .total
                .map(|total| format!("{total:.1}"))
                .unwrap_or_else(|| "-".into())
        );
    }

    println!("\nqueue");
    for entry in kilna_lib::release::queue(&conn, &profile_id)? {
        println!(
            "  {:<20} {}",
            entry.work_title,
            entry
                .total
                .map(|total| format!("{total:.1}"))
                .unwrap_or_else(|| "unscored".into())
        );
    }

    Ok(())
}
