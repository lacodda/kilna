//! A one-off live check of the batch path, run by hand rather than in CI.
//!
//! It spawns real Claude Code processes, so it is `#[ignore]`d: `cargo test
//! --test live_queue -- --ignored --nocapture` runs it deliberately.

use std::sync::Arc;

use kilna_lib::assistant::queue::{Pending, Queue};
use kilna_lib::assistant::run::{PARALLEL_LIMIT, Runs};
use kilna_lib::assistant::{self, task};
use kilna_lib::work::version::{self, NewVersion};
use kilna_lib::work::{self, NewWork};
use kilna_lib::{db, profile};

#[test]
#[ignore = "spawns real CLI processes"]
fn a_batch_larger_than_the_limit_starts_some_and_queues_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = db::open(&dir.path().join("workspace.db")).unwrap();
    profile::seed(&conn).unwrap();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;

    let action = profile::active(&conn)
        .unwrap()
        .unwrap()
        .config
        .prompts
        .first()
        .cloned()
        .unwrap()
        .key;

    // Five works, two more than the limit.
    let mut work_ids = Vec::new();
    for index in 0..5 {
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: format!("Harbour lights {index}"),
                ..Default::default()
            },
        )
        .unwrap();
        version::create(
            &mut conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "the cranes go still above the water".into(),
                label: None,
                meta: None,
                make_current: true,
            },
        )
        .unwrap();
        work_ids.push(work.id);
    }

    let runs = Arc::new(Runs::new());
    let queue = Queue::new();
    let mut started = 0usize;
    let mut streams = Vec::new();

    // Exactly what `start_tasks` does per work.
    for work_id in &work_ids {
        if runs.has_slot() {
            let prepared = task::prepare(&conn, work_id, &action).unwrap();
            let (_run, stream) = assistant::run::start_as(
                &conn,
                &runs,
                &prepared.chat_id,
                &prepared.prompt,
                None,
                Some(prepared.key),
            )
            .unwrap();
            // Kept alive for the length of the check: dropping the stream
            // would end the process and free the slot we are measuring.
            streams.push(stream);
            started += 1;
        } else {
            queue.push(Pending {
                work_id: work_id.clone(),
                action: action.clone(),
            });
        }
    }

    println!(
        "started={started} queued={} limit={PARALLEL_LIMIT}",
        queue.len()
    );

    assert_eq!(
        started, PARALLEL_LIMIT,
        "the slots must be filled before anything queues"
    );
    assert_eq!(
        queue.len(),
        work_ids.len() - PARALLEL_LIMIT,
        "everything past the limit waits rather than being refused"
    );
    assert!(!runs.has_slot(), "no slot is left while three are alive");

    // Every queued task is findable by the key its button uses.
    for work_id in &work_ids[PARALLEL_LIMIT..] {
        assert!(queue.holds(&task::key(&action, work_id)));
    }

    // Everything asked of the machine is stopped: this check is about who got
    // a slot, not about what the assistant would have answered.
    let stopped = runs.stop_all();
    println!("stopped {stopped} live runs");
    drop(streams);
}
