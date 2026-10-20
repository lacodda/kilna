//! The boards of a work's conception (v0.99, ADR 0065).
//!
//! A work is prepared before it is written, and what it is prepared from
//! is gathered on boards the profile names - scenarios to write it by,
//! sources to draw on. Every board is the same mechanism with the craft's
//! words: cards of the kinds the board holds, each saying its fields,
//! standing in order and under a topic, the person's own or the
//! assistant's, starred, turned down or not judged yet. A card is a row of
//! `board_card` (migration 0042).
//!
//! What the assistant answers - and what an agent outside the window
//! proposes - is read by [`answer::read`] against the board's kinds of card
//! and their fields; what is asked of it is [`answer::request_sheet`].

pub mod answer;

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::profile::config::{Board, CardKind, ProfileConfig};
use crate::work::Work;

/// Where a card came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "CardSource")]
pub enum Source {
    /// The person's own card, in their words.
    Own,
    /// One of the person's own worked out by the assistant.
    Refined,
    /// The assistant's own.
    Ai,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Own => "own",
            Source::Refined => "refined",
            Source::Ai => "ai",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "own" => Some(Source::Own),
            "refined" => Some(Source::Refined),
            "ai" => Some(Source::Ai),
            _ => None,
        }
    }
}

/// What the person said about a card. None of them is "not judged yet".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "CardVerdict")]
pub enum Verdict {
    /// Wanted: a scenario to write by, a source to draw on.
    Star,
    /// "Not that": left out of the order, and an example of what is not
    /// wanted when the board asks for more.
    Rejected,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Star => "star",
            Verdict::Rejected => "rejected",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "star" => Some(Verdict::Star),
            "rejected" => Some(Verdict::Rejected),
            _ => None,
        }
    }
}

/// One card on a work's board.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct BoardCard {
    pub id: String,
    pub profile_id: String,
    /// The work whose board it is on.
    pub work_id: String,
    /// The board, by its key in the profile.
    pub board: String,
    /// The kind of card, by its key on the board.
    pub kind: String,
    /// The group it stands in; empty is none.
    pub topic: String,
    /// Where it stands on the whole board.
    pub position: i64,
    pub source: Source,
    pub verdict: Option<Verdict>,
    /// What it says, by the field keys of its kind.
    pub fields: BTreeMap<String, String>,
    pub created_at: String,
    pub updated_at: String,
}

impl BoardCard {
    /// What the card is called: the first of its kind's fields that says
    /// anything, or the first field of any it holds for a kind the board no
    /// longer names.
    pub fn headline(&self, board: Option<&Board>) -> String {
        let kind = board.and_then(|board| board.card(&self.kind));
        let ordered: Vec<&str> = match kind {
            Some(kind) => kind.fields.iter().map(|field| field.key.as_str()).collect(),
            None => self.fields.keys().map(String::as_str).collect(),
        };
        ordered
            .into_iter()
            .filter_map(|key| self.fields.get(key))
            .map(|text| text.trim())
            .find(|text| !text.is_empty())
            .unwrap_or_default()
            .to_owned()
    }
}

/// A card to put on a board - what the operations log carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct NewCard {
    pub work_id: String,
    pub board: String,
    pub kind: String,
    #[serde(default)]
    pub topic: String,
    /// Where it stands; absent puts it after the last. Settled before the
    /// operation is logged, so a replay lands it where it landed live.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub position: Option<i64>,
    pub source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub verdict: Option<Verdict>,
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

/// What may change about a card; `None` leaves a part alone. `fields` is
/// merged by key, as a work's `meta` is: a key with words sets that field,
/// a key with `null` empties it, a key not named is left as it is - so an
/// undo of one field never takes back another written after it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct CardPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub verdict: Option<Option<Verdict>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "Record<string, string | null>")]
    pub fields: Option<Map<String, Value>>,
}

impl CardPatch {
    /// Whether applying it to `card` would leave it as it is.
    pub fn changes_nothing(&self, card: &BoardCard) -> bool {
        self.kind.as_ref().is_none_or(|kind| *kind == card.kind)
            && self
                .topic
                .as_ref()
                .is_none_or(|topic| topic.trim() == card.topic)
            && self
                .position
                .is_none_or(|position| position == card.position)
            && self.verdict.is_none_or(|verdict| verdict == card.verdict)
            && self
                .fields
                .as_ref()
                .is_none_or(|fields| merged(&card.fields, fields) == card.fields)
    }
}

