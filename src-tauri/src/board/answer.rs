//! What a board asks the assistant for, and what it answered (ADR 0065).
//!
//! A board is asked as a cover's ideas are (ADR 0050): how many cards of the
//! assistant's own, the person's own card to work out as one more, where to
//! dig, and whether to go further the way of what is starred. What stands on
//! the board goes with the question - starred as the direction, turned down
//! as what is not wanted, the rest not to repeat - and the answer is one
//! fenced block of cards in the fields of the board's kinds, with a few
//! directions to dig next.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{BoardCard, Source, Verdict};
use crate::error::{Error, Result};
use crate::library::{self, Asked};
use crate::profile::config::{Board, BoardRule, FieldShape, NoteKind};

/// The most cards one run is asked for.
pub const MOST: u8 = 6;

/// How many cards a board asks for when the person has not said.
pub const ON_ASK: u8 = 4;

/// The most directions an answer leaves for the next run.
pub const MOST_DIRECTIONS: usize = 4;

/// What a person asked a board for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct CardRequest {
    /// How many cards of the assistant's own: none to [`MOST`].
    pub count: u8,
    /// The person's own card, in their words, to be worked out as one more.
    pub refine: Option<String>,
    /// Where to dig: "Eastern literature", "not Russian", one of the
    /// directions the last answer offered.
    pub direction: Option<String>,
    /// "More in this direction": the starred cards are what is wanted.
    pub more: bool,
}

impl CardRequest {
    /// How many cards the answer should hold.
    pub fn total(&self) -> usize {
        usize::from(self.count) + usize::from(self.refined().is_some())
    }

    /// The card to work out, when there is one with words in it.
    pub fn refined(&self) -> Option<&str> {
        self.refine
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
    }

    /// Where to dig, when it says anything.
    pub fn heading(&self) -> Option<&str> {
        self.direction
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
    }

    /// Refuse a request that asks for nothing, or for more than one run gives.
    pub fn check(&self) -> Result<()> {
        if self.count > MOST {
            return Err(Error::refused("board.tooMany").param("most", MOST));
        }
        if self.total() == 0 {
            return Err(Error::refused("board.nothingAsked"));
        }
        Ok(())
    }
}

/// A card inside a proposal: what it says, already checked against the
/// board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(rename = "PackagedCard")]
pub struct Packaged {
    pub kind: String,
    #[serde(default)]
    pub topic: String,
    pub source: Source,
    pub fields: BTreeMap<String, String>,
    /// The work of culture a card of a kind that links to the library names:
    /// found in the library when it lands, or written into it (ADR 0066).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub entry: Option<Asked>,
    /// Proposed starred: an agent carrying over research the person already
    /// chose from. A run of the window never stars - the verdict is the
    /// person's.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(as = "Option<bool>", optional)]
    pub starred: bool,
}

/// A part of an answer left out because the board has nothing by that name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(rename = "CardDropped")]
pub struct Dropped {
    /// The card's number in the answer, from 1.
    pub card: usize,
    /// `kind`, `field`, or `card` for one that said nothing and was left out
    /// whole.
    pub part: String,
    pub value: String,
}

/// What an answer's block came to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Read {
    pub cards: Vec<Packaged>,
    pub directions: Vec<String>,
    pub dropped: Vec<Dropped>,
}

/// The work of culture a card names under `entry`, read against the library:
/// none without a title; a medium the library does not name is left out and
/// said.
fn entry_of(
    object: &serde_json::Map<String, Value>,
    library: Option<&NoteKind>,
    mut leave: impl FnMut(&str, &str),
) -> Option<Asked> {
    let entry = object.get("entry").and_then(Value::as_object)?;
    let text = |key: &str| {
        entry
            .get(key)
            .and_then(|value| match value {
                Value::String(text) => Some(text.trim().to_owned()),
                Value::Number(number) => Some(number.to_string()),
                _ => None,
            })
            .filter(|text| !text.is_empty())
    };
    let title = text("title")?;
    let medium = text("medium").and_then(|key| {
        if library.is_some_and(|kind| kind.medium(&key).is_some()) {
            Some(key)
        } else {
            leave("medium", &key);
            None
        }
    });
    Some(Asked {
        title,
        author: text("author"),
        year: text("year"),
        medium,
        original: text("original"),
    })
}

