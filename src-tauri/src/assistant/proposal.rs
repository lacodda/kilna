//! Reading a structured result out of an answer.
//!
//! An action can ask for something the application knows how to act on — a
//! score along the profile's axes, say — rather than for prose. The assistant
//! still answers in text; what changes is that part of that text is a fenced
//! JSON block this module knows how to find.
//!
//! **The assistant never writes to the workspace.** It proposes, kilna shows
//! what was proposed, and a person applies it. That rule is the reason this is
//! a parser and not a writer: everything here turns text into something the
//! frontend can display next to an "apply" button, and nothing here touches the
//! database. Decided in v0.28 for versions, and it holds unchanged for scores.
//! What a person applies becomes rows in [`super::apply`] — the one place a
//! proposal is written, and the place that marks it applied.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::profile::config::{Label, ProfileConfig, WorkKind};

/// What an answer proposed, if anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Proposal {
    /// Values along the profile's scoring axes.
    Score {
        /// Axis key to value, already checked against the profile.
        axes: Map<String, Value>,
        /// The assistant's reasoning, when it gave any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
        /// Axes the answer named that the profile does not have, and axes of
        /// the profile the answer skipped. Shown rather than hidden: a
        /// proposal that only half fits is worth applying, but not silently.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        unknown: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        missing: Vec<String>,
    },
    /// A new version in a role. The text itself is the message body — that
    /// is what *insert as version* keeps, verbatim — so the proposal carries
    /// only where it goes. Made by an agent outside the window (`kilna --mcp`).
    Version {
        role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    /// A note, on the chat's work or on nothing in particular; the body is
    /// the message body. Made by an agent outside the window.
    Note {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// A whole work, or a package of changes to one: the fields of the
    /// overview, versions by role, a score, notes — applied together with one
    /// click. Made by an agent outside the window.
    ///
    /// Which of the two it is comes from the chat the message is in: a chat
    /// on a work receives packages for that work, a chat on nothing receives
    /// new works, which then need `title` and `kind`. The texts travel here
    /// rather than in the message body, because there are several; the body
    /// is a rendering of the package a person reads before applying it.
    Work {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// The kind of the new work. Not `kind`: that name is the tag that
        /// says which variant this is.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        work_kind: Option<String>,
        /// Overview field key to value, already checked against the profile.
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        fields: Map<String, Value>,
        /// Field keys the package named that the profile does not have.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        unknown_fields: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        versions: Vec<PackagedVersion>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        score: Option<Marks>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        notes: Vec<PackagedNote>,
        /// The storyboard of a new video, or scenes added to an existing
        /// one's board. Only for a kind that has a storyboard.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        scenes: Vec<PackagedScene>,
        /// Releases to plan, with what each goes out as. An agent that has
        /// just written a video's board is the one that knows what its
        /// description should say, and making it propose the release
        /// separately would mean a second round trip for one half of one
        /// thought.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        releases: Vec<PackagedRelease>,
    },
    /// A storyboard for the chat's work: scenes added after the last, the
    /// whole board replaced, or numbered scenes revised in place. Made by
    /// an agent outside the window, or by an action of the Scenes tab.
    ///
    /// Its own variant rather than a package with only scenes, because the
    /// decision is different: *add to the board* keeps what is there,
    /// *replace the board* takes the rest to the trash, *revise* touches
    /// only the numbers it names — and the button has to say which. The
    /// body is a rendering of the board a person reads first.
    Scenes {
        scenes: Vec<PackagedScene>,
        /// What the proposal does to the board that is there.
        #[serde(default)]
        change: BoardChange,
    },
    /// A comment read off a screenshot, waiting to be kept. The channel and
    /// the work come from where the screenshot was pasted, not from the
    /// answer: the picture cannot say which channel it is, and a model asked
    /// to repeat a word back is a model that can repeat it wrong.
    Comment {
        channel: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        work_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<String>,
        body: String,
        /// The day it was written, when the picture showed one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        commented_on: Option<String>,
        /// What the picture says it was written under — a video's title —
        /// when no work was given: a hint for choosing one, never a key.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        about: Option<String>,
    },
    /// A reply to one comment; the text is the message body, the way a
    /// version's is.
    Reply { comment_id: String },
}

/// What a scenes proposal does to the board already on the work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoardChange {
    /// The scenes go after the last; the board is otherwise untouched.
    #[default]
    Add,
    /// The scenes are the whole board: a scene with the same number is
    /// rewritten in place, the rest of the old board goes to the trash, and
    /// numbers without a scene yet are created.
    Replace,
    /// Only the scenes named by number change, and only in what the
    /// proposal says about them: a field left out is kept, the blocks are
    /// set together when given. The rest of the board is untouched.
    Revise,
}

impl BoardChange {
    /// The value as it is written in `produces` and in the tool's argument.
    pub fn as_str(self) -> &'static str {
        match self {
            BoardChange::Add => "add",
            BoardChange::Replace => "replace",
            BoardChange::Revise => "revise",
        }
    }

    /// `add`, `replace` or `revise`; anything else is none.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "add" => Some(BoardChange::Add),
            "replace" => Some(BoardChange::Replace),
            "revise" => Some(BoardChange::Revise),
            _ => None,
        }
    }
}

/// A scene inside a proposal: the fields of a row, already checked against
/// the kind's words — the shot type and the block keys are the kind's, and
/// the span is a span.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackagedScene {
    /// The number on the board; after the last when omitted on an added
    /// scene, and in order from 1 on a replaced board.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shot_type: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Prompt blocks by the kind's `scene_blocks` key.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub blocks: Map<String, Value>,
}

/// A version inside a package: the text travels with it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackagedVersion {
    pub role: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A release inside a package: the kind it ships as, when, and what it says
