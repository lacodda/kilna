//! Every ordering by a timestamp needs something to break the tie.
//!
//! Timestamps are text, and two rows written close enough together share one.
//! What comes back then is whatever the query planner felt like — which is not
//! an order at all, and shows up as a shuffled transcript or a work whose score
//! flickers between two snapshots.
//!
//! This has now been found twice by the same CI runner and never by reading the
//! code: in August a chat transcript ordered by `created_at, id` where `id` is a
//! random uuid, and in September a score history ordered by `scored_at` alone.
//! Both times macOS was fast enough to write two rows inside one tick while
//! every other runner was not. A rule is cheaper than a third occurrence.

use std::path::{Path, PathBuf};

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Columns that hold a timestamp, so an `ORDER BY` on one needs a tie-break.
const TIMESTAMPS: [&str; 7] = [
    "created_at",
    "updated_at",
    "scored_at",
    "started_at",
    "deleted_at",
    "released_at",
    "scheduled_at",
];

/// What counts as breaking the tie.
///
/// `rowid` is insertion order and SQLite keeps it for free. A column with its
/// own meaning — a revision number, a title, a profile key — settles the order
/// just as well. A uuid does not, which is exactly the August bug, so `id` is
/// deliberately absent from this list.
const TIE_BREAKS: [&str; 6] = ["rowid", "revision", "title", "key", "tag", "name"];

/// The `ORDER BY` clauses in one file, as written.
fn orderings(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (index, _) in source.match_indices("ORDER BY") {
        let rest = &source[index..];
        // A clause ends at the statement's end or at what follows it in the
        // SQL: a limit, a closing paren, or the end of the string literal.
        let end = ["LIMIT", "\"", "')", "\n"]
            .iter()
            .filter_map(|stop| rest.find(stop))
            .min()
            .unwrap_or(rest.len());
        found.push(rest[..end].replace(['\r', '\n'], " ").trim().to_owned());
    }
    found
}

#[test]
fn every_ordering_by_a_timestamp_breaks_its_ties() {
    let mut offenders = Vec::new();

    fn walk(dir: &Path, offenders: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("src is readable") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                walk(&path, offenders);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }

            let source = std::fs::read_to_string(&path).expect("readable source");
            for clause in orderings(&source) {
                let orders_by_time = TIMESTAMPS.iter().any(|column| clause.contains(column));
                if !orders_by_time {
                    continue;
                }
                // The tie-break has to come after the timestamp, not be the
                // timestamp itself; anything in the list appearing anywhere
                // else in the clause is enough to settle it.
                let settled = TIE_BREAKS.iter().any(|tie| clause.contains(tie));
                if !settled {
                    offenders.push(format!(
                        "{}: {clause}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                    ));
                }
            }
        }
    }

    walk(&src(), &mut offenders);

    assert!(
        offenders.is_empty(),
        "these order by a timestamp with nothing to break the tie, so rows written in the same instant come back in an arbitrary order:\n  {}",
        offenders.join("\n  ")
    );
}

/// A uuid is not a tie-break, however much it looks like one.
///
/// `ORDER BY created_at, id` reads as deliberate and orders nothing: the id is
/// random. This is the August bug, kept here by name so that writing it again
/// fails rather than passing review.
#[test]
fn no_ordering_falls_back_to_an_id() {
    let mut offenders = Vec::new();

    fn walk(dir: &Path, offenders: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("src is readable") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                walk(&path, offenders);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }

            let source = std::fs::read_to_string(&path).expect("readable source");
            for clause in orderings(&source) {
                // The last thing an ordering falls back to is what settles it.
                let last = clause
                    .rsplit(',')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_end_matches("DESC")
                    .trim_end_matches("ASC")
                    .trim();
                if last == "id" || last.ends_with(".id") {
                    offenders.push(format!(
                        "{}: {clause}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                    ));
                }
            }
        }
    }

    walk(&src(), &mut offenders);

    assert!(
        offenders.is_empty(),
        "these fall back to a uuid, which is not an order:\n  {}",
        offenders.join("\n  ")
    );
}