/// The cards of an answer's block, read against `board`, and the library
/// its cards may link to.
///
/// The block is `{"cards": [...], "directions": [...]}` or the list of cards
/// alone. A card names its kind, which may be left out on a board of one
/// kind; its fields come under `fields` or as keys of the card itself. A
/// kind the board does not hold, or a field the kind does not name, is left
/// out and said in `dropped`; a card that says nothing is left out whole.
///
/// A card of a kind that links to the library names its work of culture
/// under `entry` - a title at least, or it is left out whole. A proof that
/// is not a link is left out, so its quote reads as one from memory; the
/// quotes of a work of a medium that is never quoted - a song's lyrics - are
/// left out, whatever the answer gave (ADR 0066).
///
/// The answer's word for where a card came from is kept only for one worked
/// out from the person's own, and only as many as were asked: everything
/// else is the assistant's own. A star is kept only from an agent, never
/// from a run. Refused only when no card could be read.
pub fn read(
    board: &Board,
    library: Option<&NoteKind>,
    raw: &Value,
    request: Option<&CardRequest>,
) -> Result<Read> {
    let (list, directions) = match raw {
        Value::Array(list) => (list, None),
        Value::Object(object) => (
            object
                .get("cards")
                .and_then(Value::as_array)
                .ok_or_else(|| Error::refused("board.noCards"))?,
            object.get("directions"),
        ),
        _ => return Err(Error::refused("board.noCards")),
    };
    let mut out = Read {
        directions: directions
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned)
                    .take(MOST_DIRECTIONS)
                    .collect()
            })
            .unwrap_or_default(),
        ..Read::default()
    };
    // An agent proposing outside a run may work out the person's own cards
    // as it likes; a run asked for one, and gets one.
    let mut refined_left = match request {
        Some(request) => usize::from(request.refined().is_some()),
        None => usize::MAX,
    };

    for (index, raw) in list.iter().enumerate() {
        let number = index + 1;
        let mut leave = |part: &str, value: &str| {
            out.dropped.push(Dropped {
                card: number,
                part: part.to_owned(),
                value: value.to_owned(),
            });
        };
        let Some(object) = raw.as_object() else {
            leave("card", &raw.to_string());
            continue;
        };
        let named = object.get("kind").and_then(Value::as_str).map(str::trim);
        let kind = match named {
            Some(key) => match board.card(key) {
                Some(kind) => kind,
                None => {
                    leave("kind", key);
                    continue;
                }
            },
            None if board.cards.len() == 1 => &board.cards[0],
            None => {
                leave("kind", "");
                continue;
            }
        };
        let entry = if kind.library {
            match entry_of(object, library, &mut leave) {
                Some(entry) => Some(entry),
                None => {
                    leave("entry", &raw.to_string());
                    continue;
                }
            }
        } else {
            None
        };
        let quoted = entry.as_ref().is_none_or(|entry| {
            entry
                .medium
                .as_deref()
                .and_then(|key| library.and_then(|kind| kind.medium(key)))
                .is_none_or(|medium| medium.quoted)
        });
        let nested = object.get("fields").and_then(Value::as_object);
        let mut fields = BTreeMap::new();
        for (key, value) in nested.unwrap_or(object) {
            // Flat on the card, the card's own words are not fields.
            if nested.is_none()
                && ["kind", "topic", "source", "entry", "starred"].contains(&key.as_str())
            {
                continue;
            }
            let Some(text) = value
                .as_str()
                .map(str::trim)
                .filter(|text| !text.is_empty())
            else {
                continue;
            };
            if !kind.fields.iter().any(|field| field.key == *key) {
                leave("field", key);
            } else if kind.is(key, FieldShape::Proof) && !library::is_link(text) {
                leave("proof", text);
            } else if !quoted
                && (kind.is(key, FieldShape::Quote) || kind.is(key, FieldShape::Proof))
            {
                leave("quote", key);
            } else {
                fields.insert(key.clone(), text.to_owned());
            }
        }
        if fields.is_empty() && entry.is_none() {
            leave("card", &raw.to_string());
            continue;
        }
        let said = object.get("source").and_then(Value::as_str).map(str::trim);
        let source = if said == Some("refined") && refined_left > 0 {
            refined_left -= 1;
            Source::Refined
        } else {
            Source::Ai
        };
        out.cards.push(Packaged {
            kind: kind.key.clone(),
            topic: object
                .get("topic")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default()
                .to_owned(),
            source,
            fields,
            entry,
            starred: request.is_none()
                && object.get("starred").and_then(Value::as_bool) == Some(true),
        });
    }
    if out.cards.is_empty() {
        return Err(Error::refused("board.noCards"));
    }
    Ok(out)
}

