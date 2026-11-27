//! The tools as a client is told them: each by its name, what it is for and
//! the shape of its arguments.
//!
//! What a description promises, the function of the same name in
//! [`super::tools`] keeps: the two are read together when a tool changes.

use serde_json::{Map, Value, json};

use super::call::MOST;
use crate::profile::config::SectionShape;

/// One entry of the tool list, with the shape of its arguments.
fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    // Every tool reads or proposes in one space: the one kilna last opened,
    // or the one `space` names (ADR 0077).
    let mut properties = properties;
    if let Some(map) = properties.as_object_mut() {
        map.insert(
            "space".into(),
            json!({
                "type": "string",
                "description": "A space by name, as `workspace` lists them; the one kilna last opened when left out",
            }),
        );
    }
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": properties, "required": required },
    })
}

fn work_arg() -> Value {
    json!({ "type": "string", "description": "A work: its id, or its exact title" })
}

/// The schema of a list of scenes, the same for a package and a storyboard.
fn scenes_arg(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": { "type": "object", "properties": {
            "position": { "type": "integer", "description": "The number on the board, from 1; after the last when omitted" },
            "section": text_arg("The part of the text it plays against: intro, verse 1, chorus"),
            "starts_at": { "type": "number", "description": "Seconds from the start of the video" },
            "ends_at": { "type": "number", "description": "Seconds from the start; not before `starts_at`" },
            "shot_type": text_arg("A key of the kind's `shot_types`"),
            "layer": text_arg("A key of the kind's `layers`"),
            "description": text_arg("What happens in the scene"),
            "blocks": { "type": "object", "description": "Prompt blocks by the kind's `scene_blocks` key, each the text a generator is given", "additionalProperties": { "type": "string" } },
            "cover_bricks": { "type": "object", "description": "The scene's own bricks over the clip's cover, by id from `styles`: style, background, composition, look, environment, angle, pose, layering, lighting, optics, grading - each a brick of the type of its own name" },
            "motion_bricks": { "type": "array", "items": { "type": "string" }, "description": "The movements of the shot, up to five brick ids of the Motion branch (`styles` with `branch`: motion); the ones the scene's picture cannot take by their contracts are left out of its loop" },
        } },
    })
}

/// How many of a long list to give at once.
fn limit_arg(default: usize) -> Value {
    json!({
        "type": "integer",
        "description": format!("At most this many; {default} by default, {MOST} at most")
    })
}

/// Where a page starts.
fn offset_arg() -> Value {
    json!({ "type": "integer", "description": "Skip this many: the `offset` of the page before plus its `limit`. 0 by default" })
}

/// The short form of a long list.
fn summary_arg(fields: &str) -> Value {
    json!({
        "type": "boolean",
        "description": format!("Leave the long texts out: only {fields}. Read one whole afterwards by its id")
    })
}

