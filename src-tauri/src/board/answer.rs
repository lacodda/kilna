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
use crate::profile::config::{Board, BoardRule};

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

/// The cards of an answer's block, read against `board`.
///
/// The block is `{"cards": [...], "directions": [...]}` or the list of cards
/// alone. A card names its kind, which may be left out on a board of one
/// kind; its fields come under `fields` or as keys of the card itself. A
/// kind the board does not hold, or a field the kind does not name, is left
/// out and said in `dropped`; a card that says nothing is left out whole.
/// The answer's word for where a card came from is kept only for one worked
/// out from the person's own, and only as many as were asked: everything
/// else is the assistant's own. Refused only when no card could be read.
pub fn read(board: &Board, raw: &Value, request: Option<&CardRequest>) -> Result<Read> {
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
        let nested = object.get("fields").and_then(Value::as_object);
        let mut fields = BTreeMap::new();
        for (key, value) in nested.unwrap_or(object) {
            // Flat on the card, the card's own words are not fields.
            if nested.is_none() && ["kind", "topic", "source"].contains(&key.as_str()) {
                continue;
            }
            let Some(text) = value
                .as_str()
                .map(str::trim)
                .filter(|text| !text.is_empty())
            else {
                continue;
            };
            if kind.fields.iter().any(|field| field.key == *key) {
                fields.insert(key.clone(), text.to_owned());
            } else {
                leave("field", key);
            }
        }
        if fields.is_empty() {
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
        });
    }
    if out.cards.is_empty() {
        return Err(Error::refused("board.noCards"));
    }
    Ok(out)
}

/// One card as a prompt reads it: each field that says anything under the
/// word for it, and the topic it stands under.
pub fn card_line(board: Option<&Board>, card: &BoardCard) -> String {
    let kind = board.and_then(|board| board.card(&card.kind));
    let mut parts = Vec::new();
    match kind {
        Some(kind) => {
            for field in &kind.fields {
                if let Some(text) = card
                    .fields
                    .get(&field.key)
                    .filter(|text| !text.trim().is_empty())
                {
                    parts.push(format!("{}: {}", field.label, flat(text)));
                }
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
/// hint of what goes in it.
fn kinds_sheet(board: &Board) -> String {
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
        for field in &kind.fields {
            out.push_str(&format!("- `{}` - {}", field.key, field.label));
            if let Some(hint) = &field.hint {
                out.push_str(&format!(", for example: “{hint}”"));
            }
            if field.multiline {
                out.push_str(" (a few sentences)");
            }
            out.push('\n');
        }
    }
    out.trim_end().to_owned()
}

/// What the board asks for, and what stands on it: the `{cards}` of an
/// action about a board.
pub fn request_sheet(board: &Board, cards: &[BoardCard], request: &CardRequest) -> String {
    let mut out = format!("The board: {}.", board.label);
    if let Some(description) = &board.description {
        out.push_str(&format!(" {description}"));
    }
    out.push_str(&format!("\n\n{}\n\n", kinds_sheet(board)));

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
            out.push_str(&format!("- {}\n", card_line(Some(board), card)));
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
    let refined = if request.refined().is_some() {
        "\n- The card worked out from the person's own says `\"source\": \"refined\"`; every other one `\"source\": \"ai\"`."
    } else {
        ""
    };
    format!(
        "\n\nEnd your answer with one fenced ```json block holding exactly {total} card{plural}, and two to {most} directions to dig next:\n\n\
```json\n\
{{\"cards\": [\n  {{\n{kind}    \"source\": \"ai\",\n    \"topic\": \"what it is grouped under, a few words, or leave it out\",\n    \"fields\": {{\n{fields}\n    }}\n  }}\n],\n\"directions\": [\"where the next run could dig, a few words\"]}}\n```\n\n\
- Write the fields in the language of the work; leave a field out rather than fill it with nothing.\n\
- A direction is somewhere this board has not been yet, said in a few words a person could click.{refined}",
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
    use crate::profile::config::{CardField, CardKind, Label};

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
                    .map(|key| CardField {
                        key: key.into(),
                        label: Label::from(key),
                        hint: None,
                        multiline: false,
                    })
                    .collect(),
            }],
        }
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
        }
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
        let read = read(&one, &raw, Some(&request)).unwrap();
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
        assert!(read(&one, &serde_json::json!({"cards": []}), None).is_err());
        assert!(read(&one, &serde_json::json!("nothing"), None).is_err());
    }
}
