//! Gestures on the boards of a work's conception (ADR 0065).
//!
//! A card is put on a board, written, judged - starred, turned down, or the
//! verdict taken back - and moved: dragged to another place or topic, or
//! carried along when a topic is renamed or merged into another. Putting and
//! changing are the card's own operations; a move of any kind is one
//! arrangement of the board, logged with the order before it, so a drag and
//! a rename are each taken back by one undo.

use rusqlite::Connection;

use super::gesture;
use crate::board::{self, BoardCard, CardPatch, NewCard, Placed, Source, Verdict};
use crate::error::{Error, Result};

/// Put a card on a board. Its place is settled before the operation is
/// logged, so a replay lands it where it landed here; its topic takes the
/// board's own spelling.
pub fn create(conn: &Connection, mut new: NewCard) -> Result<BoardCard> {
    gesture(conn, "card.create", |act| {
        new.topic = board::spelled(act, &new.work_id, &new.board, &new.topic)?;
        if new.position.is_none() {
            new.position = Some(board::next_position(act, &new.work_id, &new.board)?);
        }
        act.param("title", work_title(act, &new.work_id));
        act.json("card", &new)?;
        let minted = act.mint();
        board::create_minted(act, act.profile_id(), new, minted)
    })
}

/// Change a card, with what the fields held before, so it can be taken
/// back. A change to nothing records nothing.
pub fn update(conn: &Connection, id: &str, patch: CardPatch) -> Result<BoardCard> {
    gesture(conn, "card.update", |act| {
        let before = board::get(act, id)?.ok_or_else(|| Error::not_found("card", id))?;
        if patch.changes_nothing(&before) {
            act.unchanged();
            return Ok(before);
        }
        act.param("id", id);
        act.param("title", work_title(act, &before.work_id));
        act.json("patch", &patch)?;
        act.before(Some(&before), &patch)?;
        act.stamped();
        board::update_at(act, id, patch, act.at())
    })
}

/// The person's own card, in their words, on a board: the words go into the
/// first field of the kind, which is the card's headline.
pub fn add_own(
    conn: &Connection,
    work_id: &str,
    board_key: &str,
    kind: Option<&str>,
    words: &str,
) -> Result<BoardCard> {
    let words = words.trim();
    if words.is_empty() {
        return Err(Error::refused("board.emptyCard"));
    }
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let found = config.require_board(&work.kind, board_key)?;
    let card = match kind {
        Some(key) => found.card(key).ok_or_else(|| {
            Error::refused("board.notAKind")
                .param("kind", key)
                .param("board", found.label.as_str())
        })?,
        None => &found.cards[0],
    };
    let field = card.fields[0].key.clone();
    create(
        conn,
        NewCard {
            work_id: work_id.to_owned(),
            board: board_key.to_owned(),
            kind: card.key.clone(),
            topic: String::new(),
            position: None,
            source: Source::Own,
            verdict: None,
            fields: [(field, words.to_owned())].into_iter().collect(),
        },
    )
}

/// Star a card, turn it down, or take the verdict back. Saying the verdict
/// it already has changes nothing, and records nothing.
pub fn judge(conn: &Connection, id: &str, verdict: Option<Verdict>) -> Result<BoardCard> {
    update(
        conn,
        id,
        CardPatch {
            verdict: Some(verdict),
            ..CardPatch::default()
        },
    )
}

/// Stand a board's cards in `order`, each under its topic. The list names
/// every card of the board once; an arrangement that is the one standing
/// records nothing.
pub fn arrange(
    conn: &Connection,
    work_id: &str,
    board_key: &str,
    order: Vec<Placed>,
) -> Result<()> {
    gesture(conn, "card.arrange", |act| {
        let before = board::arrangement(act, work_id, board_key)?;
        let order: Vec<Placed> = order
            .into_iter()
            .map(|placed| {
                Ok(Placed {
                    topic: board::spelled(act, work_id, board_key, &placed.topic)?,
                    id: placed.id,
                })
            })
            .collect::<Result<_>>()?;
        if before == order {
            act.unchanged();
            return Ok(());
        }
        act.param("workId", work_id);
        act.param("board", board_key);
        act.param("title", work_title(act, work_id));
        act.json("order", &order)?;
        act.json("before", &before)?;
        act.stamped();
        board::arrange(act, work_id, board_key, &order, act.at())
    })
}