/// One card as a prompt and the order read it: the entry of the library it
/// links to, each field that says anything under the word for it, and the
/// topic it stands under.
///
/// Somebody else's words reach a writer only as checked (ADR 0066): a card's
/// quotes are given word for word with where they were checked, or - from
/// memory - left out with a word that says so; and a work of a medium that
/// is never quoted gives no quote at all.
pub fn card_line(board: Option<&Board>, library: Option<&NoteKind>, card: &BoardCard) -> String {
    let kind = board.and_then(|board| board.card(&card.kind));
    let mut parts = Vec::new();
    if let Some(entry) = &card.entry {
        parts.push(entry.described(library));
    }
    match kind {
        Some(kind) => {
            let quoted = card
                .entry
                .as_ref()
                .is_none_or(|entry| entry.quoted(library));
            let checked = kind
                .proof()
                .and_then(|proof| card.fields.get(&proof.key))
                .is_some_and(|text| library::is_link(text));
            let mut from_memory = false;
            for field in &kind.fields {
                let Some(text) = card
                    .fields
                    .get(&field.key)
                    .filter(|text| !text.trim().is_empty())
                else {
                    continue;
                };
                match field.shape {
                    FieldShape::Quote if !quoted => {}
                    FieldShape::Quote if !checked => from_memory = true,
                    FieldShape::Quote => parts.push(format!("{}: “{}”", field.label, flat(text))),
                    FieldShape::Proof if !quoted || !checked => {}
                    FieldShape::Chips => parts.push(format!(
                        "{}: {}",
                        field.label,
                        library::chips(text).join("; ")
                    )),
                    FieldShape::Proof | FieldShape::Text => {
                        parts.push(format!("{}: {}", field.label, flat(text)));
                    }
                }
            }
            if from_memory {
                parts.push(
                    "its quote is from memory, not checked, and left out: take its sense, never its words"
                        .to_owned(),
                );
            }
        }
        None => {
            for (key, text) in &card.fields {
                parts.push(format!("{key}: {}", flat(text)));
            }
        }
    }
    let mut line = parts.join(" | ");
    if let (Some(kind), Some(board)) = (kind, board)
        && board.cards.len() > 1
    {
        line = format!("({}) {line}", kind.label);
    }
    if !card.topic.is_empty() {
        line.push_str(&format!(" [topic: {}]", card.topic));
    }
    line
}