/// Where a card stands, as an arrangement of a board names it: its place is
/// its index in the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Placed {
    pub id: String,
    pub topic: String,
}

const SELECT: &str = "SELECT id, profile_id, work_id, board, kind, topic, position, source, \
                      verdict, fields, created_at, updated_at FROM board_card";

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BoardCard> {
    let source: String = row.get(7)?;
    let verdict: Option<String> = row.get(8)?;
    let fields: String = row.get(9)?;
    Ok(BoardCard {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        work_id: row.get(2)?,
        board: row.get(3)?,
        kind: row.get(4)?,
        topic: row.get(5)?,
        position: row.get(6)?,
        // The schema's CHECK holds both words to the three and the two; a
        // word this build does not know reads as the assistant's own rather
        // than failing the whole board.
        source: Source::parse(&source).unwrap_or(Source::Ai),
        verdict: verdict.as_deref().and_then(Verdict::parse),
        fields: texts(&serde_json::from_str(&fields).unwrap_or(Value::Null)),
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

/// The texts of a JSON object, by key; anything else in it reads as nothing.
fn texts(value: &Value) -> BTreeMap<String, String> {
    value
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}

/// `fields` with `patch` merged in by key: words set a field, `null` or
/// nothing but space empties it.
fn merged(
    fields: &BTreeMap<String, String>,
    patch: &Map<String, Value>,
) -> BTreeMap<String, String> {
    let mut out = fields.clone();
    for (key, value) in patch {
        match value.as_str().map(str::trim) {
            Some(text) if !text.is_empty() => {
                out.insert(key.clone(), text.to_owned());
            }
            _ => {
                out.remove(key);
            }
        }
    }
    out
}

/// The words of every field, trimmed, the empty ones left out.
fn cleaned(fields: BTreeMap<String, String>) -> BTreeMap<String, String> {
    fields
        .into_iter()
        .map(|(key, text)| (key, text.trim().to_owned()))
        .filter(|(_, text)| !text.is_empty())
        .collect()
}

/// The work a board is of, and the board: a work of the profile whose kind
/// lays the board out.
pub fn a_board(
    conn: &Connection,
    profile_id: &str,
    work_id: &str,
    board: &str,
) -> Result<(Work, ProfileConfig)> {
    let work = crate::work::get(conn, work_id)?
        .filter(|work| work.profile_id == profile_id)
        .ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, profile_id)?;
    config.require_board(&work.kind, board)?;
    Ok((work, config))
}

/// Refuse a card of a kind the board does not hold, or with a field its
/// kind does not name; the kind it is of, when it holds.
fn check_card<'a>(
    config: &'a ProfileConfig,
    work: &Work,
    board: &str,
    kind: &str,
    fields: &[&String],
) -> Result<&'a CardKind> {
    let board = config.require_board(&work.kind, board)?;
    let Some(card) = board.card(kind) else {
        return Err(Error::refused("board.notAKind")
            .param("kind", kind)
            .param("board", board.label.as_str()));
    };
    for key in fields {
        if !card.fields.iter().any(|field| field.key == **key) {
            return Err(Error::refused("board.notAField")
                .param("field", key.as_str())
                .param("kind", card.label.as_str()));
        }
    }
    Ok(card)
}

/// The fields as the row stores them: a JSON object in the order the kind
/// names its fields, so the first text in it is the card's headline - what
/// the tombstone and the trash read it by, in SQL, without the profile.
fn stored(kind: &CardKind, fields: &BTreeMap<String, String>) -> Result<String> {
    let mut keys: Vec<&String> = kind
        .fields
        .iter()
        .map(|field| &field.key)
        .filter(|key| fields.contains_key(*key))
        .collect();
    keys.extend(
        fields
            .keys()
            .filter(|key| !kind.fields.iter().any(|field| field.key == **key)),
    );
    let mut out = String::from("{");
    for (index, key) in keys.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&serde_json::to_string(key)?);
        out.push(':');
        out.push_str(&serde_json::to_string(&fields[key])?);
    }
    out.push('}');
    Ok(out)
}

/// The place after the last card of a board.
pub fn next_position(conn: &Connection, work_id: &str, board: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT coalesce(max(position), 0) + 1 FROM board_card WHERE work_id = ?1 AND board = ?2",
        params![work_id, board],
        |row| row.get(0),
    )?)
}