/// about itself.
///
/// The fields are checked against the release kind's own, so a package cannot
/// leave a value under a key no box will ever show. A date is optional: a
/// release with none is queued, which is what "plan this, I will find it a
/// day" means.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackagedRelease {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
    /// Field key to value, already checked against the release kind.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub fields: Map<String, Value>,
    /// Field keys the package named that this release kind does not have.
    /// Shown rather than hidden, the way a work's unknown fields are: a
    /// proposal that only half fits is worth applying, but not silently.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown_fields: Vec<String>,
}

/// A note inside a package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackagedNote {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub body: String,
}

/// Marks along the axes, checked against the kind — the inside of a score
/// proposal, reused by a package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marks {
    pub axes: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<String>,
}

/// The shape an answer is asked to produce.
#[derive(Debug, Clone, Deserialize)]
struct RawScore {
    #[serde(default)]
    axes: Map<String, Value>,
    #[serde(default)]
    note: Option<String>,
}

/// What a scoring action appends to its prompt.
///
/// Spelled out rather than left to the model's judgement: the block has to be
/// findable, and "reply with JSON" produces a different shape every time. The
/// axes are named with their scales because a number without its ceiling is
/// meaningless — 7 out of 10 and 7 out of 100 are different opinions. And
/// they are named with everything else the profile says about them — the
/// label, the question the axis asks, the marks of its rubric, its weight —
/// because that is the judgement the author wrote down, and a model handed
/// only `"hook": <0-10>` judges by its own. The tiers follow, so the answer
/// knows what its total means.
pub fn scoring_instruction(config: &ProfileConfig, kind: &str) -> String {
    let vocabulary = config.vocabulary(kind);

    let mut guide = String::new();
    for axis in &vocabulary.axes {
        guide.push_str(&format!(
            "- `{}` — {} (0 to {}, weight {})",
            axis.key, axis.label, axis.scale, axis.weight
        ));
        if let Some(description) = axis
            .description
            .as_ref()
            .map(Label::as_str)
            .filter(|d| !d.trim().is_empty())
        {
            guide.push_str(&format!(": {}", description.trim()));
        }
        guide.push('\n');
        if !axis.options.is_empty() {
            let options = axis
                .options
                .iter()
                .map(|option| format!("{} = {}", option.value, option.label))
                .collect::<Vec<_>>()
                .join("; ");
            guide.push_str(&format!("  answers: {options}\n"));
        }
        for mark in &axis.rubric {
            guide.push_str(&format!("  {} — {}\n", mark.at, mark.label));
        }
    }

    let tiers = if vocabulary.tiers.is_empty() {
        String::new()
    } else {
        let mut sorted: Vec<_> = vocabulary.tiers.iter().collect();
        sorted.sort_by(|a, b| {
            b.min
                .partial_cmp(&a.min)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        format!(
            "\nThe total is the weighted mean of the axes on a 0–100 scale; the tiers are: {}.\n",
            sorted
                .iter()
                .map(|tier| format!("{} from {}", tier.label, tier.min))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let shape = vocabulary
        .axes
        .iter()
        .map(|axis| format!("  \"{}\": <0-{}>", axis.key, axis.scale))
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        "\n\nJudge it along these axes, exactly as the profile defines them:\n\n{guide}{tiers}\n\
         End your reply with a fenced json block, exactly this shape and \
         nothing else inside it:\n\n```json\n{{\n  \"axes\": {{\n{shape}\n  }},\n  \
         \"note\": \"one sentence on why\"\n}}\n```\n\nSay whatever you like \
         above the block. Use every axis listed and no others."
    )
}

/// What an action that produces a version appends to its prompt.
///
/// The whole answer is what gets kept, verbatim, so the model is told not to
/// wrap it: a preamble or a closing question would become part of the text.
pub fn version_instruction(role_label: &str) -> String {
    format!(
        "\n\nYour whole reply is kept as the {role_label}, word for word: write only it — no \
         preamble, no closing question, no fences around the whole."
    )
}

/// What an action that reads a comment off a screenshot appends to its prompt.
///
/// `today` is given because a screenshot says "3 weeks ago", and turning that
/// into a day needs to know which day it is now.
pub fn comment_instruction(today: &str) -> String {
    format!(
        "\n\nToday is {today}. End your reply with a fenced json block, exactly this shape and \
         nothing else inside it:\n\n```json\n{{\n  \"author\": \"<the name shown, or null>\",\n  \
         \"text\": \"<the comment, word for word, in its own language>\",\n  \
         \"day\": \"<YYYY-MM-DD it was written, or null>\",\n  \
         \"about\": \"<the title of what it was written under, if the picture shows it, or null>\"\n}}\n```\n\n\
         Copy the text exactly: do not translate it, correct it or shorten it. A relative time \
         (\"3 weeks ago\") becomes the day it means; leave `day` null when the picture shows none. \
         If the picture holds several comments, take the one that is not a reply."
    )
}

/// What an action that drafts a reply appends to its prompt.
///
/// The whole answer is what gets kept, as with a version: a preamble would be
/// posted under someone's comment.
pub fn reply_instruction() -> &'static str {
    "\n\nYour whole reply is kept as the answer to the comment, word for word: write only it — \
     no preamble, no options to choose from, no quotation marks around it."
}

/// Find a comment in an answer to a screenshot.
///
/// `channel` and `work_id` are where the screenshot was pasted. An answer
/// with no block, or with a block that holds no text, is refused with the
/// reason rather than proposing an empty comment: a comment is its words.
pub fn read_comment(
    body: &str,
    channel: &str,
    work_id: Option<String>,
) -> std::result::Result<Proposal, String> {
    let block = fenced_json(body).ok_or("the answer holds no comment block")?;
    let raw: Map<String, Value> = serde_json::from_str(&block)
        .map_err(|err| format!("the comment block is not JSON: {err}"))?;
    let text = |key: &str| {
        raw.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && *value != "null")
            .map(str::to_owned)
    };
    let body = text("text").ok_or("the comment block holds no text")?;
    // A day that is not one is dropped rather than refusing the comment: the
    // words are what matter, and the day is a field the person can fill in.
    let commented_on = text("day").filter(|day| crate::comment::is_day(day));
    Ok(Proposal::Comment {
        channel: channel.to_owned(),
        work_id,
        author: text("author"),
        body,
        commented_on,
        about: text("about"),
    })
}

/// What an action that produces scenes appends to its prompt.
///
/// The kind's words are spelled out with their labels and hints, the way a
/// score's axes are, because a block under a key the kind does not name is
/// refused whole and a model handed only `"blocks": {}` invents its own. The
/// shape differs by what the action does to the board: a replaced board is
/// numbered from 1 in order, an added scene may leave its number out, and a
/// revision names the numbers it changes. `only` narrows a revision to one
/// scene — the one the action was started on — and `only_block` narrows it
/// further to one prompt block of that scene.
pub fn scenes_instruction(
    kind: &WorkKind,
    change: BoardChange,
    only: Option<i64>,
    only_block: Option<&str>,
) -> String {
    let shots = if kind.shot_types.is_empty() {
        String::new()
    } else {
        format!(
            "\nKinds of shot, by key: {}.\n",
            kind.shot_types
                .iter()
                .map(|shot| format!("`{}` = {}", shot.key, shot.label))
                .collect::<Vec<_>>()
                .join("; ")
        )
    };
    let blocks = if kind.scene_blocks.is_empty() {
        String::new()
    } else {
        let mut listed =
            String::from("\nPrompt blocks, by key — each is the text a generator is given:\n");
        for block in &kind.scene_blocks {
            listed.push_str(&format!("- `{}` — {}", block.key, block.label));
            if let Some(hint) = block
                .hint
                .as_ref()
                .map(Label::as_str)
                .filter(|hint| !hint.trim().is_empty())
            {
                listed.push_str(&format!(": {}", hint.trim()));
            }
            listed.push('\n');
        }
        listed
    };

    let numbering = match (change, only) {
        (BoardChange::Replace, _) => {
            "The scenes are the whole board, numbered from 1 in order; give every field you can."
        }
        (BoardChange::Add, _) => {
            "The scenes go after the last one on the board; leave `position` out."
        }
        (BoardChange::Revise, None) => {
            "Give only the scenes you change, each with its `position` as numbered on the board, and only the fields you change: a field left out is kept, `blocks` are set together."
        }
        (BoardChange::Revise, Some(_)) => {
            "Give this one scene only, with its `position` exactly as numbered on the board, and only the fields you change: a field left out is kept, `blocks` are set together."
        }
    };
    let only = match (only, only_block) {
        // Aimed at one block: the scene is given whole so its other prompts
        // are there to write against, but only this one comes back. The
        // others are not the answer's to touch — and an answer that brought
        // them would be laid over the scene, not put in its place.
        (Some(position), Some(block)) => format!(
            " The scene is number {position}. Give the `{block}` block only: leave every other block out, and change nothing else about the scene."
        ),
        (Some(position), None) => format!(" The scene is number {position}."),
        (None, _) => String::new(),
    };

    let shot_line = if kind.shot_types.is_empty() {
        String::new()
    } else {
        "      \"shot_type\": \"<a key from the kinds of shot>\",\n".to_owned()
    };
    let block_lines = kind
        .scene_blocks
        .iter()
        .map(|block| format!("        \"{}\": \"<text>\"", block.key))
        .collect::<Vec<_>>()
        .join(",\n");
    let blocks_line = if kind.scene_blocks.is_empty() {
        String::new()
    } else {
        format!("      \"blocks\": {{\n{block_lines}\n      }}\n")
    };

    format!(
        "\n\nEnd your reply with a fenced json block, exactly this shape and nothing else \
         inside it:\n\n```json\n{{\n  \"scenes\": [\n    {{\n      \"position\": <number from 1>,\n      \
         \"section\": \"<the part of the text it plays against>\",\n      \"starts_at\": <seconds>,\n      \
         \"ends_at\": <seconds>,\n{shot_line}      \"description\": \"<what happens in the scene>\",\n{blocks_line}    }}\n  ]\n}}\n```\n\n\
         {numbering}{only} Leave out a field you have nothing for rather than inventing it.\n{shots}{blocks}\
         Say whatever you like above the block. Use only the keys listed."
    )
}

/// What reading a storyboard out of an answer came to.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadScenes {
    /// No block: an answer to read.
    Nothing,
    /// A board, checked against the kind.
    Proposal(Box<Proposal>),
    /// A block that could not become a board, and why — said rather than
    /// dropped, because a button that never appears is a silent nothing.
    Refused(String),
}