/// A text on one line, the way a list item holds it.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What a card of each kind on the board says, field by field, with the
/// hint of what goes in it - and, for a kind that links to the library, the
/// work of culture it names.
fn kinds_sheet(board: &Board, library: Option<&NoteKind>) -> String {
    let mut out = String::new();
    for kind in &board.cards {
        if board.cards.len() > 1 {
            out.push_str(&format!(
                "A card of the kind `{}` ({}) says:\n",
                kind.key, kind.label
            ));
        } else {
            out.push_str("A card says:\n");
        }
        if kind.library {
            out.push_str(&entry_sheet(library));
        }
        for field in &kind.fields {
            out.push_str(&format!("- `{}` - {}", field.key, field.label));
            if let Some(hint) = &field.hint {
                out.push_str(&format!(", for example: “{hint}”"));
            }
            match field.shape {
                FieldShape::Quote => out.push_str(
                    " (somebody else's words, word for word, in the language they were written in)",
                ),
                FieldShape::Proof => out.push_str(
                    " (the link of the page where you read the quote word for word; leave it out when you could not check it)",
                ),
                FieldShape::Chips => out.push_str(
                    " (short things worth keeping - images, names, places, terms - parted by semicolons)",
                ),
                FieldShape::Text if field.multiline => out.push_str(" (a few sentences)"),
                FieldShape::Text => {}
            }
            out.push('\n');
        }
    }
    out.trim_end().to_owned()
}

