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
use crate::board::answer::Packaged;
use crate::board::{self, BoardCard, CardPatch, NewCard, Placed, Source, Verdict};
use crate::error::{Error, Result};
use crate::library::{Asked, Entry};
use crate::note::{NewNote, Note, NotePatch};

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
/// first field of the kind, which is the card's headline - or, on a kind
/// that links to the library, they name the work of culture, found in the
/// library or written into it, and the card says nothing more yet.
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
    crate::db::unit::atomically(conn, |conn| {
        let (fields, note_id) = if card.library {
            let entry = entry_for(
                conn,
                &Asked {
                    title: words.to_owned(),
                    ..Asked::default()
                },
            )?;
            (Default::default(), Some(entry.id))
        } else {
            let field = card.fields[0].key.clone();
            ([(field, words.to_owned())].into_iter().collect(), None)
        };
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
                fields,
                note_id,
            },
        )
    })
}

/// A card an answer or an agent proposed, put on the board: on a kind that
/// links to the library, the work of culture it names is found there - its
/// blanks filled from what the answer says - or written into it first, as a
/// gesture of its own, so an undo of the card leaves the library as it grew.
/// A card proposed starred lands starred: an agent carrying over research
/// the person already chose from (ADR 0066).
pub fn put(conn: &Connection, work_id: &str, board_key: &str, card: Packaged) -> Result<BoardCard> {
    crate::db::unit::atomically(conn, |conn| {
        let work =
            crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
        let config = crate::profile::config_for(conn, &work.profile_id)?;
        let laid = config.require_board(&work.kind, board_key)?;
        let kind = laid.card(&card.kind).ok_or_else(|| {
            Error::refused("board.notAKind")
                .param("kind", card.kind.clone())
                .param("board", laid.label.as_str())
        })?;
        let note_id = match (kind.library, &card.entry) {
            (true, Some(asked)) => Some(entry_for(conn, asked)?.id),
            (true, None) => {
                return Err(Error::refused("board.needsEntry").param("kind", kind.label.as_str()));
            }
            (false, _) => None,
        };
        create(
            conn,
            NewCard {
                work_id: work_id.to_owned(),
                board: board_key.to_owned(),
                kind: card.kind,
                topic: card.topic,
                position: None,
                source: card.source,
                verdict: card.starred.then_some(Verdict::Star),
                fields: card.fields,
                note_id,
            },
        )
    })
}