fn text_arg(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

/// The tools, in the order a session uses them: learn the vocabulary, read,
/// then propose.
pub(super) fn tools() -> Vec<Value> {
    // The keys a fact's data may hold, by the shape of its section: the list
    // the write is checked against, so the two cannot drift apart.
    use crate::profile::config::SectionShape;
    let data_keys = format!(
        "What a section of another shape carries: {}",
        [
            SectionShape::Slots,
            SectionShape::Details,
            SectionShape::Palette,
            SectionShape::Marks,
            SectionShape::Styles,
        ]
        .into_iter()
        .map(|shape| crate::canon::fact::data_keys(shape).join("/"))
        .collect::<Vec<_>>()
        .join(", ")
    );
    vec![
        tool(
            "workspace",
            "The space served - its name, its kind, the other spaces `space` may name, and whether \
             another machine holds it, when proposals are refused - and its profile: the craft's \
             vocabulary, per kind of work. Each kind lists its \
             version roles and how each reads, its scoring axes with weights and scales, its \
             tiers, statuses and kinds of release (each with the fields a release of it goes out \
             under and the shape of its cover, 16:9 or 9:16), and, for a kind with a storyboard, \
             its kinds of shot and the prompt blocks a scene carries; whether its works go out \
             under a cover built in the constructor; whether they play under one frame (a \
             still and a loop). A kind with \
             no kind of release never goes out itself - its works go out as what is made from \
             them. Plus the overview fields, each with the kinds it belongs to, the kinds of \
             note (`note_kinds`: a phrase - `line`, one line - and a note; `material` is spent \
             by works; `library` is the library of works of culture the boards link to, with \
             the `media` an entry is in - a medium not `quoted` is never quoted), the boards \
             of a conception (`boards`: kinds of card, their fields and each field's `shape`) \
             and how many \
             works there are. Read this first: a work is judged and versioned in its own kind's \
             keys, and the other tools speak in those keys. Also the style types of the \
             dictionary (`style_types`: key, label, form, families) and the compositions \
             (`compose`): a role a composition writes - a song's style prompt - is picked from \
             phrases of the dictionary, block by block, each block with how many it takes and \
             its rule, and closes with the work's fields as the templates write them.",
            json!({}),
            &[],
        ),
        tool(
            "guide",
            "The skills of this profile - its methods of critique, scoring, writing, video, covers, \
             styles, canon and release, each with when to use it. Without `skill`: the catalogue, \
             narrowed by `kind` and `scope`. With `skill`: the whole method, the message its button \
             sends, which `propose_*` tool its answer goes through, and the live context of the \
             space - each kind's axes with their rubrics, tiers, roles, statuses and doors, the \
             boards and the style types. Read the method before doing the work it describes.",
            json!({
                "skill": text_arg("A skill's key, as the catalogue lists it"),
                "work": text_arg("With `skill`: a work, by id or exact title - the skill's message written for it, and for a short its stretches, variant and build"),
                "kind": text_arg("A work kind key: only the skills for it, and its context alone"),
                "scope": text_arg("What the skill is about: work, scene, selection, style, comment, canon, release, cover, phrases, lab, board"),
            }),
            &[],
        ),
        tool(
            "catalogue",
            "Every work in the profile with its verdict: id, title, kind, status, score total and \
             tier, whether the score is stale (the text changed since), releases out and scheduled, \
             when it was last touched. Filter by a substring of the title, a kind or a status. \
             The answer is a page: `works`, with `total`, `limit`, `offset` and `has_more` - ask \
             for the next with `offset`.",
            json!({
                "query": text_arg("A case-insensitive substring of the title"),
                "kind": text_arg("A work kind key, as `workspace` lists them"),
                "status": text_arg("A status key, as `workspace` lists them"),
                "mood": text_arg("A mood key, as `workspace` lists them under `vibe`: the works that show it, their own or the one of what they were made from"),
                "character": text_arg("A character key, as `workspace` lists them under `vibe`"),
                "complexity": text_arg("A complexity key, as `workspace` lists them under `vibe`"),
                "limit": limit_arg(100),
                "offset": offset_arg(),
                "summary": summary_arg("id, title, kind, status and when it was last touched"),
            }),
            &[],
        ),
        tool(
            "work",
            "One work as its card shows it: the fields and meta, tags, every version by role \
             (id, revision, label, length, which is current, `parent` - the id of the version \
             it was written from, when that is known), the latest score with its axes, \
             the releases, how many notes and scenes, the cover (the prompt written from its \
             concept - picture, negative and, when the title goes apart, typography - with the \
             concept itself), the frame (for a kind that plays under one: the still, built from \
             the cover or written whole, the loop written from its settings, the negative), \
             what it was made from (`sources`, with whether the source has moved on \
             since) and what was made from it (`derived`); for a work that never goes out \
             itself, its `publications` - everything made from it, with where each stands. \
             Bodies are not included — read one with `text`; the storyboard with `scenes`. \
             The versions come a page at a time (`limit`, `offset`; `total_versions`, \
             `has_more_versions`); `summary: true` gives the card's head alone - what it is, \
             where it stands, each role with how many versions it has and which is current, \
             and counts of the rest.",
            json!({
                "work": work_arg(),
                "limit": limit_arg(100),
                "offset": offset_arg(),
                "summary": summary_arg("id, title, kind, status, stage, tags, each role with its count and current version, the latest total and counts of releases, notes, scenes, sources and derived"),
            }),
            &["work"],
        ),
        tool(
            "trials",
            "The board of an experiment - a work whose kind is a lab (see `workspace`: `lab`): \
             its anchors (what every trial keeps, word for word), and every trial by series, in \
             order - its id, `parent` (the trial it varies), `angle` (what it moves), `body` (the \
             text tried out), the bricks it was picked from, `reference` (who to listen to), \
             `outcome` (what came out once heard), `verdict` (keep, drop, or null - not judged), \
             `source` (the song's style it reworks), the anchors it has lost, how many takes it \
             has, and where a kept one went. Plus `choices`: the phrases of the dictionary a \
             trial is written from, with their ids. The trials come a page at a time \
             (`total`, `limit`, `offset`, `has_more`); `summary` leaves the texts out; `trial` \
             reads one trial whole by its id.",
            json!({
                "work": work_arg(),
                "trial": text_arg("One trial of the board, by its id: read whole"),
                "limit": limit_arg(50),
                "offset": offset_arg(),
                "summary": summary_arg("id, series, parent, angle, verdict, the length of the text and how many takes"),
            }),
            &["work"],
        ),
        tool(
            "text",
            "The body of a version: the current version of a role by default, or the version \
             named by id. Plain roles come back exactly as typed; markdown roles are markdown.",
            json!({
                "work": work_arg(),
                "role": text_arg("A version role key; the profile's first role when omitted"),
                "version": text_arg("A version id, to read a particular revision instead of the current one"),
            }),
            &["work"],
        ),
        tool(
            "scores",
            "The score history of a work, newest first: total, tier, the marks per axis, who \
             judged (empty is the author), the note, which version it judged.",
            json!({ "work": work_arg() }),
            &["work"],
        ),
        tool(
            "calendar",
            "The calendar. `releases`: every release with a date, in calendar order - what, of \
             which work, which kind, scheduled for when, released when, the link, whether its \
             date is kept (`slot_pinned_at`); past releases included. `closed`: the time off - \
             spans of days nothing should go out on, `starts_on`..`ends_on` both inside, and \
             why (`label`). Filter with `from` to start at a day. `cross_spaces` - `true` for \
             every other space, or a list of their names - adds `cross_spaces`: on the days this \
             space puts something out or plans it, what the neighbours put out or plan, in compact \
             fields only (space, date, title, cover idea, genre, mood) - to keep two channels' \
             pictures of one day apart without mixing their records.",
            json!({
                "from": text_arg("Only what is on or after this day, YYYY-MM-DD"),
                "cross_spaces": {
                    "description": "true for every other space, or a list of their names",
                    "anyOf": [{ "type": "boolean" }, { "type": "array", "items": { "type": "string" } }],
                },
            }),
            &[],
        ),
        tool(
            "propose_batch_prompts",
            "Propose a month's drafts written in one pass (v0.153): for each work its title, \
             its overview fields, its texts by role (a Suno prompt, a lyric) and the items of \
             the bank it takes, by name - one proposal into the chat named after you, which \
             the person applies whole after reading the checklist the space's quotas give over \
             the month as it would stand. Name the month with `collection` (an id or its exact \
             title) or `month` (YYYY-MM), so the checklist reads its works together. Read \
             `quotas`, the passport's bank and `register` first; a field the profile does not \
             have and a bank item it does not keep are named and left out, an unknown work or \
             role refuses the whole proposal. Nothing is written until the person applies it.",
            json!({
                "collection": text_arg("The month's collection, by id or exact title"),
                "month": text_arg("The month, YYYY-MM"),
                "works": {
                    "type": "array",
                    "description": "Each: {work (id or exact title), title?, fields?: {key: value}, versions?: [{role, body}], bank?: [item names]}",
                    "items": { "type": "object" },
                },
                "register": {
                    "type": "array",
                    "description": "Terms the month spends, kept in the register when it is applied: each {word, scope? (lyrics, title, visual_motif, sound_combo, all), note?}",
                    "items": { "type": "object" },
                },
            }),
            &["works"],
        ),
        tool(
            "propose_media",
            "Propose files you made beside kilna - takes from a music generator, pictures \
             from an image one, a rendered video - each for a work and as what (v0.156). One \
             proposal into the chat named after you; applied by the person, each file is \
             checked, moved into its work's folder (made if it is not there yet) under a name \
             of the work's, and given its role: `final_audio` and `video` become the work's \
             finished file where the kind's rule takes such a file, `final_cover` its cover, \
             `candidate_cover` a picture beside the cover to choose from, `candidate_audio` a \
             take kept in its folder's `candidates`, `asset` a file that only lies in its \
             folder. A path must be a whole path to a file on this machine; a file of the \
             wrong kind for its role refuses the whole proposal. Nothing moves until the \
             person applies it.",
            json!({
                "items": {
                    "type": "array",
                    "description": "Each: {work (id or exact title), path (whole), role: candidate_audio | final_audio | candidate_cover | final_cover | video | asset, rename_to_work?: true, reason?: why it is that work's, confidence?: 0..1}",
                    "items": { "type": "object" },
                },
            }),
            &["items"],
        ),
        tool(
            "quotas",
            "The space's rules checked over works (v0.151): the facts of the passport's quotas \
             section, each a machine predicate - a field, a metric, a comparison, the works it \
             reads, warn or error. Give `work` for one work, `collection` (an id or its exact \
             title) for a collection's works together, `month` (YYYY-MM) for the works going \
             out or planned that month, or nothing for the whole space. Returns how many rules \
             were asked and works measured, and each breach: the rule, the work (none for the \
             works together), what was measured, the value counted or the words found, and \
             what the rule wants. Check a month's drafts before proposing them.",
            json!({
                "work": work_arg(),
                "collection": text_arg("A collection, by id or exact title"),
                "month": text_arg("A month, YYYY-MM"),
            }),
            &[],
        ),
        tool(
            "notes",
            "Notes in the profile, newest first — all of them, or those of one work, or the \
             phrases of one shelf. A phrase and a note are notes of their kinds (see \
             `workspace`: `note_kinds`): a kind marked `line` is a phrase - a word, a line, a \
             quote with `attribution`, whose words it is - and comes with `shelves`, the \
             owner's shelves it stands on, `sung_in`, how many works already say it, and \
             `sung`, how its forms are sung where the record says so (the stress as a capital \
             vowel: write a lyric with these spellings, the plain ones in public text). A kind \
             marked `material` is spent by works, and each of its notes says where it stands \
             in `state`: fresh, used (by the work it names), parked or dropped. An entry of \
             the kind marked `library` is a work of culture: its `title`, its author as \
             `attribution`, its other titles as `aliases`, its `medium` and `year`. Without \
             arguments, also `shelves`: every shelf in the owner's order with how many \
             phrases it holds. The notes come a page at a time, in `notes`, with `total`, \
             `limit`, `offset` and `has_more`; `summary` leaves the bodies out; `note` reads \
             one note whole, by its id or its exact title.",
            json!({
                "work": work_arg(),
                "query": text_arg("A substring of the title, the body or whose words they are"),
                "kind": text_arg("Only notes of this kind, as `workspace` lists them"),
                "state": text_arg("Only notes in this state: fresh, used, parked or dropped"),
                "shelf": text_arg("Only the phrases on this shelf, by id or by name"),
                "note": text_arg("One note, by its id or its exact title: read whole"),
                "limit": limit_arg(50),
                "offset": offset_arg(),
                "summary": summary_arg("id, title (or the first words), kind, state, tags, the length of the body and when it was last touched"),
            }),
            &[],
        ),
        tool(
            "register",
            "The register of repeats: what this body of work has already spent. Without \
             arguments: every word of the record — its word and forms, its kind (noun, adjective, \
             verb and phrase are wording, found in a text by stem; image, scene and pattern are \
             meanings, tied to the works that carry them by name), its strictness (ban, limit, \
             rare; null for a word that is not spent - only sung its own way), how it is \
             sung, topic and note, and `uses`: how many works carry it now. With `text` or \
             `work`: that text checked against it — the terms it takes word for word and how \
             often, and the words it leans on within itself; with `work`, also `guard`: the songs \
             out or booked that the work's song shares a spent term (orange) or a rare word \
             (red within 90 days, orange before) with, and the words kept on purpose. Read it \
             before writing or judging a text; a meaning told in other words is yours to catch. \
             The record comes a page at a time, in `terms`, with `total`, `limit`, `offset` and \
             `has_more`; `summary` gives the word, its kind, its strictness and its uses alone; \
             `term` reads one word whole, by its id or the word itself, with the works that \
             carry it.",
            json!({
                "text": text_arg("A text to check against the register"),
                "work": text_arg("A work whose current text to check: its id, or its exact title"),
                "term": text_arg("One word of the record, by its id or the word itself: read whole"),
                "limit": limit_arg(100),
                "offset": offset_arg(),
                "summary": summary_arg("id, word, kind, strictness and uses"),
            }),
            &[],
        ),
        tool(
            "styles",
            "The style dictionary: the bricks a prompt is built from. A brick of a type of \
             phrase (`form: phrase` in `workspace`) is one phrase the generator reads word for \
             word - `phrase` - with what it means for a person, `explanation` per language, and \
             when to take it. Each brick has its id, type, family, whether it is one of the \
             channel's house bricks, where it came from (the starter set, changed, or the \
             owner's own) and its status (only `ready` ones are offered for a prompt), whether \
             the owner starred it, and its `branch`: `picture` for what a still picture is built \
             from, `motion` for the movements of an animated frame (the form `motion`: what \
             moves, how much, how often, and what stays still), or the key of the composition \
             that writes a text out of it. Narrow by `branch`, by `type`, by `composition` (its \
             types, in its order) or by `query` (a substring of the name, the phrase or the \
             explanation, in any language). Without any of them: each type with its branch and \
             how many bricks it has. The bricks come a page at a time (`bricks`, with `total`, \
             `limit`, `offset`, `has_more`); `summary` leaves the phrases and explanations \
             out; `brick` reads one whole by its id.",
            json!({
                "brick": text_arg("One brick, by its id: read whole"),
                "limit": limit_arg(100),
                "offset": offset_arg(),
                "summary": summary_arg("id, type, name, family, status, branch, whether it is a house brick and starred"),
                "branch": text_arg("`picture`, `motion` or a composition key"),
                "type": text_arg("A style type key, as `workspace` lists them"),
                "composition": text_arg("A composition key, as `workspace` lists them"),
                "query": text_arg("A case-insensitive substring of the name, the phrase or the explanation"),
            }),
            &[],
        ),
        tool(
            "propose_words",
            "Propose words for the owner's record, applied in one click: a phrase to keep \
             (`phrase`: true; a `shelf` by name - made when there is none), how a word is sung \
             (`sung`: [{written, sung}] - the stress as a capital vowel, a respelling as a \
             changed letter; for a homograph, the reading the song means), a term for the \
             register (`strictness`: ban, limit or rare), and the works a meaning is in \
             (`works`: ids or exact titles, for an image or a scene the texts say in other \
             words). A plain string is a phrase to keep. A word the record already keeps is \
             CHANGED by what is proposed: a `strictness` takes the place of the one it has \
             (`null` unspends it), `forms` join the ones it has (or take their place with \
             `replace_forms`: true), `kind`, `topic` and `note` are written over, `works` are \
             named for it - and the proposal says each change, was and becomes, for the person \
             to read. `action`: delete takes a word out of the record (into the trash); \
             `action`: split with `into`: [..] makes a word that is several into them, each \
             with what the one held. One `word` is one thing a text can say: synonyms go in \
             `forms`, and two things written as one with a comma or a slash are warned about. \
             A phrase already kept in the same words is not kept twice. It waits in the chat \
             on `work` when one is named, or in the chat named after you.",
            json!({
                "words": {
                    "type": "array",
                    "description": "The words: a string for a phrase to keep, or {word, action?, into?, forms?, replace_forms?, kind?, topic?, phrase?, shelf?, sung?, strictness?, works?, note?}",
                    "items": {}
                },
                "work": text_arg("The work the words come from, by id or exact title; optional"),
                "note": text_arg("One sentence on what was proposed and why, optional"),
            }),
            &["words"],
        ),
        tool(
            "propose_short",
            "Propose what the space's shorts look like, the variant one short wears, or both - taken \
             in one click. `style` is laid over the style there (read it in `workspace` under \
             `short_style`): a block is changed by the keys it names and no others - `accent` and \
             `second` (#RRGGBB), `header` {on, title_upper, call, font}, `sign` {on, art, blink, size, \
             pulse, sway}, `ground` {color, pattern: plain|blots|dots|grid|stripes, seed, blots: \
             [{x, y, size, color: accent|second, shape: splat|drip|spray|smear}], motion}, `stamp` \
             {on, shape: stamp|plate, text, sub, art, color, angle, x, wear}, `wave` {on, kind: \
             solid|lines|bars, strip}, `karaoke` {on, font, size}, `effects` {zoom, grain, vignette, \
             glitch}, `tail` {on, seconds}. `variants` are changed by `key`, added when new, and \
             taken out by `remove_variants`; a variant names only what it replaces: label, accent, \
             second, sign_art, sign_blink, ground {pattern, seed, blots}, stamp_angle, stamp_x, wave. \
             A picture (`art`, `blink`, `sign_art`, `sign_blink`) is the text of an SVG or a data: \
             address; one read as `<kept: N chars>` stays as it is when sent back or left out. \
             `work` and `variant` propose the variant a short wears, by the variant's key - without \
             them a short wears the one the round gives the day it goes out. The frame's layout is \
             the same for every short and is not proposed. The build is started by the person.",
            json!({
                "style": {
                    "type": "object",
                    "description": "What changes of the space's short style: blocks by key, `variants`, `remove_variants`",
                },
                "work": text_arg("A short, by id or exact title: where the proposal waits, and the one `variant` is for"),
                "variant": text_arg("The key of the variant `work` is to wear"),
                "note": text_arg("One sentence on what was proposed and why, optional"),
            }),
            &[],
        ),
        tool(
            "propose_style",
            "Propose styles for the dictionary, kept in one click or one by one: new bricks of \
             any type (`type` and `name` needed) and rewrites of bricks it has (`id` from \
             `styles` - a rewrite changes only the fields it names). `description` is the words \
             a generator reads, in English; `family` a key of the type's families; `when` when \
             to reach for it; `explanation` what it gives, per language ({en, ru}); `colours` \
             #RRGGBB hex; `contract` when it can go into a picture: `needs` (`subject:person`, \
             `crop:min:bust` - crops run detail < head < bust < waist < full - `scenery:yes`, \
             `title:present`), `provides` (`color:mono`, `camera:moving`) and `conflicts` \
             (`camera:static`, `color:vivid`). A brick whose contract a picture does not meet is \
             left out of that picture's prompt and said, never erased. It waits in the chat on \
             `work` when one is named, or in the chat named after you.",
            json!({
                "styles": {
                    "type": "array",
                    "description": "The styles: {id?, type?, name?, description?, family?, when?, explanation?, colours?, contract?}",
                    "items": {}
                },
                "work": text_arg("The work they are for, by id or exact title; optional"),
                "note": text_arg("One sentence on what was proposed and why, optional"),
            }),
            &["styles"],
        ),
        tool(
            "scenes",
            "The storyboard of a work, in order: each scene's number, section, seconds, kind of \
             shot (a key of the kind's `shot_types`), layer (a key of the kind's `layers`), description and prompt blocks (by the kind's \
             `scene_blocks` key). The shared context is the current version of the `context` \
             role — read it with `text`. Empty for a kind with no storyboard. The scenes come \
             a page at a time (`scenes`, with `total`, `limit`, `offset`, `has_more`); `summary` \
             leaves the prompt blocks out; `scene` reads one whole by its number or id.",
            json!({
                "work": work_arg(),
                "scene": text_arg("One scene of the board, by its number or its id: read whole"),
                "limit": limit_arg(100),
                "offset": offset_arg(),
                "summary": summary_arg("id, number, section, seconds, kind of shot, layer, the first words of the description and how much is written"),
            }),
            &["work"],
        ),
        tool(
            "search",
            "Find anything by text: works by title, versions by body, notes, assistant replies. \
             Each hit names the work it belongs to.",
            json!({ "query": text_arg("What to look for") }),
            &["query"],
        ),
        tool(
            "propose_work",
            "Propose a whole work, or a package of changes to one, applied together with one \
             click. Without `work`: a NEW work — `title` and `kind` are required, plus any of \
             the rest. With `work`: a package for that work — `title` and `kind` are ignored, \
             the rest is added to it. `fields` are the overview fields of the profile (see \
             `workspace`: `fields`), by key — a premise, a mood, a tagline go here, not into a \
             note. `versions` are texts by role; on a new work the first of them in a role that is a draft becomes its text, and the rest wait beside it, each current of its own role, \
             on an existing one they wait beside the current. `score` is marks along the kind's \
             axes. `notes` are notes on the work. `releases` plans releases and says what each \
             goes out as — its title, its description, its tags, the comment pinned under it — \
             by the field keys the kind of release names (see `workspace`). `boards` puts cards \
             on the boards of its conception, as `propose_cards` does - a new song with its \
             research. Unknown fields and axes are named and left out; an unknown role or kind \
             refuses the whole package. Nothing is written until the person applies it.",
            json!({
                "work": text_arg("An existing work to package changes for: its id, or its exact title. Omit to propose a new work"),
                "title": text_arg("The title of the new work; required without `work`"),
                "vibe": {
                    "type": "object",
                    "description": "What the work feels like, a word on each of three lines as `workspace` lists them under `vibe`; leave out a line you cannot tell. Kept only where the author has said nothing.",
                    "properties": {
                        "mood": text_arg("A mood key"),
                        "character": text_arg("A character key"),
                        "complexity": text_arg("A complexity key"),
                    },
                },
                "kind": text_arg("The kind of the new work, as `workspace` lists them; required without `work`"),
                "fields": { "type": "object", "description": "Overview field key to value, as `workspace` lists the fields", "additionalProperties": true },
                "versions": {
                    "type": "array",
                    "description": "Texts by role, each stored exactly as given",
                    "items": { "type": "object", "properties": {
                        "role": text_arg("The version role key, as `workspace` lists them"),
                        "body": text_arg("The whole text of the version"),
                        "label": text_arg("A short name for the version, optional"),
                        "from": text_arg("The id of the work's version in the same role this text rewrites, optional; it becomes the new version's parent"),
                    }, "required": ["role", "body"] },
                },
                "score": {
                    "type": "object",
                    "description": "Marks along the kind's axes, with an optional note",
                    "properties": {
                        "axes": { "type": "object", "additionalProperties": { "type": "number" } },
                        "note": text_arg("One sentence on why, optional"),
                    },
                    "required": ["axes"],
                },
                "notes": {
                    "type": "array",
                    "description": "Notes on the work",
                    "items": { "type": "object", "properties": {
                        "kind": text_arg("The kind of note, as `workspace` lists it; optional"),
                        "title": text_arg("A title for the note, optional"),
                        "body": text_arg("The note itself"),
                        "attribution": text_arg("Whose words a phrase is; optional"),
                    }, "required": ["body"] },
                },
                "scenes": scenes_arg(
                    "The storyboard of a new video, or scenes added after the last of an \
                     existing one's board; only for a kind with a storyboard (see `workspace`: \
                     `shot_types`, `layers`, `scene_blocks`)"
                ),
                "trials": {
                    "type": "array",
                    "description": "Trials for the board of an experiment (a kind with `lab`), in the shape `propose_trials` takes",
                    "items": { "type": "object" },
                },
                "boards": {
                    "type": "array",
                    "description": "Cards for the boards of the work's conception - a song with its research - each board with its cards in the shape `propose_cards` takes",
                    "items": { "type": "object", "properties": {
                        "board": text_arg("The board, by its key: `research`, `scenarios`"),
                        "cards": { "type": "array", "items": { "type": "object" } },
                    }, "required": ["board", "cards"] },
                },
                "releases": {
                    "type": "array",
                    "description": "Releases to plan for the work, each with what it goes out as",
                    "items": { "type": "object", "properties": {
                        "kind": text_arg("The kind of release, as `workspace` lists them for this kind of work"),
                        "scheduled_at": text_arg("The day it goes out, as 2026-10-02. Omit to leave it queued without a date"),
                        "fields": { "type": "object", "description": "What it goes out as, by field key of this kind of release: a title, a description, tags, a comment to pin", "additionalProperties": true },
                    }, "required": ["kind"] },
                },
            }),
            &[],
        ),
        tool(
            "propose_trials",
            "Propose trials for an experiment's board, put on it with one click. Each trial is \
             its text (`body` - the line the generator reads, the most important phrase first), \
             `angle` (what it moves, or where on the field it stands), `series` (the group it \
             stands in), `bricks` (ids or exact phrases of the dictionary it uses), `reference` \
             (who to listen to - never in the body), and optionally `parent` (the id of a trial on \
             the board, or the number of an earlier trial in this list, that it varies), `source` \
             (the id of a song's style version it reworks), `outcome` and `verdict` (keep or drop) \
             for a trial already heard, and `run_first`. Every anchor of the experiment belongs \
             in every body. What is not in the workspace is left out and said.",
            json!({
                "work": work_arg(),
                "trials": {
                    "type": "array",
                    "description": "The trials, in order",
                    "items": { "type": "object", "properties": {
                        "body": text_arg("The text tried out"),
                        "angle": text_arg("What it moves, a few words"),
                        "series": text_arg("The series it stands in"),
                        "bricks": { "type": "array", "items": { "type": "string" }, "description": "Ids or exact phrases of the dictionary it uses" },
                        "reference": text_arg("Who to listen to; never part of the body"),
                        "parent": { "description": "The trial it varies: an id on the board, or the number (from 1) of an earlier trial in this list" },
                        "source": text_arg("The id of a song's style version it reworks"),
                        "outcome": text_arg("What came out, for a trial already heard"),
                        "verdict": text_arg("keep or drop, for a trial already judged"),
                        "run_first": { "type": "boolean", "description": "Where a run starts" },
                    }, "required": ["body"] },
                },
                "note": text_arg("One sentence on what was proposed and why, optional"),
            }),
            &["work", "trials"],
        ),
        tool(
            "propose_scenes",
            "Propose a storyboard for a work whose kind has one — a video, a short: a package \
             of scenes, each with its number, the section of the text it plays against, its \
             seconds, its kind of shot, a description and its prompt blocks. `change` says \
             what happens to the board that is there: `add` (the default) puts the scenes \
             after the last; `replace` makes them the whole board — a scene with the same \
             number is rewritten in place, the rest of the old board goes to the trash; \
             `revise` touches only the numbers named, and only in the fields given — a field \
             left out is kept, `blocks` are set together. Read `scenes` first when the board \
             is not empty, and the `context` and `plot` roles with `text` for what every \
             scene shares. The kind of shot and the block keys are the kind's own words (see \
             `workspace`); an unknown one refuses the whole package. The board lands in the \
             chat on the work, rendered as a table, with an *add to the board*, *replace the \
             board* or *revise the scenes* button; nothing is written until the person \
             applies it.",
            json!({
                "work": work_arg(),
                "scenes": scenes_arg("The scenes, in order"),
                "change": { "type": "string", "enum": ["add", "replace", "revise"], "description": "What happens to the board that is there: add after the last (default), replace the whole board, or revise only the numbered scenes" },
            }),
            &["work", "scenes"],
        ),
        tool(
            "propose_release",
            "Propose what one release goes out under: its title, its description, its tags, the \
             comment pinned under it - by the field keys its kind of release names (see \
             `workspace`). Read the work first, and what it was made from: an audio release, a \
             clip or a short goes out for a song, and its text and fields are the song's. Read \
             the root card of `canon` through the `public` lens for the channel's voice, \
             signature and rules, and the `calendar` for what went out lately - a new \
             description does not sign off or ask the way the last few did. A publication goes \
             out once, so the work names the release. The fields land in the chat on the work, \
             each beside what is written now; the person takes them one by one or all at once. \
             Nothing is written until they do.",
            json!({
                "work": work_arg(),
                "fields": { "type": "object", "description": "What it goes out under, by field key of its kind of release", "additionalProperties": { "type": "string" } },
                "note": text_arg("One sentence on the choices made, optional"),
            }),
            &["work", "fields"],
        ),
        tool(
            "cover",
            "The board of ideas for the cover of a publication - a clip, an audio release, a \
             short - and everything an idea is built from. `cover` is the concept as it stands \
             (the idea, the scene, the hero, the built frame, the bricks of the style \
             dictionary by id, the accent, the mark); `ideas` are the ideas on the board, each \
             with where it came from (own, refined, ai, sibling), the person's verdict (star: \
             on the shortlist; rejected: not that; none), its angle, headline and concept; \
             `siblings` are the covers of the other publications of the same song, fitted to \
             this one's shape; `choices` is the text an idea chooses from - the layouts and \
             the five settings of the frame, the bricks with their ids and when to use them \
             (the channel's house styles marked), the palette, the variants of the mark with \
             what each means, the heroes of the canon, and what a picture of the channel never \
             shows. Read it before `propose_cover`.",
            json!({ "work": work_arg() }),
            &["work"],
        ),
        tool(
            "propose_cover",
            "Propose ideas for the cover of a publication - a clip, an audio release, a short. \
             Each idea is a concept the constructor shows: `idea` (what the cover says, in the \
             language of the work) and `scene` (what the picture shows, in English) are \
             required; `angle` (what sets it apart) and `headline` (what its card is called); \
             `avoid`, what this scene must keep out beyond the channel's bans, in English - \
             it goes with the scene into the negative; `hero`, a card id from the heroes; `layout` and optionally `column`, `row`, \
             `size`, `crop`, `place` to move its frame; `style`, `typography`, `dressing`, \
             `background`, brick ids; `accent`, a colour #RRGGBB or a name of the palette; \
             `mark`, a variant id whose meaning fits the song; `captions`, lines by slot of \
             the dressing. Read `cover` first for the ids: a name that is not in the \
             workspace is left out of its idea and said in the answer, the rest lands. \
             Vary the ideas by angle - the hero close or small, from below, in a vast space - \
             reach for the house styles, never propose what the channel bans, and do not \
             repeat the ideas on the board: a rejected one shows what is not wanted. The \
             ideas wait in the chat on the work and above its board; one click puts them on \
             the board, where the person stars them, turns them down or takes one into the \
             constructor.",
            json!({
                "work": work_arg(),
                "ideas": {
                    "type": "array",
                    "description": "The ideas, each a concept",
                    "items": { "type": "object", "properties": {
                        "angle": text_arg("What sets it apart, a few words"),
                        "headline": text_arg("What its card is called"),
                        "idea": text_arg("What the cover says, in the language of the work"),
                        "scene": text_arg("What the picture shows, in English, for a generator"),
                        "avoid": text_arg("What this scene must keep out beyond the channel's bans, in English, or leave it out"),
                        "hero": text_arg("A card id from the heroes of `cover`, or leave it out"),
                        "layout": text_arg("One of the layouts of `cover`"),
                        "column": text_arg("left, centre or right"),
                        "row": text_arg("top, middle or bottom"),
                        "size": text_arg("small, emblem, large, full, over or macro"),
                        "crop": text_arg("full, waist, bust, head or detail"),
                        "place": text_arg("Where the title goes: left, right, top, bottom, behind, overlap, vertical, corner or poster"),
                        "style": text_arg("An image style brick id"),
                        "typography": text_arg("A lettering brick id"),
                        "dressing": text_arg("A dressing brick id"),
                        "background": text_arg("A ground brick id"),
                        "accent": text_arg("A colour #RRGGBB, or a name of the channel's palette"),
                        "mark": text_arg("A variant id of the channel's mark"),
                        "captions": { "type": "object", "description": "Lines by slot of the dressing", "additionalProperties": { "type": "array", "items": { "type": "string" } } },
                    }, "required": ["idea", "scene"] },
                },
                "note": text_arg("One sentence on the choices made, optional"),
            }),
            &["work", "ideas"],
        ),
        tool(
            "brief",
            "The order a writer works from, for a work prepared on a conception - a song, a \
             chapter, an episode (`conception` of its kind in `workspace`). `order` is the whole \
             order in markdown, the same text the person copies out of kilna: the idea, the \
             brief in the author's words, the texts it keeps to (a song's style prompt), the \
             fields, each board as its rule reads it - the scenario to write by, those to \
             choose from or combine, or every one not turned down - the notes and phrases \
             attached with how each is sung, the channel's voice and the canon the work is \
             about, and the register of what is worn out. `order` is null while nothing of the \
             conception is written. `boards` are the boards themselves: each card with its id, \
             kind, topic, where it came from (own, refined, ai), the person's verdict (star: \
             wanted; rejected: not that; none), the entry of the library it links to and its \
             fields; the directions the last run offered; and `elsewhere`, the other works each \
             entry already worked in. In `order` a quote stands word for word only with where \
             it was checked. Write from `order`, and answer with `propose_version` - the text \
             in the role it is written in - or `propose_work` given `work`.",
            json!({ "work": work_arg() }),
            &["work"],
        ),
        tool(
            "propose_cards",
            "Propose cards onto a board of a work's conception - the research it draws on, the \
             scenarios to write it by, and whatever else the craft lays out (`boards` in \
             `workspace`, and the boards a kind's `conception` names). Each card names its \
             `kind`, which may be left out on a board of one kind, its `fields` by the keys of \
             that kind, and optionally a `topic` to group it under - one of the board's topics, \
             or a new one of a few words. A kind marked `library` names its work of culture \
             under `entry` - `title`, `author`, `year`, `medium` (a key of the library's \
             `media` in `workspace`), `original` - and links to the entry of the library of \
             that name, written into it when there is none. A field of the shape `quote` is \
             somebody else's words, word for word; `proof` is the link where they were read, \
             and a quote without one reaches the writer only as its sense; a work of a medium \
             that is not `quoted` (a song) gives no quote at all; `chips` are short things to \
             keep, parted by semicolons. `starred: true` carries over a choice the person \
             already made - research they took from before. Give two to four `directions`: \
             where the next run could dig, a few words each, shown on the board to click. Read \
             `brief` first: do not repeat what stands on the board, take the starred cards as \
             the direction and keep away from the turned-down ones. A kind or a field the board \
             does not have is left out and said in the answer, the rest lands. The cards wait \
             in the chat on the work and above its board; one click puts them on the board.",
            json!({
                "work": work_arg(),
                "board": text_arg("The board, by its key: `research`, `scenarios`"),
                "cards": {
                    "type": "array",
                    "description": "The cards, each in the fields of a kind of the board",
                    "items": { "type": "object", "properties": {
                        "kind": text_arg("A kind of card of the board; may be left out on a board of one kind"),
                        "topic": text_arg("What it is grouped under, a few words, or leave it out"),
                        "entry": {
                            "type": "object",
                            "description": "For a kind marked `library`: the work of culture the card is about",
                            "properties": {
                                "title": text_arg("Its title as it is known in the language of the work being written"),
                                "author": text_arg("Who made it, optional"),
                                "year": text_arg("When it was made, as said: 1913, c. 1308, 1880-83; optional"),
                                "medium": text_arg("A key of the library's media, optional"),
                                "original": text_arg("Its title in the language it was made in, when that differs; optional"),
                            },
                            "required": ["title"],
                        },
                        "fields": { "type": "object", "description": "What the card says, by the field keys of its kind", "additionalProperties": { "type": "string" } },
                        "starred": { "type": "boolean", "description": "The person already chose it; optional" },
                    } },
                },
                "directions": { "type": "array", "description": "Two to four places the next run could dig, a few words each", "items": { "type": "string" } },
                "note": text_arg("One sentence on the choices made, optional"),
            }),
            &["work", "board", "cards"],
        ),
        tool(
            "propose_version",
            "Propose a new version of a work in a role — a rewritten lyric, a style prompt. The \
             text lands in a chat on the work with an *insert as version* button; the person \
             decides. Nothing is written to the work until they do. Say in `note` what you \
             changed and why, briefly: it is shown beside the text. For a role a composition \
             writes (`compose` in `workspace`), give `bricks` instead of `body`: the phrases \
             picked from the dictionary, by id or exact name, in the order they should be \
             written - kilna writes the text from them as its own constructor does, the work's \
             fields closing it.",
            json!({
                "work": work_arg(),
                "role": text_arg("The version role key, as `workspace` lists them"),
                "body": text_arg("The whole text of the proposed version, exactly as it should be stored; leave it out when `bricks` are given"),
                "bricks": { "type": "array", "items": { "type": "string" }, "description": "For a role a composition writes: the bricks the text is written from, by id or exact name, in order" },
                "label": text_arg("A short name for the version, optional"),
                "from": text_arg("The id of the version in the same role this text rewrites, as `work` lists them, optional; it becomes the new version's parent, so the history shows what it came from"),
                "note": text_arg("One or two sentences on what changed and why, optional"),
            }),
            &["work", "role"],
        ),
        tool(
            "propose_score",
            "Propose a score along the profile's axes. Use the axis keys and scales from \
             `workspace`; every axis, no others. The proposal is shown with the numbers and an \
             apply button; the person decides. Nothing is written until they do.",
            json!({
                "work": work_arg(),
                "axes": { "type": "object", "description": "Axis key to mark, within the axis scale", "additionalProperties": { "type": "number" } },
                "note": text_arg("One sentence on why, optional"),
            }),
            &["work", "axes"],
        ),
        tool(
            "propose_note",
            "Propose a note — on a work, or on nothing in particular. Lands in the chat with an \
             *add as note* button; the person decides. `kind` is the profile's kind of note (see \
             `workspace`: `note_kinds`): a phrase - one line, a quote with `attribution` - or a \
             note, the plain note when left out. A phrase may name the `shelf` it goes on, \
             made when there is none. A note can carry `tags` and a `layer` (who may read it). \
             `note` - the id or the exact title of a note that is there - proposes a CHANGE to \
             it rather than a second note: give only what changes (a new `body`, `title`, \
             `tags`), and the person reads was against becomes before agreeing. A change may \
             also move the note - `to_work` names the work it goes to, `off_work: true` takes \
             it off its work - and make it another `kind`; a card of the canon is neither \
             moved nor made another kind (`propose_canon` changes it). `notes` \
             proposes several at once, each an object of these same fields: they wait as one \
             proposal, kept whole or note by note.",
            json!({
                "work": work_arg(),
                "kind": text_arg("The kind of note, as `workspace` lists it; optional"),
                "title": text_arg("A title for the note, optional; a phrase has none"),
                "body": text_arg("The note itself"),
                "attribution": text_arg("Whose words a phrase is - the author, the film; optional"),
                "shelf": text_arg("The shelf a phrase goes on, by name; optional"),
                "origin_work": text_arg("The work a new note or phrase arose at, by id or exact title - it stays fresh and remembers it; optional"),
                "tags": { "type": "array", "items": { "type": "string" }, "description": "The note's tags; for a change, the tags it will have" },
                "layer": text_arg("Who may read it: public, internal or inWorks; optional"),
                "note": text_arg("A note that is there, by id or exact title: propose a change to it"),
                "to_work": text_arg("With `note`: the work the note is moved to, by id or exact title"),
                "off_work": { "type": "boolean", "description": "With `note`: take the note off its work, to stand on none" },
                "notes": {
                    "type": "array",
                    "description": "Several notes in one proposal: [{body, title?, kind?, attribution?, shelf?, tags?, layer?, note?, to_work?, off_work?}]",
                    "items": {}
                },
            }),
            &[],
        ),
        tool(
            "canon",
            "The canon: the world the works share, kept as facts on cards — people, places, \
             objects, groups, events, themes, and one root card for the channel. Without \
             arguments: every card (id, kind, name, aliases, layer, the work it lives at, how \
             many facts, drafts and live zones) and the kinds of card with their sections. With \
             `card`: that card whole — its facts by section, each with its layer (`public`, \
             `internal`: take the detail never the name, date or address; `inWorks`: only inside \
             the works), its status, source and time in the world; its relations; its \
             pictures; where it appears; its description for a picture generator and whether \
             it is stale. `lens` narrows the card to what a task may read: `cover`, `work` or \
             `public`. `query` searches the facts; `timeline` lists the facts dated in the world, \
             earliest first. Without arguments it also says whether the space has its passport \
             (`has_root_card`, `root_card_id`) and `section_schemas`: what an entry of each \
             shape of section carries beside its words. A long card is read in parts: \
             `summary: true` gives its head, its summary and how many facts each section \
             holds; `section` and `status` narrow the facts; `limit` and `offset` page them \
             (`total_facts`, `has_more`). `card: \"root\"` is the passport. Without `card` \
             the cards, the facts a `query` finds and the `timeline` come a page at a time \
             too (`total`, `limit`, `offset`, `has_more`), and `summary: true` gives the \
             cards by name alone - id, kind, title, layer - without the kinds and schemas.",
            json!({
                "card": text_arg("A card: its id, its exact name or alias, or root for the passport"),
                "summary": { "type": "boolean", "description": "With `card`: its head, summary and counts by section, no facts. Without: the cards by name alone" },
                "section": text_arg("With `card`: only the facts of this section key"),
                "status": text_arg("With `card`: only facts in this state - canon, draft, open or retired"),
                "limit": { "type": "integer", "description": "At most this many: facts of `card`, or cards, found facts or dated facts without it (200 cards by default)" },
                "offset": { "type": "integer", "description": "Skip this many first" },
                "kind": text_arg("Only cards of this kind, as the list names them"),
                "lens": { "type": "string", "enum": ["cover", "work", "public"], "description": "Read the card as this task may: cover, work or public" },
                "query": text_arg("Search the facts for these words"),
                "timeline": { "type": "boolean", "description": "The facts dated in the world, earliest first; of `card` when given" },
            }),
            &[],
        ),
        tool(
            "propose_canon",
            "Propose cards, facts and relations for the canon, applied with one click or item by \
             item. Read `canon` first: a fact the canon holds is not proposed again, a sharper \
             wording is a `refine` of its id, a fact no longer true is a `retire` with a reason, \
             and a fact that contradicts the canon says so in `contradicts` — the person sees \
             both before applying. Name cards by id, exact name, or the `handle` of a card \
             proposed in the same call. `work` is the work the facts were read from: each fact \
             with no other source cites it (quote the line in `line`), and a card with \
             `on_work: true` lives at it. Sections are the kind's keys (see `canon`); an item \
             the canon has no place for is named and left out. `pictures` attach files on this \
             machine to a card - or to one fact of it, an outfit - with a role; `descriptions` \
             write what a generator is told about a card, in English, from the facts it \
             holds (read the card first). `card_updates` rename a card, give it other names, a \
             layer or a summary (`note`) - `card: \"root\"` is the passport, and a second \
             passport proposed in `cards` updates the one standing. A fact can also be `move`d \
             to another section or `delete`d, and a `refine` with a `section` files it \
             elsewhere; a new fact the card nearly holds already comes back marked \
             `similar_to`. A relation with `action: \"delete\"` is taken away. Nothing is \
             written until the person applies it.",
            json!({
                "work": text_arg("The work the facts were read from: its id or exact title, optional"),
                "cards": {
                    "type": "array",
                    "description": "New cards",
                    "items": { "type": "object", "properties": {
                        "handle": text_arg("How facts and relations of this call name it: new-1"),
                        "kind": text_arg("A kind of card"),
                        "title": text_arg("Its name"),
                        "aliases": { "type": "array", "items": { "type": "string" }, "description": "Other forms of the name the texts use" },
                        "layer": text_arg("public, internal or inWorks: who may know the card exists"),
                        "on_work": { "type": "boolean", "description": "It lives only in `work` — a hero of that one work" },
                        "note": text_arg("Its free note, optional"),
                    }, "required": ["kind", "title"] },
                },
                "facts": {
                    "type": "array",
                    "description": "Facts to add, refine or retire",
                    "items": { "type": "object", "properties": {
                        "change": { "type": "string", "enum": ["add", "refine", "retire", "move", "delete"] },
                        "card": text_arg("For `add`: a card id, name, handle, or root"),
                        "fact": text_arg("For `refine`, `retire`, `move` and `delete`: the fact's id"),
                        "section": text_arg("For `add` and `move`: a section key of the card's kind; for `refine`, where it is filed now"),
                        "text": text_arg("The fact, one short statement"),
                        "layer": text_arg("public, internal or inWorks"),
                        "status": text_arg("open for a live zone, left open on purpose; anything else is kept as a draft until the person settles it"),
                        "line": text_arg("The line of `work` it was read off, quoted"),
                        "document": text_arg("Or: the document it comes from, as named"),
                        "decision": text_arg("Or: the decision it comes from, as named"),
                        "when": text_arg("When it happened in the world, in words"),
                        "sort": text_arg("Where it sorts: YYYY, YYYY-MM or YYYY-MM-DD"),
                        "reason": text_arg("For `retire`: why it is no longer true"),
                        "data": { "type": "object", "description": data_keys, "additionalProperties": true },
                        "contradicts": { "type": "array", "items": { "type": "object", "properties": {
                            "fact": text_arg("The id of a fact of the canon"),
                            "why": text_arg("One line"),
                        } } },
                    } },
                },
                "relations": {
                    "type": "array",
                    "description": "Relations between cards; a pair already related is redrawn",
                    "items": { "type": "object", "properties": {
                        "from": text_arg("A card id, name or handle"),
                        "to": text_arg("A card id, name or handle"),
                        "kind": text_arg("A kind of relation"),
                        "label": text_arg("What `to` is to `from`"),
                        "back_label": text_arg("What `from` is to `to`"),
                        "layer": text_arg("public, internal or inWorks"),
                        "action": { "type": "string", "enum": ["upsert", "delete"], "description": "delete takes the relation between the two away; upsert (the default) draws or redraws it" },
                    }, "required": ["from", "to"] },
                },
                "card_updates": {
                    "type": "array",
                    "description": "Cards of the canon changed",
                    "items": { "type": "object", "properties": {
                        "card": text_arg("A card id, name or alias, or root for the passport"),
                        "title": text_arg("Its new name"),
                        "aliases": { "type": "array", "items": { "type": "string" }, "description": "Its other names, the whole list" },
                        "layer": text_arg("public, internal or inWorks"),
                        "note": text_arg("Its summary, the card's free note, whole"),
                        "on_work": { "type": "boolean", "description": "true: it lives at `work`; false: it lives at no work" },
                    }, "required": ["card"] },
                },
                "pictures": {
                    "type": "array",
                    "description": "Pictures to attach, copied into the workspace when kept",
                    "items": { "type": "object", "properties": {
                        "path": text_arg("The file: an absolute path on this machine, JPEG, PNG, WebP, GIF or AVIF"),
                        "card": text_arg("The card it shows: an id, name or handle"),
                        "fact": text_arg("Or: the id of the fact it shows - an outfit; the card is the fact's"),
                        "role": text_arg("portrait, reference, outfit, mood, still or mark; reference when absent. Portraits and references go to a generator with the card"),
                    }, "required": ["path"] },
                },
                "descriptions": {
                    "type": "array",
                    "description": "What a generator is told about a card, replacing what it is told now",
                    "items": { "type": "object", "properties": {
                        "card": text_arg("A card id or name"),
                        "text": text_arg("The description, in English, from the card's appearance facts"),
                    }, "required": ["card", "text"] },
                },
            }),
            &[],
        ),
    ]
}

/// What an entry of a section of `shape` carries beside its words, as an
/// agent writes it in `propose_canon` (v0.144): the `data` keys the shape
/// reads, each said in a line.
pub(super) fn section_schema(shape: SectionShape) -> Value {
    let (text, data): (&str, Vec<(&str, &str)>) = match shape {
        SectionShape::Facts => ("one short statement", vec![]),
        SectionShape::Slots => ("the words of the slot", vec![("slot", "the slot's name")]),
        SectionShape::Details => (
            "the detail's name",
            vec![
                ("template", "how it is said on a cover or a frame"),
                ("places", "where it goes: a list of cover, frame"),
                ("on", "true when it is on by default"),
            ],
        ),
        SectionShape::Palette => ("what the colour is for", vec![("color", "#RRGGBB")]),
        SectionShape::Marks => (
            "the variant of the mark and what it means",
            vec![
                ("code", "its short code"),
                ("prompt", "how a generator is told to draw it"),
            ],
        ),
        SectionShape::Styles => (
            "optional: the style's name",
            vec![("styleId", "the id of a brick of the style dictionary")],
        ),
        SectionShape::Bans => ("what is never done or said", vec![]),
        SectionShape::Branding => (
            "the kit's name",
            vec![
                ("kind", "avatar, banner or watermark"),
                ("prompt", "how a generator is told to draw it, in English"),
                ("negative", "what the picture must not show, in English"),
                ("about", "the idea of it, in a line or two"),
                ("archived", "true for a kit of the look before a rebranding"),
                (
                    "active",
                    "the id of the picture in use, among the kit's own",
                ),
            ],
        ),
        SectionShape::Quotas => (
            "the rule's name, as a person reads it",
            vec![
                (
                    "field",
                    "what is measured: title, tags, cover, a text role (suno_prompt, lyrics), an overview field (genre), a release field (playlists, or youtube.title)",
                ),
                (
                    "metric",
                    "char_len, word_count, count_per_value, unique_count, match_count, share or forbidden_terms",
                ),
                (
                    "op",
                    "between, min, max or eq; not_contains for forbidden_terms",
                ),
                (
                    "value",
                    "a number, [low, high] for between, a percent for share, a list of words for forbidden_terms",
                ),
                (
                    "pattern",
                    "a regular expression, for match_count and optionally share",
                ),
                (
                    "filter",
                    "optional: {field, equals: [values], not?: true} - only the works whose field says one of them",
                ),
                (
                    "scope",
                    "item (each work), collection_period (a month's or a collection's works) or global",
                ),
                ("severity", "warn or error"),
                ("active", "false to keep the rule without checking it"),
            ],
        ),
        SectionShape::Bank => (
            "the item's name",
            vec![
                ("group", "the region, continent or rubric it belongs to"),
                ("attributes", "a list: instruments, devices, character"),
                ("cooldown_months", "how many months it rests after a use"),
                ("last_used_at", "YYYY-MM-DD of the last use"),
                ("used_in", "a list of {work_id, title, date}"),
                ("locked", "true for an item out of the pool for good"),
                ("note", "anything else"),
            ],
        ),
        SectionShape::Relations | SectionShape::Appearances => ("", vec![]),
    };
    debug_assert!(
        data.iter()
            .all(|(key, _)| crate::canon::fact::data_keys(shape).contains(key)),
        "a schema names a key the shape does not read"
    );
    json!({
        "text": text,
        "data": data.into_iter().map(|(key, said)| (key.to_owned(), json!(said))).collect::<Map<String, Value>>(),
    })
}