/// How a card of a kind that links to the library names its work of
/// culture, and what is never quoted.
fn entry_sheet(library: Option<&NoteKind>) -> String {
    let media = library
        .map(|kind| {
            kind.media
                .iter()
                .map(|medium| format!("`{}` ({})", medium.key, medium.label))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let mut out = String::from(
        "- `entry` - the work of culture it is: `title` as it is known in the language of the work being written, `author`, `year`",
    );
    if !media.is_empty() {
        out.push_str(&format!(", `medium` - one of: {media}"));
    }
    out.push_str(
        ", and `original` - its title in the language it was made in, when that differs\n",
    );
    let unquoted: Vec<String> = library
        .map(|kind| {
            kind.media
                .iter()
                .filter(|medium| !medium.quoted)
                .map(|medium| medium.label.to_string())
                .collect()
        })
        .unwrap_or_default();
    if !unquoted.is_empty() {
        out.push_str(&format!(
            "  A work of {} is never quoted: name it, say why it fits, give no words of it.\n",
            unquoted.join(", ").to_lowercase()
        ));
    }
    out
}

/// What the board asks for, and what stands on it: the `{cards}` of an
/// action about a board. `others` is what the library already gave other
/// works, for a board that links to it.
pub fn request_sheet(
    board: &Board,
    library: Option<&NoteKind>,
    cards: &[BoardCard],
    request: &CardRequest,
    others: &str,
) -> String {
    let mut out = format!("The board: {}.", board.label);
    if let Some(description) = &board.description {
        out.push_str(&format!(" {description}"));
    }
    out.push_str(&format!("\n\n{}\n\n", kinds_sheet(board, library)));

    let count = usize::from(request.count);
    let plural = if count == 1 { "" } else { "s" };
    match (count, request.refined()) {
        (0, Some(own)) => out.push_str(&format!(
            "Asked for: the person's own card, worked out - nothing else.\n\nTheir card, in their words: “{own}”\n"
        )),
        (_, Some(own)) => out.push_str(&format!(
            "Asked for: {count} new card{plural} of your own, and the person's own card worked out into one more.\n\nTheir card, in their words: “{own}”\n"
        )),
        (_, None) => out.push_str(&format!("Asked for: {count} new card{plural} of your own.\n")),
    }
    if let Some(heading) = request.heading() {
        out.push_str(&format!("\nWhere to dig this time: {heading}\n"));
    }

    let starred: Vec<&BoardCard> = cards
        .iter()
        .filter(|card| card.verdict == Some(Verdict::Star))
        .collect();
    let rejected: Vec<&BoardCard> = cards
        .iter()
        .filter(|card| card.verdict == Some(Verdict::Rejected))
        .collect();
    let open: Vec<&BoardCard> = cards.iter().filter(|card| card.verdict.is_none()).collect();
    if request.more && !starred.is_empty() {
        out.push_str(
            "\nMore in this direction: the starred cards below are what the person likes. Go further their way, from sides they have not taken yet.\n",
        );
    }
    let mut list = |heading: &str, cards: &[&BoardCard]| {
        if cards.is_empty() {
            return;
        }
        out.push_str(&format!("\n{heading}\n"));
        for card in cards {
            out.push_str(&format!("- {}\n", card_line(Some(board), library, card)));
        }
    };
    list(
        if request.more {
            "Starred - examples of what is wanted:"
        } else {
            "Starred already - do not repeat these:"
        },
        &starred,
    );
    list(
        "Turned down - examples of what is NOT wanted; stay away from them:",
        &rejected,
    );
    list("On the board, not judged yet - do not repeat these:", &open);

    let mut topics: Vec<&str> = Vec::new();
    for card in cards {
        if !card.topic.is_empty() && !topics.contains(&card.topic.as_str()) {
            topics.push(&card.topic);
        }
    }
    if !topics.is_empty() {
        out.push_str(&format!(
            "\nThe board groups its cards under these topics: {}. Put a card under one of them when it belongs there, or under a new topic of a few words.\n",
            topics.join(", ")
        ));
    }
    if board.links_the_library() && !others.trim().is_empty() {
        out.push_str(&format!(
            "\nThe library already gave these to other works. Bring one back only when nothing new fits as well, and say in its card why it is worth repeating:\n{}\n",
            others.trim_end()
        ));
    }
    out.trim_end().to_owned()
}

/// What an action about a board appends to its prompt: the block to answer
/// with, spelled out, because "reply with JSON" makes a different shape every
/// time.
pub fn instruction(board: &Board, request: &CardRequest) -> String {
    let total = request.total();
    let kind = if board.cards.len() == 1 {
        String::new()
    } else {
        format!(
            "    \"kind\": \"one of: {}\",\n",
            board
                .cards
                .iter()
                .map(|kind| kind.key.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let first = &board.cards[0];
    let fields = first
        .fields
        .iter()
        .map(|field| format!("      \"{}\": \"{}\"", field.key, field.label))
        .collect::<Vec<_>>()
        .join(",\n");
    let entry = if first.library {
        "    \"entry\": {\"title\": \"…\", \"author\": \"…\", \"year\": \"…\", \"medium\": \"…\", \"original\": \"…\"},\n"
    } else {
        ""
    };
    let refined = if request.refined().is_some() {
        "\n- The card worked out from the person's own says `\"source\": \"refined\"`; every other one `\"source\": \"ai\"`."
    } else {
        ""
    };
    let library = if board.links_the_library() {
        "\n- A card that names a work of culture gives it under `entry`; leave out what you do not know rather than guess it.\n- Give a quote only as it is written in the work, and its proof only for a page where you read those words yourself; a quote you could not check goes without a proof."
    } else {
        ""
    };
    format!(
        "\n\nEnd your answer with one fenced ```json block holding exactly {total} card{plural}, and two to {most} directions to dig next:\n\n\
```json\n\
{{\"cards\": [\n  {{\n{kind}    \"source\": \"ai\",\n    \"topic\": \"what it is grouped under, a few words, or leave it out\",\n{entry}    \"fields\": {{\n{fields}\n    }}\n  }}\n],\n\"directions\": [\"where the next run could dig, a few words\"]}}\n```\n\n\
- Write the fields in the language of the work; leave a field out rather than fill it with nothing.\n\
- A direction is somewhere this board has not been yet, said in a few words a person could click.{refined}{library}",
        plural = if total == 1 { "" } else { "s" },
        most = MOST_DIRECTIONS,
    )
}

/// How the order reads a board, by its rule and what is starred on it: the
/// cards a writer works from, and the sentence that says how.
///
/// A board of rivals (`one`) gives the starred ones when any are - the one to
/// write by, or those to choose from or combine - and every card not turned
/// down when none is, the choice left to the writer. A board that adds up
/// (`all`) gives every card not turned down, the starred first. A turned-down
/// card never goes.
pub fn for_writer<'a>(board: &Board, cards: &'a [BoardCard]) -> (String, Vec<&'a BoardCard>) {
    let kept: Vec<&BoardCard> = cards
        .iter()
        .filter(|card| card.verdict != Some(Verdict::Rejected))
        .collect();
    let starred: Vec<&BoardCard> = kept
        .iter()
        .copied()
        .filter(|card| card.verdict == Some(Verdict::Star))
        .collect();
    match board.rule {
        BoardRule::One => match starred.len() {
            0 => (
                "Nothing is starred: choose the strongest of these, or combine them - the choice is yours.".to_owned(),
                kept,
            ),
            1 => ("Write by this one.".to_owned(), starred),
            _ => ("Choose one of these, or combine them.".to_owned(), starred),
        },
        BoardRule::All => {
            let mut ordered = starred.clone();
            ordered.extend(kept.iter().copied().filter(|card| card.verdict != Some(Verdict::Star)));
            let rule = if starred.is_empty() {
                "Draw on all of these.".to_owned()
            } else {
                "Draw on all of these; the starred ones (★) matter most.".to_owned()
            };
            (rule, ordered)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Entry;
    use crate::profile::config::{CardField, CardKind, Label, Medium};

    fn field(key: &str, shape: FieldShape) -> CardField {
        CardField {
            key: key.into(),
            label: Label::from(key),
            hint: None,
            multiline: false,
            shape,
        }
    }

    fn board(rule: BoardRule) -> Board {
        Board {
            key: "scenarios".into(),
            label: Label::from("Scenarios"),
            description: None,
            rule,
            cards: vec![CardKind {
                key: "scenario".into(),
                label: Label::from("Scenario"),
                fields: ["line", "image", "ending"]
                    .into_iter()
                    .map(|key| field(key, FieldShape::Text))
                    .collect(),
                library: false,
            }],
        }
    }

    /// A board of research: works of culture linked to the library, with a
    /// quote, its proof and chips, and concepts that link to nothing.
    fn research() -> Board {
        Board {
            key: "research".into(),
            label: Label::from("Research"),
            description: None,
            rule: BoardRule::All,
            cards: vec![
                CardKind {
                    key: "work".into(),
                    label: Label::from("Work"),
                    fields: vec![
                        field("why", FieldShape::Text),
                        field("quote", FieldShape::Quote),
                        field("proof", FieldShape::Proof),
                        field("chips", FieldShape::Chips),
                    ],
                    library: true,
                },
                CardKind {
                    key: "concept".into(),
                    label: Label::from("Concept"),
                    fields: vec![field("term", FieldShape::Text)],
                    library: false,
                },
            ],
        }
    }

    fn library() -> NoteKind {
        let mut kind = NoteKind::new("source", "Library");
        kind.library = true;
        kind.media = vec![
            Medium {
                key: "film".into(),
                label: Label::from("Film"),
                quoted: true,
            },
            Medium {
                key: "music".into(),
                label: Label::from("Music"),
                quoted: false,
            },
        ];
        kind
    }

    fn card(id: &str, verdict: Option<Verdict>) -> BoardCard {
        BoardCard {
            id: id.into(),
            profile_id: "p".into(),
            work_id: "w".into(),
            board: "scenarios".into(),
            kind: "scenario".into(),
            topic: String::new(),
            position: 0,
            source: Source::Ai,
            verdict,
            fields: BTreeMap::from([("line".to_owned(), id.to_owned())]),
            created_at: String::new(),
            updated_at: String::new(),
            note_id: None,
            entry: None,
        }
    }

    /// A card of the research board linked to an entry of `medium`.
    fn linked(medium: &str, fields: &[(&str, &str)]) -> BoardCard {
        BoardCard {
            board: "research".into(),
            kind: "work".into(),
            fields: fields
                .iter()
                .map(|(key, text)| ((*key).to_owned(), (*text).to_owned()))
                .collect(),
            note_id: Some("e".into()),
            entry: Some(Entry {
                id: "e".into(),
                title: "Old Joy".into(),
                author: Some("Kelly Reichardt".into()),
                year: Some("2006".into()),
                medium: Some(medium.into()),
                aliases: Vec::new(),
            }),
            ..card("c", None)
        }
    }

    /// A card of the library names its work of culture under `entry`, or is
    /// left out; a medium the library does not name is dropped from it; a
    /// proof that is not a link is dropped, and the quotes of a song never
    /// land; a star lands only from an agent.
    #[test]
    fn a_card_of_the_library_is_read_with_its_entry_and_its_quotes_held_to_the_rule() {
        let research = research();
        let library = library();
        let raw = serde_json::json!({ "cards": [
            {"kind": "work", "starred": true,
             "entry": {"title": "Old Joy", "author": "Kelly Reichardt", "year": 2006, "medium": "film"},
             "fields": {"why": "two friends drift", "quote": "Sorrow is nothing but worn-out joy", "proof": "checked"}},
            {"kind": "work", "entry": {"title": "Blowin' in the Wind", "medium": "music"},
             "fields": {"why": "the question", "quote": "How many roads", "proof": "https://example.org/song"}},
            {"kind": "work", "entry": {"title": "A ballad", "medium": "opera"}, "fields": {"why": "x"}},
            {"kind": "work", "fields": {"why": "no entry"}},
            {"kind": "concept", "fields": {"term": "pharisee"}}
        ]});
        let read = read(&research, Some(&library), &raw, None).unwrap();
        assert_eq!(read.cards.len(), 4, "{:?}", read.dropped);
        let film = &read.cards[0];
        assert_eq!(film.entry.as_ref().unwrap().year.as_deref(), Some("2006"));
        assert!(film.starred, "an agent may carry a star over");
        assert!(film.fields.contains_key("quote"));
        assert!(!film.fields.contains_key("proof"), "a proof is a link");
        let song = &read.cards[1];
        assert!(!song.fields.contains_key("quote"), "a song is never quoted");
        assert!(!song.fields.contains_key("proof"));
        assert_eq!(read.cards[2].entry.as_ref().unwrap().medium, None);
        assert_eq!(
            read.dropped
                .iter()
                .map(|d| (d.card, d.part.as_str()))
                .collect::<Vec<_>>(),
            [
                (1, "proof"),
                (2, "quote"),
                (2, "quote"),
                (3, "medium"),
                (4, "entry")
            ]
        );
        let ran = super::read(
            &research,
            Some(&library),
            &raw,
            Some(&CardRequest::default()),
        )
        .unwrap();
        assert!(!ran.cards[0].starred, "a run never stars");
    }

    /// A quote reaches a writer word for word only with where it was checked;
    /// from memory it is left out and said; a song gives none at all.
    #[test]
    fn a_quote_reaches_the_writer_only_as_checked() {
        let research = research();
        let library = library();
        let words = "Sorrow is nothing but worn-out joy";
        let checked = card_line(
            Some(&research),
            Some(&library),
            &linked(
                "film",
                &[("quote", words), ("proof", "https://example.org/old-joy")],
            ),
        );
        assert!(
            checked.starts_with("(Work) Kelly Reichardt, “Old Joy”, 2006 · Film"),
            "{checked}"
        );
        assert!(checked.contains(words), "{checked}");
        assert!(checked.contains("https://example.org/old-joy"), "{checked}");

        let remembered = card_line(
            Some(&research),
            Some(&library),
            &linked(
                "film",
                &[("quote", words), ("chips", "a lake; a log\nthe hot spring")],
            ),
        );
        assert!(!remembered.contains(words), "{remembered}");
        assert!(remembered.contains("from memory"), "{remembered}");
        assert!(
            remembered.contains("a lake; a log; the hot spring"),
            "{remembered}"
        );

        let sung = card_line(
            Some(&research),
            Some(&library),
            &linked(
                "music",
                &[("quote", words), ("proof", "https://example.org/x")],
            ),
        );
        assert!(
            !sung.contains(words) && !sung.contains("from memory"),
            "{sung}"
        );
        assert!(!sung.contains("example.org"), "{sung}");
    }

    fn ids(cards: &[&BoardCard]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    /// The star rule of a board of rivals: one starred is the one, several
    /// are what to choose from, none leaves every card not turned down - and
    /// a turned-down card never reaches the writer.
    #[test]
    fn a_board_of_rivals_gives_the_writer_what_the_stars_say() {
        let one = board(BoardRule::One);
        let none = [
            card("a", None),
            card("b", Some(Verdict::Rejected)),
            card("c", None),
        ];
        let (rule, given) = for_writer(&one, &none);
        assert_eq!(ids(&given), ["a", "c"]);
        assert!(rule.contains("Nothing is starred"), "{rule}");

        let single = [
            card("a", None),
            card("b", Some(Verdict::Star)),
            card("c", None),
        ];
        let (rule, given) = for_writer(&one, &single);
        assert_eq!(ids(&given), ["b"]);
        assert_eq!(rule, "Write by this one.");

        let several = [
            card("a", Some(Verdict::Star)),
            card("b", Some(Verdict::Rejected)),
            card("c", Some(Verdict::Star)),
        ];
        let (rule, given) = for_writer(&one, &several);
        assert_eq!(ids(&given), ["a", "c"]);
        assert!(rule.contains("combine"), "{rule}");
    }

    /// A board that adds up gives every card not turned down, starred first.
    #[test]
    fn a_board_that_adds_up_gives_everything_but_the_turned_down_starred_first() {
        let all = board(BoardRule::All);
        let cards = [
            card("a", None),
            card("b", Some(Verdict::Rejected)),
            card("c", Some(Verdict::Star)),
        ];
        let (_, given) = for_writer(&all, &cards);
        assert_eq!(ids(&given), ["c", "a"]);
    }

    /// Fields come under `fields` or flat on the card; a field the kind does
    /// not name, a kind the board does not hold and a card with nothing in
    /// it are left out and said; one "refined" is kept when one was asked.
    #[test]
    fn an_answer_is_read_against_the_board() {
        let one = board(BoardRule::One);
        let raw = serde_json::json!({
            "cards": [
                {"source": "refined", "topic": " Sea ", "fields": {"line": "A keeper counts ships", "mood": "grey"}},
                {"source": "refined", "line": "A boat repainted", "ending": "it sinks"},
                {"kind": "poem", "fields": {"line": "x"}},
                {"fields": {"line": "  "}}
            ],
            "directions": ["Eastern literature", " ", "the sea as a border", "a", "b", "c"]
        });
        let request = CardRequest {
            count: 3,
            refine: Some("my own".into()),
            ..CardRequest::default()
        };
        let read = read(&one, None, &raw, Some(&request)).unwrap();
        assert_eq!(read.cards.len(), 2);
        assert_eq!(read.cards[0].source, Source::Refined);
        assert_eq!(read.cards[0].topic, "Sea");
        assert_eq!(
            read.cards[1].source,
            Source::Ai,
            "only one was asked to be worked out"
        );
        assert_eq!(
            read.cards[1].fields.get("ending").map(String::as_str),
            Some("it sinks")
        );
        assert_eq!(
            read.dropped
                .iter()
                .map(|d| (d.card, d.part.as_str()))
                .collect::<Vec<_>>(),
            [(1, "field"), (3, "kind"), (4, "card")]
        );
        assert_eq!(
            read.directions,
            ["Eastern literature", "the sea as a border", "a", "b"]
        );
    }

    /// An answer with nothing to put on the board is refused, not landed
    /// as an empty proposal.
    #[test]
    fn an_answer_with_no_card_is_refused() {
        let one = board(BoardRule::One);
        assert!(read(&one, None, &serde_json::json!({"cards": []}), None).is_err());
        assert!(read(&one, None, &serde_json::json!("nothing"), None).is_err());
    }
}