/// The entry of the library a card asks for: the one the library already
/// has under one of its names - what it does not say yet filled in from the
/// answer, a name it did not know kept as another title - or a new one. A
/// medium the library does not name is left out.
pub fn entry_for(conn: &Connection, asked: &Asked) -> Result<Entry> {
    let title = asked.title.trim();
    if title.is_empty() {
        return Err(Error::refused("library.needsTitle"));
    }
    let profile_id = super::active_profile_id(conn)?;
    let config = crate::profile::config_for(conn, &profile_id)?;
    let library = config
        .library()
        .ok_or_else(|| Error::refused("library.none"))?;
    let said = |text: &Option<String>| {
        text.as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    let medium = said(&asked.medium).filter(|key| library.medium(key).is_some());
    match crate::library::find(conn, &profile_id, asked)? {
        Some(entry) => {
            let mut patch = NotePatch::default();
            let mut changed = false;
            if entry.author.is_none() && said(&asked.author).is_some() {
                patch.attribution = Some(said(&asked.author));
                changed = true;
            }
            if entry.year.is_none() && said(&asked.year).is_some() {
                patch.year = Some(said(&asked.year));
                changed = true;
            }
            if entry.medium.is_none() && medium.is_some() {
                patch.medium = Some(medium.clone());
                changed = true;
            }
            let mut aliases = entry.aliases.clone();
            for name in [Some(title.to_owned()), said(&asked.original)]
                .into_iter()
                .flatten()
            {
                let known = entry.knows(&name)
                    || aliases
                        .iter()
                        .any(|alias| crate::words::plain(alias) == crate::words::plain(&name));
                if !known {
                    aliases.push(name);
                }
            }
            if aliases != entry.aliases {
                patch.aliases = Some(aliases);
                changed = true;
            }
            if !changed {
                return Ok(entry);
            }
            let note = super::note::update(conn, &entry.id, patch)?;
            Ok(Entry::of(&note))
        }
        None => {
            let note = super::note::create(
                conn,
                NewNote {
                    body: String::new(),
                    kind: Some(library.key.clone()),
                    title: Some(title.to_owned()),
                    attribution: said(&asked.author),
                    aliases: said(&asked.original)
                        .filter(|original| {
                            crate::words::plain(original) != crate::words::plain(title)
                        })
                        .into_iter()
                        .collect(),
                    medium,
                    year: said(&asked.year),
                    ..NewNote::default()
                },
            )?;
            Ok(Entry::of(&note))
        }
    }
}

/// A chip of a card - an image, a name, a place - kept as a phrase of the
/// work the card is on (ADR 0066): the phrase already written in these words
/// joins the work, a new one is written for it. A phrase kept for another
/// work stays where it is and says so: a phrase is kept for one work.
pub fn chip_to_phrase(conn: &Connection, card_id: &str, chip: &str) -> Result<Note> {
    let chip = chip.trim();
    if chip.is_empty() {
        return Err(Error::refused("board.emptyChip"));
    }
    let card = board::get(conn, card_id)?.ok_or_else(|| Error::not_found("card", card_id))?;
    let config = crate::profile::config_for(conn, &card.profile_id)?;
    let Some(kind) = config
        .note_kinds
        .iter()
        .find(|kind| kind.line && kind.material)
    else {
        return Err(Error::refused("board.noPhrases"));
    };
    crate::db::unit::atomically(conn, |conn| {
        match crate::lines::find(conn, &card.profile_id, chip)? {
            Some(found) if found.work_id.as_deref() == Some(card.work_id.as_str()) => Ok(found),
            Some(found) if found.work_id.is_none() => super::note::update(
                conn,
                &found.id,
                NotePatch {
                    work_id: Some(Some(card.work_id.clone())),
                    ..NotePatch::default()
                },
            ),
            Some(found) => Err(Error::refused("board.chipKeptElsewhere")
                .param("phrase", found.body.clone())
                .param(
                    "title",
                    work_title(conn, found.work_id.as_deref().unwrap_or_default()),
                )),
            None => super::note::create(
                conn,
                NewNote {
                    body: chip.to_owned(),
                    kind: Some(kind.key.clone()),
                    work_id: Some(card.work_id.clone()),
                    ..NewNote::default()
                },
            ),
        }
    })
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

    /// A card of the research board naming a work of culture, as an answer
    /// gives it.
    fn source(title: &str, author: Option<&str>, why: &str) -> Packaged {
        Packaged {
            kind: "work".into(),
            topic: String::new(),
            source: Source::Ai,
            fields: [("why".to_owned(), why.to_owned())].into_iter().collect(),
            entry: Some(Asked {
                title: title.into(),
                author: author.map(str::to_owned),
                medium: Some("film".into()),
                ..Asked::default()
            }),
            starred: false,
        }
    }

    /// The same work of culture on two works' boards is one entry of the
    /// library: found by a name it goes by, its blanks filled, a name it did
    /// not know kept - and each card says where else it has worked.
    #[test]
    fn a_work_of_culture_is_one_entry_wherever_it_works() {
        let (conn, first) = song();
        let profile_id = crate::work::get(&conn, &first).unwrap().unwrap().profile_id;
        let second = fixtures::song(&conn, &profile_id, "Another Song").id;

        let here = put(
            &conn,
            &first,
            "research",
            source("Old Joy", None, "two friends"),
        )
        .unwrap();
        let mut again = source("old joy", Some("Kelly Reichardt"), "the hot spring");
        again.entry.as_mut().unwrap().original = Some("Старая радость".into());
        let there = put(&conn, &second, "research", again).unwrap();

        assert_eq!(here.note_id, there.note_id, "one entry, not two");
        let entry = there.entry.unwrap();
        assert_eq!(
            entry.author.as_deref(),
            Some("Kelly Reichardt"),
            "a blank filled"
        );
        assert_eq!(
            entry.title, "Old Joy",
            "the title it was written under stays"
        );
        assert!(entry.knows("Старая радость"), "a name it did not know kept");
        assert_eq!(
            crate::library::entries(&conn, &profile_id).unwrap().len(),
            1
        );

        let elsewhere = crate::library::elsewhere(&conn, &first, "research").unwrap();
        assert_eq!(elsewhere.len(), 1);
        assert_eq!(elsewhere[0].works[0].title, "Another Song");

        // Turned down there, it has not worked there.
        judge(&conn, &there.id, Some(Verdict::Rejected)).unwrap();
        assert!(
            crate::library::elsewhere(&conn, &first, "research")
                .unwrap()
                .is_empty()
        );
    }

    /// A card of the library is held to it: a kind that links to the library
    /// needs an entry, another kind links to nothing, a proof is a link, and
    /// the person's own words on such a kind name the entry.
    #[test]
    fn a_card_of_the_library_is_held_to_it() {
        let (conn, work_id) = song();
        let mut new = NewCard {
            work_id: work_id.clone(),
            board: "research".into(),
            kind: "work".into(),
            topic: String::new(),
            position: None,
            source: Source::Own,
            verdict: None,
            fields: [("why".to_owned(), "x".to_owned())].into_iter().collect(),
            note_id: None,
        };
        let code = |result: Result<BoardCard>| result.unwrap_err().refusal().unwrap().code;
        assert_eq!(code(create(&conn, new.clone())), "board.needsEntry");

        let own = add_own(&conn, &work_id, "research", Some("work"), "Frances Ha").unwrap();
        assert_eq!(own.entry.as_ref().unwrap().title, "Frances Ha");
        assert!(
            own.fields.is_empty(),
            "an entry is enough to stand on the board"
        );

        new.note_id = own.note_id.clone();
        new.fields.insert("proof".into(), "checked".into());
        assert_eq!(code(create(&conn, new.clone())), "board.proofNotALink");

        new.kind = "concept".into();
        new.fields = [("term".to_owned(), "x".to_owned())].into_iter().collect();
        assert_eq!(code(create(&conn, new)), "board.noEntryHere");
    }

    /// An entry a board still links to is not deleted; with the card gone, it
    /// is.
    #[test]
    fn an_entry_a_board_links_to_stays() {
        let (conn, work_id) = song();
        let card = put(&conn, &work_id, "research", source("Old Joy", None, "x")).unwrap();
        let note_id = card.note_id.clone().unwrap();
        let refused = crate::actions::trash::discard(&conn, crate::trash::Entity::Note, &note_id)
            .unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "library.onBoards");
        crate::actions::trash::discard(&conn, crate::trash::Entity::Card, &card.id).unwrap();
        crate::actions::trash::discard(&conn, crate::trash::Entity::Note, &note_id).unwrap();
    }

    /// A chip becomes a phrase of the work: a new one, or the phrase already
    /// written in those words joining the work; a phrase kept for another
    /// work stays there and says so.
    #[test]
    fn a_chip_becomes_a_phrase_of_the_work() {
        let (conn, work_id) = song();
        let profile_id = crate::work::get(&conn, &work_id)
            .unwrap()
            .unwrap()
            .profile_id;
        let mut card = source("Inferno", Some("Dante"), "the hypocrites");
        card.fields
            .insert("chips".into(), "gilded lead cloaks; Arzamas".into());
        let card = put(&conn, &work_id, "research", card).unwrap();

        let made = chip_to_phrase(&conn, &card.id, "gilded lead cloaks").unwrap();
        assert_eq!(made.work_id.as_deref(), Some(work_id.as_str()));
        assert_eq!(made.kind, "phrase");
        let again = chip_to_phrase(&conn, &card.id, "Gilded lead cloaks").unwrap();
        assert_eq!(again.id, made.id, "the same words are one phrase");

        let loose = crate::actions::note::create(
            &conn,
            NewNote {
                body: "Arzamas".into(),
                kind: Some("phrase".into()),
                ..NewNote::default()
            },
        )
        .unwrap();
        let joined = chip_to_phrase(&conn, &card.id, "arzamas").unwrap();
        assert_eq!(joined.id, loose.id);
        assert_eq!(joined.work_id.as_deref(), Some(work_id.as_str()));

        let other = fixtures::song(&conn, &profile_id, "Another Song").id;
        let elsewhere = put(&conn, &other, "research", source("Inferno", None, "y")).unwrap();
        let refused = chip_to_phrase(&conn, &elsewhere.id, "Arzamas").unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "board.chipKeptElsewhere");
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
                note_id: None,
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
            note_id: None,
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