/// Find a storyboard in an answer, if it holds one.
///
/// A block in the wrong words is not silently nothing: the answer is still
/// an answer, but the reason there is no button travels with it. `only`
/// narrows a revision to the scene the action was about — a revision that
/// numbers another scene is refused rather than applied to a stranger — and
/// `only_block` narrows it to one prompt block of that scene the same way.
pub fn read_scenes(
    body: &str,
    kind: &WorkKind,
    change: BoardChange,
    only: Option<i64>,
    only_block: Option<&str>,
) -> ReadScenes {
    let Some(block) = fenced_json(body) else {
        return ReadScenes::Nothing;
    };
    let raw: RawScenes = match serde_json::from_str(&block) {
        Ok(raw) => raw,
        Err(error) => {
            return ReadScenes::Refused(format!("the json block is not a board: {error}"));
        }
    };
    if raw.scenes.is_empty() {
        return ReadScenes::Refused("the json block names no scenes".into());
    }
    let scenes = match scenes_from(&raw.scenes, kind) {
        Ok(scenes) => scenes,
        Err(error) => return ReadScenes::Refused(error.to_string()),
    };
    if change == BoardChange::Revise {
        if let Some(missing) = scenes.iter().position(|scene| scene.position.is_none()) {
            return ReadScenes::Refused(format!(
                "scene {} of the revision has no `position`; a revision names the numbers it changes",
                missing + 1
            ));
        }
        if let Some(only) = only {
            if let Some(other) = scenes
                .iter()
                .find(|scene| scene.position != Some(only))
                .and_then(|scene| scene.position)
            {
                return ReadScenes::Refused(format!(
                    "the answer revised scene {other}; the action was about scene {only}"
                ));
            }
        }
        // Aimed at one block, an answer that writes another is refused the
        // same way an answer about another scene is. It would be applied —
        // the blocks are laid over, not put in place — but it would still
        // rewrite a prompt nobody asked it to, and quietly.
        if let Some(only_block) = only_block {
            if let Some(other) = scenes
                .iter()
                .flat_map(|scene| scene.blocks.keys())
                .find(|key| key.as_str() != only_block)
            {
                return ReadScenes::Refused(format!(
                    "the answer wrote the `{other}` block; the action was about `{only_block}`"
                ));
            }
        }
    }
    ReadScenes::Proposal(Box::new(Proposal::Scenes { scenes, change }))
}