/// The topic a card is put under: the board's own spelling of it when the
/// board already has it in other letters - "Faith" for "faith" - so a topic
/// is one group whoever named it.
pub fn spelled(conn: &Connection, work_id: &str, board: &str, topic: &str) -> Result<String> {
    let topic = topic.trim();
    if topic.is_empty() {
        return Ok(String::new());
    }
    let known: Option<String> = conn
        .query_row(
            "SELECT topic FROM board_card WHERE work_id = ?1 AND board = ?2
             AND lower(topic) = lower(?3) ORDER BY position, rowid LIMIT 1",
            params![work_id, board, topic],
            |row| row.get(0),
        )
        .optional()?;
    // SQLite's lower() knows only ASCII: a topic in another script is
    // compared here as well.
    if let Some(known) = known {
        return Ok(known);
    }
    let topics: Vec<String> = conn
        .prepare("SELECT DISTINCT topic FROM board_card WHERE work_id = ?1 AND board = ?2")?
        .query_map(params![work_id, board], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let wanted = topic.to_lowercase();
    Ok(topics
        .into_iter()
        .find(|known| known.to_lowercase() == wanted)
        .unwrap_or_else(|| topic.to_owned()))
}

/// Put a card on a work's board.
///
/// Refused when the work's kind lays out no such board, when the card is of
/// a kind the board does not hold or names a field its kind does not have,
/// and when it says nothing at all.
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewCard,
    minted: Minted,
) -> Result<BoardCard> {
    let (work, config) = a_board(conn, profile_id, &new.work_id, &new.board)?;
    let fields = cleaned(new.fields);
    let kind = check_card(
        &config,
        &work,
        &new.board,
        &new.kind,
        &fields.keys().collect::<Vec<_>>(),
    )?;
    if fields.is_empty() {
        return Err(Error::refused("board.emptyCard"));
    }
    let position = match new.position {
        Some(position) => position,
        None => next_position(conn, &work.id, &new.board)?,
    };
    conn.execute(
        "INSERT INTO board_card (id, profile_id, work_id, board, kind, topic, position, source,
                                 verdict, fields, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            minted.id(),
            profile_id,
            work.id,
            new.board,
            new.kind,
            new.topic.trim(),
            position,
            new.source.as_str(),
            new.verdict.map(Verdict::as_str),
            stored(kind, &fields)?,
            minted.at(),
        ],
    )?;
    get(conn, minted.id())?.ok_or_else(|| Error::Internal("the card vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<BoardCard>> {
    Ok(conn
        .query_row(&format!("{SELECT} WHERE id = ?1"), params![id], read_row)
        .optional()?)
}

/// One board of a work, in order.
pub fn for_board(conn: &Connection, work_id: &str, board: &str) -> Result<Vec<BoardCard>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE work_id = ?1 AND board = ?2 ORDER BY position, created_at, rowid"
    ))?;
    let rows = statement
        .query_map(params![work_id, board], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Every board of a work, board by board, in order.
pub fn for_work(conn: &Connection, work_id: &str) -> Result<Vec<BoardCard>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE work_id = ?1 ORDER BY board, position, created_at, rowid"
    ))?;
    let rows = statement
        .query_map(params![work_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Change a card: its words, its kind, its place, the person's verdict.
pub fn update_at(conn: &Connection, id: &str, patch: CardPatch, at: &str) -> Result<BoardCard> {
    let current = get(conn, id)?.ok_or_else(|| Error::not_found("card", id))?;
    let work = crate::work::get(conn, &current.work_id)?
        .ok_or_else(|| Error::not_found("work", &current.work_id))?;
    let config = crate::profile::config_for(conn, &current.profile_id)?;
    let kind = patch.kind.unwrap_or_else(|| current.kind.clone());
    let fields = match &patch.fields {
        Some(fields) => merged(&current.fields, fields),
        None => current.fields.clone(),
    };
    // A card keeps what it says when its kind changes, and is held to the
    // fields of the kind it now is - as it is when it is put on the board.
    let written = if kind != current.kind || patch.fields.is_some() {
        let of = check_card(
            &config,
            &work,
            &current.board,
            &kind,
            &fields.keys().collect::<Vec<_>>(),
        )?;
        if fields.is_empty() {
            return Err(Error::refused("board.emptyCard"));
        }
        stored(of, &fields)?
    } else {
        conn.query_row(
            "SELECT fields FROM board_card WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?
    };
    let topic = match &patch.topic {
        Some(topic) => spelled(conn, &current.work_id, &current.board, topic)?,
        None => current.topic.clone(),
    };
    let position = patch.position.unwrap_or(current.position);
    let verdict = patch.verdict.unwrap_or(current.verdict);
    conn.execute(
        "UPDATE board_card SET kind = ?2, topic = ?3, position = ?4, verdict = ?5, fields = ?6,
                               updated_at = ?7
         WHERE id = ?1",
        params![
            id,
            kind,
            topic,
            position,
            verdict.map(Verdict::as_str),
            written,
            at,
        ],
    )?;
    get(conn, id)?.ok_or_else(|| Error::not_found("card", id))
}

/// Stand a board's cards in the order given, each under the topic given:
/// the one write behind dragging a card, renaming a topic and merging two.
///
/// The list names every card of the board, once - a card left out would
/// have no place to stand. Places are counted from one; a card already where
/// the list puts it is not written.
pub fn arrange(
    conn: &Connection,
    work_id: &str,
    board: &str,
    order: &[Placed],
    at: &str,
) -> Result<()> {
    let standing = for_board(conn, work_id, board)?;
    let mut given: Vec<&str> = order.iter().map(|placed| placed.id.as_str()).collect();
    let mut known: Vec<&str> = standing.iter().map(|card| card.id.as_str()).collect();
    given.sort_unstable();
    known.sort_unstable();
    if given != known {
        return Err(Error::refused("board.arrangeNotTheBoard"));
    }
    for (index, placed) in order.iter().enumerate() {
        let position = i64::try_from(index).unwrap_or(i64::MAX) + 1;
        conn.execute(
            "UPDATE board_card SET position = ?2, topic = ?3, updated_at = ?4
             WHERE id = ?1 AND (position IS NOT ?2 OR topic IS NOT ?3)",
            params![placed.id, position, placed.topic.trim(), at],
        )?;
    }
    Ok(())
}

/// The arrangement a board stands in now.
pub fn arrangement(conn: &Connection, work_id: &str, board: &str) -> Result<Vec<Placed>> {
    Ok(for_board(conn, work_id, board)?
        .into_iter()
        .map(|card| Placed {
            id: card.id,
            topic: card.topic,
        })
        .collect())
}

/// The arrangement once `id` stands under `topic`, at `index` among the
/// cards of that topic - the place a drop names. An index past the end is
/// the end.
pub fn moved(standing: &[Placed], id: &str, topic: &str, index: usize) -> Result<Vec<Placed>> {
    let Some(at) = standing.iter().position(|placed| placed.id == id) else {
        return Err(Error::not_found("card", id));
    };
    let mut order = standing.to_vec();
    let mut card = order.remove(at);
    card.topic = topic.trim().to_owned();
    // Where the topic's cards stand in what is left, and the place in the
    // whole list that is `index` among them. A topic with no card yet starts
    // after the last card.
    let places: Vec<usize> = order
        .iter()
        .enumerate()
        .filter(|(_, placed)| placed.topic == card.topic)
        .map(|(place, _)| place)
        .collect();
    let target = match places.get(index) {
        Some(&place) => place,
        None => places.last().map_or(order.len(), |&last| last + 1),
    };
    order.insert(target, card);
    Ok(order)
}

/// The arrangement once every card under `from` stands under `to`: a topic
/// renamed, or - when `to` is another topic of the board, in any letters -
/// merged into it, its cards after the other's.
pub fn retopicked(standing: &[Placed], from: &str, to: &str) -> Vec<Placed> {
    let to = to.trim();
    let into = standing
        .iter()
        .find(|placed| placed.topic != from && placed.topic.to_lowercase() == to.to_lowercase())
        .map(|placed| placed.topic.clone());
    let Some(into) = into else {
        return standing
            .iter()
            .map(|placed| Placed {
                id: placed.id.clone(),
                topic: if placed.topic == from {
                    to.to_owned()
                } else {
                    placed.topic.clone()
                },
            })
            .collect();
    };
    // A merge: the merged cards go after the last card of the topic they
    // join, in their own order.
    let (moving, mut order): (Vec<Placed>, Vec<Placed>) = standing
        .iter()
        .cloned()
        .partition(|placed| placed.topic == from);
    let after = order
        .iter()
        .rposition(|placed| placed.topic == into)
        .map_or(order.len(), |last| last + 1);
    let moving: Vec<Placed> = moving
        .into_iter()
        .map(|placed| Placed {
            id: placed.id,
            topic: into.clone(),
        })
        .collect();
    order.splice(after..after, moving);
    order
}

/// What a board of a work shows: its cards in order, and where the last
/// answer said to dig next.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct BoardView {
    pub board: String,
    pub cards: Vec<BoardCard>,
    /// The directions the last answer put on this board offered - "Asian
    /// literature", "the sea as a border" - for the next run to go. Empty
    /// until an answer offers any.
    pub directions: Vec<String>,
}

/// Every board a work's kind lays out, in its conception's order, with the
/// cards on each. A board the kind no longer lays out but the work still
/// holds cards on comes after them, so nothing written is hidden.
pub fn boards(conn: &Connection, work: &Work) -> Result<Vec<BoardView>> {
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let mut keys: Vec<String> = config
        .boards_of(&work.kind)
        .into_iter()
        .map(|board| board.key.clone())
        .collect();
    let mut cards = for_work(conn, &work.id)?;
    for card in &cards {
        if !keys.contains(&card.board) {
            keys.push(card.board.clone());
        }
    }
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        let (here, rest): (Vec<BoardCard>, Vec<BoardCard>) =
            cards.into_iter().partition(|card| card.board == key);
        cards = rest;
        out.push(BoardView {
            directions: directions(conn, &work.id, &key)?,
            board: key,
            cards: here,
        });
    }
    Ok(out)
}

/// Where the last answer that landed on a board said to dig next: read off
/// the proposal it landed as, which is where an answer's words are kept.
pub fn directions(conn: &Connection, work_id: &str, board: &str) -> Result<Vec<String>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT json_extract(meta, '$.proposal.directions') FROM chat_message
             WHERE json_extract(meta, '$.proposal.kind') = 'cards'
               AND json_extract(meta, '$.proposal.work_id') = ?1
               AND json_extract(meta, '$.proposal.board') = ?2
               AND json_extract(meta, '$.applied') IS NOT NULL
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
            params![work_id, board],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    Ok(raw
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(id: &str, topic: &str) -> Placed {
        Placed {
            id: id.into(),
            topic: topic.into(),
        }
    }

    fn ids(order: &[Placed]) -> Vec<String> {
        order
            .iter()
            .map(|placed| format!("{}:{}", placed.id, placed.topic))
            .collect()
    }

    /// A card dropped among another topic's cards stands at that place among
    /// them, and one dropped past their end after the last of them.
    #[test]
    fn a_dropped_card_stands_where_the_drop_names() {
        let standing = vec![
            placed("a", "x"),
            placed("b", "x"),
            placed("c", "y"),
            placed("d", "y"),
        ];
        assert_eq!(
            ids(&moved(&standing, "a", "y", 1).unwrap()),
            ["b:x", "c:y", "a:y", "d:y"]
        );
        assert_eq!(
            ids(&moved(&standing, "a", "y", 9).unwrap()),
            ["b:x", "c:y", "d:y", "a:y"]
        );
        assert_eq!(
            ids(&moved(&standing, "d", "x", 0).unwrap()),
            ["d:x", "a:x", "b:x", "c:y"]
        );
        // A topic no card has yet starts at the end.
        assert_eq!(
            ids(&moved(&standing, "b", "z", 0).unwrap()),
            ["a:x", "c:y", "d:y", "b:z"]
        );
    }

    /// Renaming a topic gives its cards the new word where they stand;
    /// renaming it to another topic's word, in any letters, merges it -
    /// its cards after that topic's, in that topic's spelling.
    #[test]
    fn a_topic_renamed_onto_another_is_merged_into_it() {
        let standing = vec![
            placed("a", "x"),
            placed("b", "y"),
            placed("c", "x"),
            placed("d", "Z"),
        ];
        assert_eq!(
            ids(&retopicked(&standing, "x", "w")),
            ["a:w", "b:y", "c:w", "d:Z"]
        );
        assert_eq!(
            ids(&retopicked(&standing, "x", "z")),
            ["b:y", "d:Z", "a:Z", "c:Z"]
        );
    }
}
