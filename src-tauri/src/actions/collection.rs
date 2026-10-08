//! Gestures on collections: an album, a book, a season.

use rusqlite::Connection;

use super::{BulkOutcome, Skipped, count, gesture};
use crate::collection::{self, Collection, CollectionPatch, NewCollection};
use crate::error::{Error, Reason, Result};
use crate::journal::Record;

pub fn create(conn: &Connection, new: NewCollection) -> Result<Collection> {
    gesture(conn, "collection.create", |act| {
        act.json("collection", &new)?;
        let minted = act.mint();
        let made = collection::create_minted(act, act.profile_id(), new, minted)?;
        act.journal(
            Record::new("collection.created")
                .param("title", made.title.clone())
                .about("collection", made.id.clone()),
        );
        Ok(made)
    })
}

pub fn update(conn: &Connection, id: &str, patch: CollectionPatch) -> Result<Collection> {
    gesture(conn, "collection.update", |act| {
        let before = collection::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();
        collection::update_at(act, id, patch, act.at())
    })
}

/// Put works in a collection in the order given; works not listed leave it.
///
/// What the screen sends after a drag, a removal or a pick: the whole list as
/// it is to stand. Every work the change touches - those it held and those it
/// is given - is recorded where it stood, so one undo puts the album back as
/// it was, a work taken from another album included. A work the profile does
/// not hold is dropped from the list rather than half placed, and the same
/// order sent twice is no change at all.
pub fn set_contents(conn: &Connection, id: &str, work_ids: &[String]) -> Result<()> {
    gesture(conn, "collection.setContents", |act| {
        let held = collection::get(act, id)?.ok_or_else(|| Error::not_found("collection", id))?;

        let mut listed: Vec<String> = Vec::new();
        for placement in collection::placements(act, act.profile_id(), work_ids)? {
            if !listed.contains(&placement.id) {
                listed.push(placement.id);
            }
        }
        if listed == held.work_ids {
            act.unchanged();
            return Ok(());
        }

        let mut touched = held.work_ids.clone();
        touched.extend(
            listed
                .iter()
                .filter(|id| !held.work_ids.contains(id))
                .cloned(),
        );
        let before = collection::placements(act, act.profile_id(), &touched)?;

        act.param("id", id);
        act.param("title", held.title.clone());
        act.json("workIds", &listed)?;
        act.json("before", &before)?;
        act.stamped();
        collection::set_contents_at(act, id, &listed, act.at())?;

        // A reordering is not news; what came in and what went out is.
        let added = listed
            .iter()
            .filter(|id| !held.work_ids.contains(id))
            .count();
        let removed = held
            .work_ids
            .iter()
            .filter(|id| !listed.contains(id))
            .count();
        // Each line written out whole, not chosen in a loop: the gate that
        // holds the journal's keys and holes to the locale reads them from
        // the source.
        if added > 0 {
            act.journal(
                Record::new("collection.added")
                    .param("title", held.title.clone())
                    .param("count", count(added))
                    .about("collection", id.to_owned()),
            );
        }
        if removed > 0 {
            act.journal(
                Record::new("collection.removed")
                    .param("title", held.title.clone())
                    .param("count", count(removed))
                    .about("collection", id.to_owned()),
            );
        }
        Ok(())
    })
}