#[derive(Deserialize)]
struct RawScenes {
    #[serde(default)]
    scenes: Vec<Value>,
}

/// Find a scoring proposal in an answer, if it holds one.
///
/// Returns `None` rather than an error when there is no block: an action can be
/// asked for a score and answer in prose anyway, and that is a reply to read,
/// not a failure to report.
pub fn read_score(body: &str, config: &ProfileConfig, kind: &str) -> Option<Proposal> {
    let raw: RawScore = serde_json::from_str(&fenced_json(body)?).ok()?;
    score_from(raw.axes, raw.note, config.vocabulary(kind))
}

/// A scoring proposal out of marks by axis key, checked against the profile.
///
/// The check the fenced block gets, and the same one an agent's `propose_score`
/// gets: unknown axes are named rather than dropped silently, marks are clamped
/// to the axis scale, and marks on no known axis at all are nothing to propose.
pub fn score_from(
    raw: Map<String, Value>,
    note: Option<String>,
    config: &WorkKind,
) -> Option<Proposal> {
    let Marks {
        axes,
        note,
        unknown,
        missing,
    } = marks_from(raw, note, config)?;
    Some(Proposal::Score {
        axes,
        note,
        unknown,
        missing,
    })
}

/// Marks by axis key, checked against a kind — see [`score_from`].
pub fn marks_from(
    raw: Map<String, Value>,
    note: Option<String>,
    config: &WorkKind,
) -> Option<Marks> {
    let mut axes = Map::new();
    let mut unknown = Vec::new();

    for (key, value) in raw {
        match config.axes.iter().find(|axis| axis.key == key) {
            // Clamped rather than refused: an answer that says 11 out of 10
            // means "as high as it goes", and throwing the whole proposal away
            // over one number would waste the rest of it.
            Some(axis) => {
                if let Some(number) = value.as_f64() {
                    let bounded = number.clamp(0.0, axis.scale);
                    axes.insert(key, serde_json::json!(bounded));
                } else {
                    unknown.push(key);
                }
            }
            None => unknown.push(key),
        }
    }

    let missing: Vec<String> = config
        .axes
        .iter()
        .filter(|axis| !axes.contains_key(&axis.key))
        .map(|axis| axis.key.clone())
        .collect();

    // Nothing usable is nothing to propose. A block naming only axes the
    // profile dropped is not a score, and offering to apply it would be
    // offering to write an empty snapshot.
    if axes.is_empty() {
        return None;
    }

    unknown.sort();

    Some(Marks {
        axes,
        note: note.filter(|note| !note.trim().is_empty()),
        unknown,
        missing,
    })
}

/// Overview fields out of a map by key, checked against the profile.
///
/// The same bargain as the axes: a key the profile has no field for is named
/// rather than written — a value under an unknown key would sit in `meta`
/// where no screen shows it. Values are kept as given; the overview stores
/// what is typed and a number that is not one yet stays as typed there too.
/// Empty values are dropped: an agent that sends `""` for a field it has
/// nothing to say about must not blank what is there.
/// The fields of a proposed release, split into what the release kind has and
/// what it does not.
///
/// The same rule a work's overview fields follow: an unknown key is reported
/// rather than stored, and a blank value is dropped rather than written as an
/// empty box. The vocabulary is the release kind's, not the profile's — a
/// clip and a beta read are asked for different things, and a key belonging
/// to the other one is as unknown here as a key belonging to nothing.
pub fn release_fields_from(
    raw: Map<String, Value>,
    kind: &crate::profile::config::ReleaseKind,
) -> (Map<String, Value>, Vec<String>) {
    let mut fields = Map::new();
    let mut unknown = Vec::new();

    for (key, value) in raw {
        if !kind.fields.iter().any(|field| field.key == key) {
            unknown.push(key);
            continue;
        }
        let empty = match &value {
            Value::Null => true,
            Value::String(text) => text.trim().is_empty(),
            _ => false,
        };
        if !empty {
            fields.insert(key, value);
        }
    }

    unknown.sort();
    (fields, unknown)
}