/// Move a card under `topic`, to `index` among that topic's cards - where a
/// drop puts it.
pub fn move_card(conn: &Connection, id: &str, topic: &str, index: usize) -> Result<()> {
    let card = board::get(conn, id)?.ok_or_else(|| Error::not_found("card", id))?;
    let standing = board::arrangement(conn, &card.work_id, &card.board)?;
    let topic = board::spelled(conn, &card.work_id, &card.board, topic)?;
    let order = board::moved(&standing, id, &topic, index)?;
    arrange(conn, &card.work_id, &card.board, order)
}

/// Rename a topic of a board, or merge it into another by giving it that
/// one's name.
pub fn retopic(
    conn: &Connection,
    work_id: &str,
    board_key: &str,
    from: &str,
    to: &str,
) -> Result<()> {
    let standing = board::arrangement(conn, work_id, board_key)?;
    if !standing.iter().any(|placed| placed.topic == from) {
        return Err(Error::refused("board.noSuchTopic").param("topic", from));
    }
    arrange(
        conn,
        work_id,
        board_key,
        board::retopicked(&standing, from, to),
    )
}

/// The title of the work a board is on, for the journal's sentence.
fn work_title(conn: &Connection, work_id: &str) -> String {
    crate::work::get(conn, work_id)
        .ok()
        .flatten()
        .map(|work| work.title)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    /// A Studio workspace with a song, whose board of scenarios is the one
    /// these tests work on.
    fn song() -> (Connection, String) {
        let (conn, profile_id) = fixtures::workspace();
        let work = fixtures::song(&conn, &profile_id, "A Song");
        (conn, work.id)
    }

    fn undo_last(conn: &Connection) {
        let offer = crate::undo::last(conn).unwrap().unwrap();
        crate::undo::undo(conn, &offer.operation_id).unwrap();
    }

    fn scenario(conn: &Connection, work_id: &str, line: &str, topic: &str) -> BoardCard {
        create(
            conn,
            NewCard {
                work_id: work_id.to_owned(),
                board: "scenarios".into(),
                kind: "scenario".into(),
                topic: topic.into(),
                position: None,
                source: Source::Ai,
                verdict: None,
                fields: [("line".to_owned(), line.to_owned())].into_iter().collect(),
            },
        )
        .unwrap()
    }

    fn standing(conn: &Connection, work_id: &str) -> Vec<String> {
        board::for_board(conn, work_id, "scenarios")
            .unwrap()
            .into_iter()
            .map(|card| format!("{}:{}", card.fields["line"], card.topic))
            .collect()
    }

    /// A star and back, each one operation, each undone to the verdict
    /// before it; saying the same verdict twice records nothing.
    #[test]
    fn a_verdict_is_logged_and_taken_back() {
        let (conn, work_id) = song();
        let card = scenario(&conn, &work_id, "A keeper counts ships", "");
        judge(&conn, &card.id, Some(Verdict::Star)).unwrap();
        let logged = crate::operation::count(&conn).unwrap();
        judge(&conn, &card.id, Some(Verdict::Star)).unwrap();
        assert_eq!(
            crate::operation::count(&conn).unwrap(),
            logged,
            "the same verdict is nothing"
        );
        undo_last(&conn);
        assert_eq!(board::get(&conn, &card.id).unwrap().unwrap().verdict, None);
    }

    /// A card is held to its board: a kind it does not hold, a field its
    /// kind does not name, a work whose kind lays out no such board.
    #[test]
    fn a_card_is_held_to_its_board() {
        let (conn, work_id) = song();
        let mut new = NewCard {
            work_id: work_id.clone(),
            board: "scenarios".into(),
            kind: "poem".into(),
            topic: String::new(),
            position: None,
            source: Source::Own,
            verdict: None,
            fields: [("line".to_owned(), "x".to_owned())].into_iter().collect(),
        };
        assert!(
            create(&conn, new.clone()).is_err(),
            "a kind the board does not hold"
        );
        new.kind = "scenario".into();
        new.fields = [("mood".to_owned(), "x".to_owned())].into_iter().collect();
        assert!(
            create(&conn, new.clone()).is_err(),
            "a field the kind does not name"
        );
        new.fields = [("line".to_owned(), "  ".to_owned())].into_iter().collect();
        assert!(
            create(&conn, new.clone()).is_err(),
            "a card that says nothing"
        );
        new.board = "sources".into();
        new.fields = [("line".to_owned(), "x".to_owned())].into_iter().collect();
        assert!(
            create(&conn, new).is_err(),
            "a board the kind does not lay out"
        );
    }

    /// A drag and a rename are each one arrangement, undone whole; a topic
    /// is one group whoever spells it, and renaming one onto another merges
    /// them.
    #[test]
    fn moves_are_one_arrangement_each_and_undone_whole() {
        let (conn, work_id) = song();
        let a = scenario(&conn, &work_id, "a", "Sea");
        scenario(&conn, &work_id, "b", "sea");
        let c = scenario(&conn, &work_id, "c", "Town");
        assert_eq!(standing(&conn, &work_id), ["a:Sea", "b:Sea", "c:Town"]);

        move_card(&conn, &a.id, "Town", 1).unwrap();
        assert_eq!(standing(&conn, &work_id), ["b:Sea", "c:Town", "a:Town"]);
        undo_last(&conn);
        assert_eq!(standing(&conn, &work_id), ["a:Sea", "b:Sea", "c:Town"]);

        retopic(&conn, &work_id, "scenarios", "Town", "SEA").unwrap();
        assert_eq!(standing(&conn, &work_id), ["a:Sea", "b:Sea", "c:Sea"]);
        undo_last(&conn);
        assert_eq!(standing(&conn, &work_id), ["a:Sea", "b:Sea", "c:Town"]);

        let logged = crate::operation::count(&conn).unwrap();
        move_card(&conn, &c.id, "Town", 0).unwrap();
        assert_eq!(
            crate::operation::count(&conn).unwrap(),
            logged,
            "a drop where it stands is nothing"
        );
    }

    /// Undoing a card put on the board sends it to the trash, and a field
    /// written later is not taken back with an earlier one.
    #[test]
    fn a_card_is_undone_into_the_trash_and_its_fields_one_by_one() {
        let (conn, work_id) = song();
        let card = scenario(&conn, &work_id, "a", "");
        let set = |key: &str, text: &str| {
            update(
                &conn,
                &card.id,
                CardPatch {
                    fields: Some(
                        [(key.to_owned(), serde_json::Value::from(text))]
                            .into_iter()
                            .collect(),
                    ),
                    ..CardPatch::default()
                },
            )
            .unwrap()
        };
        set("image", "a lantern");
        set("ending", "it goes out");
        undo_last(&conn);
        let fields = board::get(&conn, &card.id).unwrap().unwrap().fields;
        assert_eq!(fields.get("image").map(String::as_str), Some("a lantern"));
        assert_eq!(fields.get("ending"), None);

        // Only the newest operation is ever undone: a card put on the board
        // and taken back at once.
        let other = scenario(&conn, &work_id, "b", "");
        let offer = crate::undo::last(&conn).unwrap().unwrap();
        assert_eq!(offer.action, "undo.card.create");
        undo_last(&conn);
        assert!(board::get(&conn, &other.id).unwrap().is_none());
        let entry = crate::trash::list(&conn, &other.profile_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.entity_id == other.id)
            .expect("the card waits in the trash");
        assert_eq!(entry.label, "b");
        crate::trash::restore(&conn, &entry.id).unwrap();
        assert!(board::get(&conn, &other.id).unwrap().is_some());
    }
}