/// Put works at the end of a collection: what the catalogue's bar, a drag
/// onto a collection and a work's own header do.
///
/// A batch, so it reports a batch: a work already in this collection is
/// passed over as already there - sending it to the end would reorder the
/// album as a side effect of adding something else - and a work that is not
/// there at all as not found. A work in another collection leaves it, and the
/// undo puts it back there.
pub fn add(conn: &Connection, id: &str, work_ids: &[String]) -> Result<BulkOutcome> {
    gesture(conn, "collection.add", |act| {
        let target = collection::get(act, id)?.ok_or_else(|| Error::not_found("collection", id))?;
        let found = collection::placements(act, act.profile_id(), work_ids)?;

        let mut added: Vec<String> = Vec::new();
        let mut skipped: Vec<Skipped> = Vec::new();
        for work_id in work_ids {
            if added.contains(work_id) || skipped.iter().any(|skip| &skip.id == work_id) {
                continue;
            }
            let reason = match found.iter().find(|placement| &placement.id == work_id) {
                None => Some(Reason::of("error.notFound")),
                Some(placement) if placement.collection_id.as_deref() == Some(id) => {
                    Some(Reason::of("skip.alreadyThere"))
                }
                Some(_) => None,
            };
            match reason {
                Some(reason) => skipped.push(Skipped {
                    id: work_id.clone(),
                    title: Some(act.title_of(work_id)).filter(|title| !title.is_empty()),
                    reason,
                }),
                None => added.push(work_id.clone()),
            }
        }

        if added.is_empty() {
            act.unchanged();
        } else {
            let before: Vec<_> = found
                .into_iter()
                .filter(|placement| added.contains(&placement.id))
                .collect();
            act.param("id", id);
            act.param("title", target.title.clone());
            act.json("workIds", &added)?;
            act.json("before", &before)?;
            act.stamped();
            collection::append_at(act, id, &added, act.at())?;
            act.journal(
                Record::new("collection.added")
                    .param("title", target.title)
                    .param("count", count(added.len()))
                    .about("collection", id.to_owned()),
            );
        }

        Ok(BulkOutcome {
            changed: added.len(),
            skipped,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::operation;

    fn album(conn: &Connection, title: &str) -> Collection {
        create(
            conn,
            NewCollection {
                kind: "album".into(),
                title: title.into(),
                description: None,
                meta: None,
                target_size: None,
                due_on: None,
            },
        )
        .unwrap()
    }

    fn contents(conn: &Connection, id: &str) -> Vec<String> {
        collection::get(conn, id).unwrap().unwrap().work_ids
    }

    #[test]
    fn adding_passes_over_what_is_there_already_and_what_is_not_there_at_all() {
        let (conn, profile_id) = fixtures::workspace();
        let target = album(&conn, "Album");
        let inside = fixtures::song(&conn, &profile_id, "Inside").id;
        let fresh = fixtures::song(&conn, &profile_id, "Fresh").id;
        set_contents(&conn, &target.id, std::slice::from_ref(&inside)).unwrap();

        let outcome = add(
            &conn,
            &target.id,
            &[inside.clone(), "gone".into(), fresh.clone(), fresh.clone()],
        )
        .unwrap();

        assert_eq!(outcome.changed, 1, "a work named twice is added once");
        let reasons: Vec<_> = outcome
            .skipped
            .iter()
            .map(|skip| (skip.id.as_str(), skip.reason.key.as_str()))
            .collect();
        assert_eq!(
            reasons,
            [
                (inside.as_str(), "skip.alreadyThere"),
                ("gone", "error.notFound")
            ]
        );
        assert_eq!(contents(&conn, &target.id), vec![inside, fresh.clone()]);
        let logged = operation::latest(&conn, 1).unwrap().remove(0);
        assert_eq!(logged.kind, "collection.add");
        assert_eq!(logged.params["workIds"], serde_json::json!([fresh]));
    }

    #[test]
    fn adding_nothing_new_records_nothing() {
        let (conn, profile_id) = fixtures::workspace();
        let target = album(&conn, "Album");
        let inside = fixtures::song(&conn, &profile_id, "Inside").id;
        set_contents(&conn, &target.id, std::slice::from_ref(&inside)).unwrap();
        let before = operation::count(&conn).unwrap();

        let outcome = add(&conn, &target.id, &[inside]).unwrap();

        assert_eq!(outcome.changed, 0);
        assert_eq!(operation::count(&conn).unwrap(), before);
        assert!(
            add(&conn, "nope", &[]).is_err(),
            "an unknown collection is refused"
        );
    }

    #[test]
    fn the_same_order_sent_again_is_no_change() {
        let (conn, profile_id) = fixtures::workspace();
        let target = album(&conn, "Album");
        let one = fixtures::song(&conn, &profile_id, "One").id;
        let two = fixtures::song(&conn, &profile_id, "Two").id;
        set_contents(&conn, &target.id, &[one.clone(), two.clone()]).unwrap();
        let before = operation::count(&conn).unwrap();

        set_contents(
            &conn,
            &target.id,
            &[one.clone(), "gone".into(), two.clone()],
        )
        .unwrap();

        assert_eq!(
            operation::count(&conn).unwrap(),
            before,
            "an id the profile does not hold is dropped, and what is left is the order there was"
        );
        assert_eq!(contents(&conn, &target.id), vec![one, two]);
    }

    #[test]
    fn one_undo_puts_an_arrangement_back_where_every_work_stood() {
        let (conn, profile_id) = fixtures::workspace();
        let first = album(&conn, "First");
        let second = album(&conn, "Second");
        let one = fixtures::song(&conn, &profile_id, "One").id;
        let two = fixtures::song(&conn, &profile_id, "Two").id;
        let borrowed = fixtures::song(&conn, &profile_id, "Borrowed").id;
        set_contents(&conn, &first.id, &[one.clone(), two.clone()]).unwrap();
        set_contents(&conn, &second.id, std::slice::from_ref(&borrowed)).unwrap();

        // Reordered, one taken out, one taken from the other album.
        set_contents(&conn, &first.id, &[borrowed.clone(), two.clone()]).unwrap();
        let offer = crate::undo::last(&conn)
            .unwrap()
            .expect("an arrangement can be undone");
        crate::undo::undo(&conn, &offer.operation_id).unwrap();

        assert_eq!(contents(&conn, &first.id), vec![one, two]);
        assert_eq!(contents(&conn, &second.id), vec![borrowed]);
    }

    #[test]
    fn one_undo_takes_an_addition_back_to_where_the_works_came_from() {
        let (conn, profile_id) = fixtures::workspace();
        let target = album(&conn, "Target");
        let other = album(&conn, "Other");
        let kept = fixtures::song(&conn, &profile_id, "Kept").id;
        let loose = fixtures::song(&conn, &profile_id, "Loose").id;
        let taken = fixtures::song(&conn, &profile_id, "Taken").id;
        let stays = fixtures::song(&conn, &profile_id, "Stays").id;
        set_contents(&conn, &target.id, std::slice::from_ref(&kept)).unwrap();
        set_contents(&conn, &other.id, &[stays.clone(), taken.clone()]).unwrap();

        add(&conn, &target.id, &[loose.clone(), taken.clone()]).unwrap();
        assert_eq!(
            contents(&conn, &target.id),
            vec![kept.clone(), loose.clone(), taken.clone()]
        );
        let offer = crate::undo::last(&conn)
            .unwrap()
            .expect("an addition can be undone");
        crate::undo::undo(&conn, &offer.operation_id).unwrap();

        assert_eq!(contents(&conn, &target.id), vec![kept]);
        assert_eq!(contents(&conn, &other.id), vec![stays, taken]);
        let back = crate::work::get(&conn, &loose).unwrap().unwrap();
        assert!(
            back.collection_id.is_none(),
            "a loose work goes back to being loose"
        );
    }
}