pub fn fields_from(
    raw: Map<String, Value>,
    config: &ProfileConfig,
) -> (Map<String, Value>, Vec<String>) {
    let mut fields = Map::new();
    let mut unknown = Vec::new();

    for (key, value) in raw {
        if !config.work_meta_fields.iter().any(|field| field.key == key) {
            unknown.push(key);
            continue;
        }
        let empty = match &value {
            Value::Null => true,
            Value::String(text) => text.trim().is_empty(),
            _ => false,
        };
        if !empty {
            fields.insert(key, value);
        }
    }

    unknown.sort();
    (fields, unknown)
}

/// Scenes out of what an agent sent, checked against the kind — the check
/// the storyboard's own writer makes, made here so the package is refused
/// whole before it lands, the way an unknown role refuses a package.
///
/// A shot type or a block key the kind does not name is a refusal, not an
/// omission: a scene typed as `closeup` when the vocabulary says `close`
/// would never be found by the board's filter, and a block under a key no
/// template reads would never be shown. A kind with no storyboard at all
/// takes no scenes. Numbers, when given, start at 1; a scene cannot end
/// before it starts.
pub fn scenes_from(raw: &[Value], kind: &WorkKind) -> crate::error::Result<Vec<PackagedScene>> {
    use crate::error::Error;

    if kind.shot_types.is_empty() && kind.scene_blocks.is_empty() {
        return Err(Error::Other(format!(
            "`{}` has no storyboard: the kind names no kinds of shot and no prompt blocks",
            kind.key
        )));
    }
    let names = |keys: &mut dyn Iterator<Item = &str>| {
        let listed: Vec<String> = keys.map(|key| format!("`{key}`")).collect();
        if listed.is_empty() {
            "none".to_owned()
        } else {
            listed.join(", ")
        }
    };

    let mut scenes = Vec::with_capacity(raw.len());
    for (index, item) in raw.iter().enumerate() {
        let Some(item) = item.as_object() else {
            return Err(Error::Other(format!(
                "scene {} is not an object",
                index + 1
            )));
        };
        let text = |key: &str| {
            item.get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        let number = |key: &str| item.get(key).and_then(Value::as_f64);

        let position = match item.get("position") {
            None | Some(Value::Null) => None,
            Some(value) => match value.as_i64() {
                Some(position) if position >= 1 => Some(position),
                _ => {
                    return Err(Error::Other(format!(
                        "scene {}: a scene is numbered from 1",
                        index + 1
                    )));
                }
            },
        };

        let shot_type = text("shot_type");
        if let Some(shot) = shot_type.as_deref() {
            if !kind.shot_types.iter().any(|s| s.key == shot) {
                return Err(Error::Other(format!(
                    "scene {}: no kind of shot `{shot}` for `{}`; its kinds of shot are {}",
                    index + 1,
                    kind.key,
                    names(&mut kind.shot_types.iter().map(|s| s.key.as_str()))
                )));
            }
        }

        let mut blocks = Map::new();
        if let Some(given) = item.get("blocks") {
            let Some(given) = given.as_object() else {
                return Err(Error::Other(format!(
                    "scene {}: `blocks` must be an object of block key to text",
                    index + 1
                )));
            };
            for (key, value) in given {
                if !kind.scene_blocks.iter().any(|b| b.key == *key) {
                    return Err(Error::Other(format!(
                        "scene {}: no prompt block `{key}` for `{}`; its blocks are {}",
                        index + 1,
                        kind.key,
                        names(&mut kind.scene_blocks.iter().map(|b| b.key.as_str()))
                    )));
                }
                // A block is text; a blank one is no block, so an agent that
                // sends "" for what it has nothing to say about leaves the box
                // empty rather than filling it with nothing.
                match value {
                    Value::String(body) if !body.trim().is_empty() => {
                        blocks.insert(key.clone(), Value::String(body.clone()));
                    }
                    Value::String(_) | Value::Null => {}
                    other => {
                        blocks.insert(key.clone(), Value::String(other.to_string()));
                    }
                }
            }
        }

        let starts_at = number("starts_at");
        let ends_at = number("ends_at");
        if let (Some(from), Some(to)) = (starts_at, ends_at) {
            if to < from {
                return Err(Error::Other(format!(
                    "scene {}: it ends at {to} before it starts at {from}",
                    index + 1
                )));
            }
        }
        for at in [starts_at, ends_at].into_iter().flatten() {
            if !(at.is_finite() && at >= 0.0) {
                return Err(Error::Other(format!(
                    "scene {}: seconds are counted from 0",
                    index + 1
                )));
            }
        }

        scenes.push(PackagedScene {
            position,
            section: text("section"),
            starts_at,
            ends_at,
            shot_type,
            description: text("description").unwrap_or_default(),
            blocks,
        });
    }
    Ok(scenes)
}

/// The contents of the last fenced json block in a body.
///
/// The last, not the first: an answer may show an example of the shape before
/// filling it in, and what it settled on is what comes last. The fence is
/// required — a bare object in prose is too easy to find by accident.
fn fenced_json(body: &str) -> Option<String> {
    let mut found: Option<String> = None;
    let mut current: Option<Vec<&str>> = None;

    for line in body.lines() {
        let trimmed = line.trim();
        match &mut current {
            None => {
                // ```json, ```JSON, or ``` json. Anything else — prose, or a
                // fence of another language — is skipped rather than ending
                // the search: the block worth reading usually comes after
                // several lines that are neither.
                if let Some(rest) = trimmed.strip_prefix("```") {
                    if rest.trim().eq_ignore_ascii_case("json") {
                        current = Some(Vec::new());
                    }
                }
            }
            Some(collected) => {
                if trimmed.starts_with("```") {
                    found = Some(collected.join("\n"));
                    current = None;
                } else {
                    collected.push(line);
                }
            }
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;

    fn config() -> ProfileConfig {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        profile::active(&conn).unwrap().unwrap().config
    }

    fn block(inner: &str) -> String {
        format!("Here is what I think.\n\n```json\n{inner}\n```")
    }

    /// The instructions are prose a model reads: a run of spaces in the middle
    /// of a sentence is a line continuation that went wrong in the source —
    /// the class of damage a patch script did here once, which compiles and
    /// reads as garbage.
    #[test]
    fn the_comment_instructions_read_as_sentences() {
        for text in [
            comment_instruction("2026-09-22"),
            reply_instruction().to_owned(),
        ] {
            assert!(!text.contains("   "), "a hole in the prose: {text:?}");
            assert!(
                text.starts_with("\n\n"),
                "set apart from the prompt above it"
            );
        }
        assert!(comment_instruction("2026-09-22").contains("```json\n{\n  \"author\""));
    }

    #[test]
    fn a_comment_is_read_out_of_its_block() {
        let answer = block(
            r#"{"author": "anna", "text": "  loved the bridge  ", "day": "2026-02-30", "about": "Harbour lights"}"#,
        );

        let Ok(Proposal::Comment {
            channel,
            body,
            author,
            commented_on,
            about,
            work_id,
        }) = read_comment(&answer, "main", Some("w1".into()))
        else {
            panic!("a comment should have been read");
        };

        assert_eq!(channel, "main");
        assert_eq!(work_id.as_deref(), Some("w1"));
        assert_eq!(body, "loved the bridge");
        assert_eq!(author.as_deref(), Some("anna"));
        assert_eq!(
            commented_on, None,
            "the 30th of February is dropped, not kept"
        );
        assert_eq!(about.as_deref(), Some("Harbour lights"));
    }

    #[test]
    fn a_comment_with_no_words_is_refused_with_the_reason() {
        assert!(read_comment("no block at all", "main", None).is_err());
        assert!(read_comment(&block(r#"{"author": "anna", "text": "  "}"#), "main", None).is_err());
        assert!(read_comment(&block(r#"{"text": null}"#), "main", None).is_err());
    }

    fn score_of(
        proposal: Proposal,
    ) -> (Map<String, Value>, Option<String>, Vec<String>, Vec<String>) {
        let Proposal::Score {
            axes,
            note,
            unknown,
            missing,
        } = proposal
        else {
            panic!("a fenced block reads as a score");
        };
        (axes, note, unknown, missing)
    }

    #[test]
    fn a_well_formed_block_is_read() {
        let config = config();
        let first = config.work_kinds[0].axes[0].key.clone();
        let body = block(&format!(
            r#"{{"axes": {{"{first}": 7}}, "note": "the chorus carries it"}}"#
        ));

        let (axes, note, _, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(axes.get(&first).and_then(Value::as_f64), Some(7.0));
        assert_eq!(note.as_deref(), Some("the chorus carries it"));
    }

    #[test]
    fn prose_without_a_block_proposes_nothing() {
        let config = config();

        assert_eq!(
            read_score("I would call it strong, maybe a 7.", &config, "song"),
            None,
            "an answer in prose is a reply to read, not a failure"
        );
    }

    #[test]
    fn an_axis_the_profile_does_not_have_is_named_not_applied() {
        let config = config();
        let first = config.work_kinds[0].axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": 5, "vibes": 9}}}}"#));

        let (axes, _, unknown, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert!(!axes.contains_key("vibes"));
        assert_eq!(unknown, vec!["vibes".to_owned()]);
    }

    #[test]
    fn axes_the_answer_skipped_are_reported() {
        let config = config();
        let first = config.work_kinds[0].axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": 5}}}}"#));

        let (_, _, _, missing) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(missing.len(), config.work_kinds[0].axes.len() - 1);
        assert!(!missing.contains(&first));
    }

    #[test]
    fn a_value_past_the_scale_is_clamped_not_refused() {
        let config = config();
        let axis = config.work_kinds[0].axes[0].clone();
        let body = block(&format!(
            r#"{{"axes": {{"{}": {}}}}}"#,
            axis.key,
            axis.scale + 4.0
        ));

        let (axes, _, _, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(
            axes.get(&axis.key).and_then(Value::as_f64),
            Some(axis.scale),
            "\"as high as it goes\" is an opinion worth keeping"
        );
    }

    #[test]
    fn a_negative_value_is_clamped_to_zero() {
        let config = config();
        let first = config.work_kinds[0].axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": -3}}}}"#));

        let (axes, _, _, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(axes.get(&first).and_then(Value::as_f64), Some(0.0));
    }

    #[test]
    fn a_block_naming_only_unknown_axes_proposes_nothing() {
        let config = config();
        let body = block(r#"{"axes": {"vibes": 9, "energy": 4}}"#);

        assert_eq!(
            read_score(&body, &config, "song"),
            None,
            "an empty snapshot is not worth offering to write"
        );
    }

    #[test]
    fn the_last_block_wins_over_an_example() {
        let config = config();
        let axis = config.work_kinds[0].axes[0].key.clone();
        let body = format!(
            "The shape looks like this:\n\n```json\n{{\"axes\": {{\"{axis}\": 0}}}}\n```\n\n\
             And here is my actual answer:\n\n```json\n{{\"axes\": {{\"{axis}\": 8}}}}\n```"
        );

        let (axes, _, _, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(
            axes.get(&axis).and_then(Value::as_f64),
            Some(8.0),
            "what the answer settled on comes last"
        );
    }

    #[test]
    fn an_unfenced_object_is_not_a_proposal() {
        let config = config();
        let axis = config.work_kinds[0].axes[0].key.clone();
        let body = format!("I would write {{\"axes\": {{\"{axis}\": 7}}}} if you asked.");

        assert_eq!(read_score(&body, &config, "song"), None);
    }

    #[test]
    fn a_block_of_another_language_is_ignored() {
        let config = config();
        let axis = config.work_kinds[0].axes[0].key.clone();
        let body = format!("```python\n{{\"axes\": {{\"{axis}\": 7}}}}\n```");

        assert_eq!(read_score(&body, &config, "song"), None);
    }

    #[test]
    fn broken_json_inside_the_fence_proposes_nothing() {
        let config = config();

        assert_eq!(read_score(&block("{axes: oops"), &config, "song"), None);
    }

    #[test]
    fn an_unclosed_fence_proposes_nothing() {
        let config = config();
        let axis = config.work_kinds[0].axes[0].key.clone();
        let body = format!("```json\n{{\"axes\": {{\"{axis}\": 7}}}}");

        assert_eq!(
            read_score(&body, &config, "song"),
            None,
            "a block that never ended may be a truncated answer"
        );
    }

    #[test]
    fn a_blank_note_is_dropped_rather_than_stored_empty() {
        let config = config();
        let first = config.work_kinds[0].axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": 5}}, "note": "   "}}"#));

        let (_, note, _, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(note, None);
    }

    #[test]
    fn the_instruction_names_every_axis_with_its_scale() {
        let config = config();

        let instruction = scoring_instruction(&config, "song");

        for axis in &config.work_kinds[0].axes {
            assert!(
                instruction.contains(&axis.key),
                "every axis must be named: {}",
                axis.key
            );
        }
        assert!(
            instruction.contains("```json"),
            "the block has to be findable"
        );
    }

    #[test]
    fn fields_are_checked_against_the_profile_and_blanks_are_dropped() {
        let config = config();
        let first = config.work_meta_fields[0].key.clone();
        let mut raw = Map::new();
        raw.insert(first.clone(), Value::from("three"));
        raw.insert("colour".into(), Value::from("blue"));
        raw.insert("premise".into(), Value::from("   "));

        let (fields, unknown) = fields_from(raw, &config);

        assert_eq!(fields.get(&first).and_then(Value::as_str), Some("three"));
        assert!(
            !fields.contains_key("premise"),
            "a blank value must not blank a field"
        );
        assert_eq!(unknown, vec!["colour".to_owned()]);
    }

    #[test]
    fn a_stored_proposal_reads_back_as_it_was_written() {
        let proposal = Proposal::Work {
            title: Some("Harbour lights".into()),
            work_kind: Some("song".into()),
            fields: Map::new(),
            unknown_fields: Vec::new(),
            versions: vec![PackagedVersion {
                role: "lyrics".into(),
                body: "one line".into(),
                label: None,
            }],
            score: None,
            notes: Vec::new(),
            scenes: Vec::new(),
            releases: Vec::new(),
        };
        let stored = serde_json::to_value(&proposal).unwrap();
        assert_eq!(stored["kind"], "work");
        assert!(
            stored.get("fields").is_none(),
            "empty parts are not written"
        );
        let read: Proposal = serde_json::from_value(stored).unwrap();
        assert_eq!(read, proposal);
    }

    #[test]
    fn a_storyboard_block_is_read_and_a_wrong_word_is_refused_with_its_reason() {
        let config = config();
        let kind = config.vocabulary("video");

        let read = read_scenes(
            &block(
                r#"{"scenes": [{"section": "intro", "shot_type": "wide", "description": "the harbour", "blocks": {"still": "cranes"}}]}"#,
            ),
            kind,
            BoardChange::Replace,
            None,
            None,
        );
        let ReadScenes::Proposal(boxed) = read else {
            panic!("a board is read: {read:?}");
        };
        let Proposal::Scenes { scenes, change } = *boxed else {
            panic!("a board");
        };
        assert_eq!(change, BoardChange::Replace);
        assert_eq!(scenes[0].shot_type.as_deref(), Some("wide"));
        assert_eq!(scenes[0].blocks["still"], "cranes");

        assert_eq!(
            read_scenes("only prose", kind, BoardChange::Replace, None, None),
            ReadScenes::Nothing
        );
        let ReadScenes::Refused(why) = read_scenes(
            &block(r#"{"scenes": [{"shot_type": "closeup"}]}"#),
            kind,
            BoardChange::Replace,
            None,
            None,
        ) else {
            panic!("a wrong word is refused with its reason");
        };
        assert!(why.contains("no kind of shot `closeup`"), "{why}");
        let ReadScenes::Refused(why) = read_scenes(
            &block(r#"{"scenes": []}"#),
            kind,
            BoardChange::Add,
            None,
            None,
        ) else {
            panic!("an empty board is refused");
        };
        assert!(why.contains("names no scenes"), "{why}");
    }

    #[test]
    fn a_revision_names_its_numbers_and_holds_to_its_scene() {
        let config = config();
        let kind = config.vocabulary("video");

        let ReadScenes::Refused(why) = read_scenes(
            &block(r#"{"scenes": [{"blocks": {"still": "x"}}]}"#),
            kind,
            BoardChange::Revise,
            None,
            None,
        ) else {
            panic!("a revision without numbers is refused");
        };
        assert!(why.contains("has no `position`"), "{why}");

        let ReadScenes::Refused(why) = read_scenes(
            &block(r#"{"scenes": [{"position": 2, "blocks": {"still": "x"}}]}"#),
            kind,
            BoardChange::Revise,
            Some(1),
            None,
        ) else {
            panic!("a revision of another scene is refused");
        };
        assert!(
            why.contains("revised scene 2; the action was about scene 1"),
            "{why}"
        );

        let ReadScenes::Proposal(boxed) = read_scenes(
            &block(r#"{"scenes": [{"position": 1, "blocks": {"still": "x"}}]}"#),
            kind,
            BoardChange::Revise,
            Some(1),
            None,
        ) else {
            panic!("the scene's own revision is read");
        };
        let Proposal::Scenes { scenes, change } = *boxed else {
            panic!("a board");
        };
        assert_eq!(change, BoardChange::Revise);
        assert_eq!(scenes[0].position, Some(1));
    }

    /// An action aimed at one block says so, and holds the answer to it: a
    /// reply that rewrites another block is refused rather than applied
    /// quietly over a prompt nobody asked about.
    #[test]
    fn an_action_about_one_block_asks_for_it_and_holds_the_answer_to_it() {
        let config = config();
        let kind = config.vocabulary("video");

        let aimed = scenes_instruction(kind, BoardChange::Revise, Some(2), Some("motion"));
        assert!(aimed.contains("The scene is number 2."), "{aimed}");
        assert!(
            aimed.contains("`motion` block only"),
            "the instruction names the block: {aimed}"
        );
        assert!(
            aimed.contains("leave every other block out"),
            "and says what to leave alone: {aimed}"
        );
        // A continued line in Rust keeps the next line's indentation unless
        // the backslash eats it, and `cargo fmt` then joins the two with the
        // spaces still in. The sentence is read by a model and, in the
        // preview dialog, by a person; a run of spaces inside it is the tell.
        // Checked on the prose, not on the json example above it, which is
        // indented on purpose.
        let prose = aimed
            .rsplit("```")
            .next()
            .expect("the instruction carries its example");
        assert!(
            !prose.contains("  "),
            "the sentence carries a wrapped line's indentation: {prose:?}"
        );

        let read = read_scenes(
            &block(r#"{"scenes": [{"position": 2, "blocks": {"motion": "a slower push"}}]}"#),
            kind,
            BoardChange::Revise,
            Some(2),
            Some("motion"),
        );
        assert!(
            matches!(read, ReadScenes::Proposal(_)),
            "the block it was asked for is read"
        );

        let ReadScenes::Refused(why) = read_scenes(
            &block(
                r#"{"scenes": [{"position": 2, "blocks": {"motion": "ok", "still": "and this"}}]}"#,
            ),
            kind,
            BoardChange::Revise,
            Some(2),
            Some("motion"),
        ) else {
            panic!("an answer that writes another block is refused");
        };
        assert!(
            why.contains("`still` block") && why.contains("about `motion`"),
            "{why}"
        );
    }

    #[test]
    fn the_scenes_instruction_names_the_kinds_words_and_the_change() {
        let config = config();
        let kind = config.vocabulary("video");

        let whole = scenes_instruction(kind, BoardChange::Replace, None, None);
        assert!(whole.contains("`wide` = Wide"), "{whole}");
        assert!(
            whole.contains("- `still` — Still frame: The frame as a picture"),
            "{whole}"
        );
        assert!(whole.contains("numbered from 1 in order"), "{whole}");
        assert!(
            whole.contains("\"shot_type\": \"<a key from the kinds of shot>\""),
            "{whole}"
        );

        let one = scenes_instruction(kind, BoardChange::Revise, Some(3), None);
        assert!(one.contains("this one scene only"), "{one}");
        assert!(one.contains("The scene is number 3."), "{one}");

        let added = scenes_instruction(kind, BoardChange::Add, None, None);
        assert!(added.contains("leave `position` out"), "{added}");
    }
}

#[cfg(test)]
mod instruction_tests {
    use super::*;
    use crate::profile::config::{Axis, AxisKind, AxisMark, Tier, WorkKind};

    /// The model is told what the author wrote about each axis — the label,
    /// the question, the marks of the rubric, the weight — and what the
    /// total means; not only a key and a ceiling.
    #[test]
    fn the_scoring_instruction_carries_labels_rubrics_and_tiers() {
        let mut kind = WorkKind::new("song", "Song");
        kind.axes.push(Axis {
            key: "imagery".into(),
            label: "Imagery".into(),
            weight: 2.0,
            scale: 10.0,
            description: Some("Can the picture be seen?".into()),
            kind: AxisKind::Scale,
            options: Vec::new(),
            rubric: vec![
                AxisMark {
                    at: 3.0,
                    label: "worn images".into(),
                },
                AxisMark {
                    at: 8.0,
                    label: "one fresh image per verse".into(),
                },
            ],
        });
        kind.tiers.push(Tier {
            key: "hold".into(),
            label: "Hold".into(),
            min: 55.0,
        });
        kind.tiers.push(Tier {
            key: "clip".into(),
            label: "Clip".into(),
            min: 80.0,
        });
        let config = ProfileConfig {
            format: crate::profile::config::FORMAT,
            work_kinds: vec![kind],
            collection_kinds: Vec::new(),
            work_meta_fields: Vec::new(),
            marks: Vec::new(),
            stages: Vec::new(),
            prompts: Vec::new(),
            rhythm: None,
            catalogue_columns: None,
            catalogue_columns_by_kind: None,
            note_kinds: Vec::new(),
            style_types: Vec::new(),
        };

        let text = scoring_instruction(&config, "song");

        for expected in [
            "`imagery` — Imagery (0 to 10, weight 2)",
            "Can the picture be seen?",
            "3 — worn images",
            "8 — one fresh image per verse",
            "Clip from 80, Hold from 55",
            "\"imagery\": <0-10>",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
    }

    #[test]
    fn a_version_instruction_names_the_role() {
        let text = version_instruction("Critique");
        assert!(text.contains("kept as the Critique"), "{text}");
    }
}
