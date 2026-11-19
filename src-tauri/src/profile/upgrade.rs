//! Carrying a shipped profile forward into the copy a space holds, and
//! bringing a stored document to the current format.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;

use crate::error::Result;
use crate::time::now;

use super::config::{self, Derive, ProfileConfig};
use super::*;

/// Bring every stored profile document to [`config::FORMAT`].
///
/// Reading is the migration (see `RawProfileConfig`); this makes it stick.
/// A document already in the current format is left untouched, byte for
/// byte, so an edit the owner made by hand is not reserialised behind them.
pub(super) fn rewrite_older_formats(conn: &Connection) -> Result<()> {
    let rows: Vec<(String, String)> = conn
        .prepare("SELECT id, config FROM profile")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, raw) in rows {
        if raw_format(&raw) == config::FORMAT {
            continue;
        }
        let config: ProfileConfig = serde_json::from_str(&raw)?;
        conn.execute(
            "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, serde_json::to_string(&config)?, now()],
        )?;
    }
    Ok(())
}

/// Carry newly shipped parts of a built-in profile into the copy a workspace
/// already holds.
///
/// A profile is copied into the database on first run and then belongs to the
/// user, so shipping a new field changes nothing for anybody who already ran
/// the app — the older copy simply lacks it. What is added is what has no
/// user-authored value to overwrite, one [`Step`] per part, in [`UPGRADE`]'s
/// order: a missing prompt, a status's derive role where the stored one still
/// says `Manual`, a release kind's requirements where it states none, and so
/// on - each step says what it carries and why.
///
/// A role the user deliberately set to `Manual` is indistinguishable from one
/// that was never written, and is restored along with the rest. The same is
/// true of a deleted prompt or meta field: absence is absence either way. That
/// is the price of not asking; both are one click away in the profile editor.
pub(super) fn carry_forward(conn: &Connection, shipped: &BuiltinProfile) -> Result<()> {
    let Some((id, raw)): Option<(String, String)> = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = ?1",
            params![shipped.key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
    else {
        return Ok(());
    };

    let mut config: ProfileConfig = serde_json::from_str(&raw)?;
    let mut changed = false;
    // The entries kilna goes on shipping, one by one (ADR 0087), before the
    // steps: some of them read what arrives here - a board laid out in a
    // conception only where the action it runs is there. An older wording a
    // step then brings up to what ships is met as untouched at the next start.
    let mut ledger = super::shipped::Ledger::load(conn, &id)?;
    let seen = super::shipped::carry(&mut config, &shipped.config, &mut ledger)?;
    let count =
        |verdict: crate::origin::Verdict| seen.iter().filter(|one| one.verdict == verdict).count();
    let (renewed, added) = (
        count(crate::origin::Verdict::Renewed),
        count(crate::origin::Verdict::Added),
    );
    if renewed + added > 0 {
        crate::log::info(
            "profile",
            &format!(
                "`{}`: {renewed} entries renewed, {added} added",
                shipped.key
            ),
        );
        changed = true;
    }
    ledger.save(conn, &id, &super::shipped::origin_of(&shipped.key))?;
    for step in UPGRADE {
        if (step.carry)(&mut config, &shipped.config) {
            // Said, because an upgrade that changed a person's profile at
            // start is the first thing to look for when a word moved.
            crate::log::info(
                "profile",
                &format!("carried {} into `{}`", step.name, shipped.key),
            );
            changed = true;
        }
    }

    // A stored copy still in format 1 was read into format 2 by the parser;
    // writing it back is what makes the migration permanent rather than
    // something every read repeats.
    if raw_format(&raw) != config::FORMAT {
        changed = true;
    }

    // The shipped profile's name follows it only where the stored copy still
    // carries the name it shipped with before: "Music" became "Studio" in
    // v0.57, because the craft makes videos too. A name the owner chose
    // themselves is theirs, so the match is against the old shipped name,
    // never against "anything different".
    for (before, after) in RENAMED {
        if shipped.name == after {
            let renamed = conn.execute(
                "UPDATE profile SET name = ?2, updated_at = ?3 WHERE id = ?1 AND name = ?4",
                params![id, after, now(), before],
            )?;
            if renamed > 0 {
                changed = true;
            }
        }
    }
    // The one-sentence description follows on the same terms: only where the
    // stored copy still says exactly what the old shipped file said.
    for (before, after) in REDESCRIBED {
        if shipped.description == after {
            conn.execute(
                "UPDATE profile SET description = ?2, updated_at = ?3 WHERE id = ?1 AND description = ?4",
                params![id, after, now(), before],
            )?;
        }
    }

    if !changed {
        return Ok(());
    }

    conn.execute(
        "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, serde_json::to_string(&config)?, now()],
    )?;

    Ok(())
}

/// One thing an upgrade carries from a shipped profile into the copy a
/// workspace holds, and whether it changed anything.
///
/// Every step is written on the same terms, the ones [`carry_forward`] states:
/// it fills what the stored copy leaves blank, matched by key, and never
/// overwrites a word the owner wrote. And every step is idempotent - run on
/// its own result it changes nothing - which the tests below hold each one to,
/// so a step can be added without reading the others.
pub(super) struct Step {
    /// What it carries, as the log and a failing test name it.
    pub(super) name: &'static str,
    pub(super) carry: fn(&mut ProfileConfig, &ProfileConfig) -> bool,
}

/// The upgrade, in the order it runs. A field newly shipped reaches no
/// workspace that already exists - which is every real one - unless a step
/// here carries it: add the step, and the tests hold it to the rules.
pub(super) const UPGRADE: &[Step] = &[
    Step {
        name: "a prompt's method, product, kinds and scope, and a score template untouched since v0.61",
        carry: prompt_details,
    },
    Step {
        name: "a status's derive role and colour",
        carry: status_details,
    },
    Step {
        name: "a release kind's requirements, glyph and fields",
        carry: release_kind_details,
    },
    Step {
        name: "what a place will not go out without - a cover, a field written - and the links it is known by",
        carry: going_out_needs,
    },
    Step {
        name: "version roles, and how a role's body reads",
        carry: version_roles,
    },
    Step {
        name: "the storyboard's and the cover's vocabulary",
        carry: board_vocabulary,
    },
    Step {
        name: "kinds of work the stored copy lacks, without their judgement",
        carry: arriving_kinds,
    },
    Step {
        name: "the release rhythm",
        carry: rhythm,
    },
    Step {
        name: "note kinds the line no longer ships",
        carry: retired_note_kinds,
    },
    Step {
        name: "the stops of the stage dial",
        carry: stages,
    },
    Step {
        name: "the lines a work's feel is said along",
        carry: vibe_axes,
    },
    Step {
        name: "tier words untouched since they shipped",
        carry: reword_untouched_tiers,
    },
    Step {
        name: "style types, with their hint, glyph, form, families and retirement",
        carry: style_types,
    },
    Step {
        name: "the compositions a role is written out of the dictionary by",
        carry: compositions,
    },
    Step {
        name: "a mark's glyph",
        carry: mark_icons,
    },
    Step {
        name: "the canon: a note kind's sections, root and glyph, a section's shape, lenses and hint, and the kinds of relation",
        carry: canon_vocabulary,
    },
    Step {
        name: "publications: a door's cover shape, a kind's cover, frame and the title of what is made from it, a field's kinds, options and start",
        carry: publication_vocabulary,
    },
    Step {
        name: "what a release goes out under: a title reading the origin where it still read the work's own, a field's tail and its premiere's, a kind that is an excerpt, a field of the work's own media",
        carry: going_out_words,
    },
    Step {
        name: "a door's fields shipped since it was stored, and a field's limit still as it shipped wrong",
        carry: arriving_fields,
    },
    Step {
        name: "an action's method still reading as an earlier release shipped it",
        carry: rewritten_methods,
    },
    Step {
        name: "the folder a kind keeps its files in on disk",
        carry: kind_folders,
    },
    // After the folders: a rule arrives only beside the folder it was
    // written for.
    Step {
        name: "the rule a kind finds its finished file by",
        carry: kind_files,
    },
    Step {
        name: "a kind's board of trials and the role its kept trials go into",
        carry: lab_boards,
    },
    // Before the boards: a board whose cards link to the library arrives
    // only in a copy that has one.
    Step {
        name: "the library a note kind is, and the media of its entries",
        carry: library_vocabulary,
    },
    Step {
        name: "the boards of a conception, and the conception of a kind",
        carry: conceptions,
    },
    Step {
        name: "boards newly laid out in a conception the stored kind already has",
        carry: conception_boards,
    },
    Step {
        name: "the scenarios' template untouched since v0.99, once the research is there",
        carry: research_templates,
    },
    Step {
        name: "a field's label untouched since it shipped",
        carry: relabel_untouched_fields,
    },
    Step {
        name: "the tab a kind opens on, where it was renamed",
        carry: renamed_tabs,
    },
    Step {
        name: "a field's type still as it shipped before: the anchors, a text until they became a list",
        carry: retyped_fields,
    },
    Step {
        name: "a field's kinds and a kind's roles still as they shipped before a card lost its leftovers",
        carry: card_leftovers,
    },
    Step {
        name: "the part of the register a shipped field is read in",
        carry: register_fields,
    },
    // Last: every word the steps above brought in is already there to be
    // read against its shipped self.
    Step {
        name: "the second language of a word still reading as it shipped in English",
        carry: second_language,
    },
];

/// An action's method (ADR 0021) arrives on the terms a role's body did: where
/// the stored action names none and the shipped one does. What the action
/// produces follows only where the stored copy says nothing, and so do its
/// kinds and scope (v0.64) - an action stored before they existed is for every
/// kind, which is what `critique` on a video was until the shipped copy said
/// `song`. A `score` template still reading exactly as it shipped before v0.61
/// - naming the axes by hand - moves to the shipped wording, because the
///   instruction now states the axes from the profile; a reworded one stays.
pub(super) fn prompt_details(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for prompt in &mut config.prompts {
        let Some(shipped) = shipped.prompts.iter().find(|s| s.key == prompt.key) else {
            continue;
        };
        if prompt.method().is_none() && shipped.method().is_some() {
            prompt.method = shipped.method.clone();
            changed = true;
        }
        if prompt.produces.is_none() && shipped.produces.is_some() {
            prompt.produces = shipped.produces.clone();
            changed = true;
        }
        if prompt.kinds.is_empty() && !shipped.kinds.is_empty() {
            prompt.kinds = shipped.kinds.clone();
            changed = true;
        }
        if prompt.scope.is_none() && shipped.scope.is_some() {
            prompt.scope = shipped.scope.clone();
            changed = true;
        }
        if SCORE_TEMPLATES_BEFORE_METHODS.contains(&prompt.template.as_str())
            && prompt.template != shipped.template
        {
            prompt.template = shipped.template.clone();
            changed = true;
        }
        if TEMPLATES_BEFORE_THE_REGISTER
            .iter()
            .any(|(key, before)| *key == prompt.key && *before == prompt.template)
            && prompt.template != shipped.template
        {
            prompt.template = shipped.template.clone();
            changed = true;
        }
        if TEMPLATES_BEFORE_THE_CONTEXT
            .iter()
            .any(|(key, before)| *key == prompt.key && *before == prompt.template)
            && prompt.template != shipped.template
        {
            prompt.template = shipped.template.clone();
            changed = true;
        }
        // Whether the action reads the web (v0.100), where the stored action
        // says it does not and the shipped one does.
        if !prompt.web && shipped.web {
            prompt.web = true;
            changed = true;
        }
    }
    changed
}

/// The stored kind and the shipped kind, met by key. A kind the owner
/// invented meets nothing and keeps what it has.
pub(super) fn shared_kinds<'a>(
    config: &'a mut ProfileConfig,
    shipped: &'a ProfileConfig,
) -> impl Iterator<Item = (&'a mut config::WorkKind, &'a config::WorkKind)> {
    config.work_kinds.iter_mut().filter_map(|kind| {
        let shipped_kind = shipped.work_kinds.iter().find(|s| s.key == kind.key)?;
        Some((kind, shipped_kind))
    })
}

/// The `derive` role of a status, where the stored copy still says `Manual`
/// and the shipped one names a meaning - without it the status automation is
/// silently inert in every workspace that predates it, which is how it was
/// found. And a badge's colour, where the stored status names none.
pub(super) fn status_details(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        for status in &mut kind.statuses {
            let Some(shipped) = shipped_kind.statuses.iter().find(|s| s.key == status.key) else {
                continue;
            };
            if status.derive == Derive::Manual && shipped.derive != Derive::Manual {
                status.derive = shipped.derive;
                changed = true;
            }
            if status.colour.is_none() && shipped.colour.is_some() {
                status.colour = shipped.colour;
                changed = true;
            }
        }
    }
    changed
}

/// A release kind's `requires` list and glyph, where the stored copy states
/// none: readiness marks and calendar chips would sit blank in every workspace
/// that predates them. Its fields arrive whole or not at all - a workspace that
/// already lists fields has an owner who decided what a release says, and
/// appending the shipped ones would put two titles in one box.
pub(super) fn release_kind_details(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        for release_kind in &mut kind.release_kinds {
            let Some(shipped) = shipped_kind
                .release_kinds
                .iter()
                .find(|s| s.key == release_kind.key)
            else {
                continue;
            };
            if release_kind.requires.is_empty() && !shipped.requires.is_empty() {
                release_kind.requires = shipped.requires.clone();
                changed = true;
            }
            if release_kind.icon.is_none() && shipped.icon.is_some() {
                release_kind.icon = shipped.icon.clone();
                changed = true;
            }
            if release_kind.fields.is_empty() && !shipped.fields.is_empty() {
                release_kind.fields = shipped.fields.clone();
                changed = true;
            }
        }
    }
    changed
}

/// What a door will not let a release out without, and what its addresses
/// look like (v0.97), where the stored door says nothing: a cover the shipped
/// door requires, a field of the same key the shipped door requires, the
/// patterns of its links. A requirement the owner took off is written as
/// `false` and stays off - unlike a role's `sung`, an absent flag here is
/// told apart from a cleared one, because the checklist is toggled in the
/// settings and a box that ticks itself back at every start is a broken box.
pub(super) fn going_out_needs(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        for door in &mut kind.release_kinds {
            let Some(shipped) = shipped_kind
                .release_kinds
                .iter()
                .find(|s| s.key == door.key)
            else {
                continue;
            };
            if door.requires_cover.is_none() && shipped.requires_cover.is_some() {
                door.requires_cover = shipped.requires_cover;
                changed = true;
            }
            if door.links.is_empty() && !shipped.links.is_empty() {
                door.links = shipped.links.clone();
                changed = true;
            }
            for field in &mut door.fields {
                let Some(shipped_field) = shipped.fields.iter().find(|s| s.key == field.key) else {
                    continue;
                };
                if field.required.is_none() && shipped_field.required.is_some() {
                    field.required = shipped_field.required;
                    changed = true;
                }
            }
        }
    }
    changed
}

/// A role newly shipped for a kind the workspace has is appended after the
/// owner's; a role they renamed or retyped stays theirs, because the match is
/// by key. How a role's body reads arrives where the stored copy states
/// nothing.
pub(super) fn version_roles(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        changed |= insert_new_keys(
            &mut kind.version_roles,
            &shipped_kind.version_roles,
            |role| &role.key,
        );
        for role in &mut kind.version_roles {
            let Some(shipped) = shipped_kind
                .version_roles
                .iter()
                .find(|s| s.key == role.key)
            else {
                continue;
            };
            if role.body.is_none() && shipped.body.is_some() {
                role.body = shipped.body.clone();
                changed = true;
            }
            // A lyric is sung (v0.90): the stored role learns it where it
            // says nothing, and a role the owner marked keeps the mark.
            if !role.sung && shipped.sung {
                role.sung = true;
                changed = true;
            }
            // What a version from nothing starts from (v0.91), where the
            // stored role names no skeleton of its own.
            if role.skeleton.is_none() && shipped.skeleton.is_some() {
                role.skeleton = shipped.skeleton.clone();
                changed = true;
            }
            // Whether the role is a draft of the work, where the stored role
            // does not say (v0.99). A copy made before the field existed
            // counted a song's style prompt as a version of the song, and
            // since a version that is no draft is never the work's current
            // text, saying nothing let a style prompt become the song.
            if role.counts_as_version.is_none() && shipped.counts_as_version.is_some() {
                role.counts_as_version = shipped.counts_as_version;
                changed = true;
            }
        }
    }
    changed
}

/// The storyboard's kinds of shot, layers and prompt blocks, where the stored
/// kind names none: the video kind reached the owner's workspace before scenes
/// existed, and without this it would have no Scenes tab. A list the owner
/// narrowed or renamed is left alone. Which block says what the picture
/// shows (v0.88) follows the shipped block of the same key, where no stored
/// block says it yet - the mark did not exist before.
pub(super) fn board_vocabulary(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        if kind.shot_types.is_empty() && !shipped_kind.shot_types.is_empty() {
            kind.shot_types = shipped_kind.shot_types.clone();
            changed = true;
        }
        if kind.scene_blocks.is_empty() && !shipped_kind.scene_blocks.is_empty() {
            kind.scene_blocks = shipped_kind.scene_blocks.clone();
            changed = true;
        }
        // The layers (v0.112) arrive only beside a storyboard: a kind whose
        // owner took the board off has no scene for a layer to be of.
        if kind.layers.is_empty()
            && !shipped_kind.layers.is_empty()
            && (!kind.shot_types.is_empty() || !kind.scene_blocks.is_empty())
        {
            kind.layers = shipped_kind.layers.clone();
            changed = true;
        }
        if !kind.scene_blocks.iter().any(|block| block.picture)
            && let Some(shipped_block) = shipped_kind.scene_blocks.iter().find(|b| b.picture)
            && let Some(block) = kind
                .scene_blocks
                .iter_mut()
                .find(|block| block.key == shipped_block.key)
        {
            block.picture = true;
            changed = true;
        }
    }
    changed
}

/// A kind the shipped profile gained since this workspace was made - a video
/// beside the songs - arrives with its statuses, roles and kinds of release,
/// and WITHOUT its axes and tiers. The judgement is the craft's own: this
/// workspace's axes are the owner's words, and a stranger's axes appearing
/// silently beside them would be the one thing the profile must never do. A
/// fresh workspace, seeded rather than carried forward, gets the whole kind.
pub(super) fn arriving_kinds(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for (index, candidate) in shipped.work_kinds.iter().enumerate() {
        if config
            .work_kinds
            .iter()
            .any(|kind| kind.key == candidate.key)
        {
            continue;
        }
        // Where the shipped list puts it: after the kind it follows there,
        // when the workspace has that one - the audio release between the
        // clip and the short, as the menu that makes them lists them -
        // and at the end otherwise.
        let after = shipped.work_kinds[..index].iter().rev().find_map(|before| {
            config
                .work_kinds
                .iter()
                .position(|kind| kind.key == before.key)
        });
        let at = after.map_or(config.work_kinds.len(), |position| position + 1);
        config.work_kinds.insert(at, candidate.without_judgement());
        changed = true;
    }
    changed
}

/// The rhythm, where the stored copy has none: without it the auto-layout is
/// a button that only ever refuses.
pub(super) fn rhythm(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    if config.rhythm.is_none() && shipped.rhythm.is_some() {
        config.rhythm = shipped.rhythm.clone();
        return true;
    }
    false
}

/// The folder a kind keeps its files in on disk (v0.93, ADR 0057), where the
/// stored kind names none. A template changes nothing until the workspace has
/// a media folder on this machine, so a workspace that never sets one never
/// sees it; a template the owner rewrote stays theirs.
pub(super) fn kind_folders(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for kind in &mut config.work_kinds {
        if kind.folder.is_some() {
            continue;
        }
        let Some(folder) = shipped
            .work_kinds
            .iter()
            .find(|candidate| candidate.key == kind.key)
            .and_then(|candidate| candidate.folder.clone())
        else {
            continue;
        };
        kind.folder = Some(folder);
        changed = true;
    }
    changed
}

/// The rule a kind finds its finished file by (v0.101, ADR 0070), where the
/// stored kind names none - and only while the kind keeps its files where
/// the shipped profile does: a rule is a path inside the folder its kind's
/// template names, and under a folder the owner laid out themselves it would
/// point at nothing, or at the wrong file.
pub(super) fn kind_files(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for kind in &mut config.work_kinds {
        if kind.file.is_some() {
            continue;
        }
        let Some(candidate) = shipped
            .work_kinds
            .iter()
            .find(|candidate| candidate.key == kind.key)
        else {
            continue;
        };
        if candidate.file.is_none() || candidate.folder != kind.folder {
            continue;
        }
        kind.file = candidate.file.clone();
        changed = true;
    }
    changed
}

/// A kind's board of trials (v0.95, ADR 0061), where the stored kind has
/// none: an experiment kind the owner made before the lab existed becomes a
/// lab by the shipped kind of the same key. The role its kept trials go into
/// arrives where the stored lab names none. A lab the owner took off a kind
/// cannot be told from one never given, and comes back - the price
/// `carry_forward` states. So does the field its anchors are read from.
pub(super) fn lab_boards(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        let Some(shipped_lab) = &shipped_kind.lab else {
            continue;
        };
        match &mut kind.lab {
            None => {
                kind.lab = Some(shipped_lab.clone());
                changed = true;
            }
            Some(lab) => {
                if lab.harvest.is_none() && shipped_lab.harvest.is_some() {
                    lab.harvest = shipped_lab.harvest.clone();
                    changed = true;
                }
                if lab.anchors.is_none() && shipped_lab.anchors.is_some() {
                    lab.anchors = shipped_lab.anchors.clone();
                    changed = true;
                }
            }
        }
    }
    changed
}

/// The boards a conception lays out, by key, and a kind's conception where
/// the stored kind has none (v0.99, ADR 0065).
///
/// A conception arrives only where it would hold in the stored copy: its
/// brief is a role of the kind that does not count as a version - the step
/// before carries that word onto a role that said nothing, and a role the
/// owner said counts keeps the kind without a conception rather than
/// making their copy refuse to load. What it sends with the order and the
/// boards it lays out are narrowed to what the stored copy has, and the
/// idea to a field it has. A conception the owner took off a kind comes
/// back, the price `carry_forward` states.
pub(super) fn conceptions(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    // A board whose cards link to the library needs one: a copy whose own
    // kind of the key could not become it goes without the board rather
    // than failing to load.
    let arriving: Vec<config::Board> = shipped
        .boards
        .iter()
        .filter(|board| !board.links_the_library() || config.library().is_some())
        .cloned()
        .collect();
    let mut changed = insert_new_keys(&mut config.boards, &arriving, |board| &board.key);
    let boards: Vec<String> = config.boards.iter().map(|b| b.key.clone()).collect();
    let fields: Vec<(String, Vec<String>)> = config
        .work_meta_fields
        .iter()
        .map(|field| (field.key.clone(), field.kinds.clone()))
        .collect();
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        let Some(shipped_conception) = &shipped_kind.conception else {
            continue;
        };
        if kind.conception.is_some() {
            continue;
        }
        let brief_holds = kind.version_roles.iter().any(|role| {
            role.key == shipped_conception.brief
                && role.comments_on.is_none()
                && !role.counts_as_a_version()
        });
        if !brief_holds {
            continue;
        }
        let has_role = |key: &String| kind.version_roles.iter().any(|role| role.key == *key);
        let idea = shipped_conception.idea.clone().filter(|key| {
            fields.iter().any(|(field, kinds)| {
                field == key && (kinds.is_empty() || kinds.contains(&kind.key))
            })
        });
        let with = shipped_conception
            .with
            .iter()
            .filter(|key| has_role(key))
            .cloned()
            .collect();
        kind.conception = Some(config::Conception {
            brief: shipped_conception.brief.clone(),
            idea,
            with,
            boards: shipped_conception
                .boards
                .iter()
                .filter(|key| boards.contains(key))
                .cloned()
                .collect(),
        });
        changed = true;
    }
    changed
}

/// The boards a shipped conception lays out that the stored conception of
/// the same kind does not (v0.100, ADR 0066): the research arrived after
/// the scenarios, on songs already prepared on a conception. Each goes where
/// the shipped list puts it - after the board it follows there, or first -
/// when the stored copy has the board at all. A board the owner took off a
/// conception comes back, the price `carry_forward` states.
pub(super) fn conception_boards(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let boards: Vec<String> = config.boards.iter().map(|b| b.key.clone()).collect();
    let mut changed = false;
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        let (Some(conception), Some(shipped_conception)) =
            (kind.conception.as_mut(), shipped_kind.conception.as_ref())
        else {
            continue;
        };
        for (index, key) in shipped_conception.boards.iter().enumerate() {
            if conception.boards.contains(key) || !boards.contains(key) {
                continue;
            }
            let after = shipped_conception.boards[..index]
                .iter()
                .rev()
                .find_map(|before| conception.boards.iter().position(|b| b == before));
            let at = after.map_or(0, |position| position + 1);
            conception.boards.insert(at, key.clone());
            changed = true;
        }
    }
    changed
}

/// The scenarios' template still reading as it shipped in v0.99 follows the
/// shipped wording, which reads what is starred in the research - once the
/// copy has the board of research to read: a template that names a board
/// the copy does not have would read nothing.
pub(super) fn research_templates(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    if config.board("research").is_none() {
        return false;
    }
    let mut changed = false;
    for prompt in &mut config.prompts {
        let Some(shipped) = shipped.prompts.iter().find(|s| s.key == prompt.key) else {
            continue;
        };
        if TEMPLATES_BEFORE_THE_RESEARCH
            .iter()
            .any(|(key, before)| *key == prompt.key && *before == prompt.template)
            && prompt.template != shipped.template
        {
            prompt.template = shipped.template.clone();
            changed = true;
        }
    }
    changed
}

/// The library (v0.100, ADR 0066): a stored note kind of the key the shipped
/// library has - Blog's `source`, a plain kind since it shipped - becomes the
/// library where it is a plain page, and no other kind of the copy is one;
/// its media arrive where it names none, and newly shipped ones by key. A
/// copy that never had the kind gains it whole with the keyed vocabulary.
pub(super) fn library_vocabulary(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let Some(shipped_library) = shipped.library() else {
        return false;
    };
    let another = config
        .note_kinds
        .iter()
        .any(|kind| kind.library && kind.key != shipped_library.key);
    let Some(kind) = config
        .note_kinds
        .iter_mut()
        .find(|kind| kind.key == shipped_library.key)
    else {
        return false;
    };
    let mut changed = false;
    if !kind.library {
        if another || kind.is_card() || kind.material || kind.line {
            return false;
        }
        kind.library = true;
        changed = true;
    }
    changed |= insert_new_keys(&mut kind.media, &shipped_library.media, |medium| {
        &medium.key
    });
    changed
}

/// Every label a field ever shipped under, by key, and what it ships under
/// now. A stored field still labelled exactly as one of these follows the
/// shipped word; one the owner relabelled is theirs.
///
/// The premise is the idea a brief is refined into since v0.99, and the
/// Russian word for it says so; before the labels spoke two languages it
/// shipped as one English word.
pub(super) fn relabel_untouched_fields(
    config: &mut ProfileConfig,
    shipped: &ProfileConfig,
) -> bool {
    use config::Label;
    let as_shipped = |key: &str| -> Vec<Label> {
        match key {
            "premise" => vec![
                Label::One("Premise".into()),
                Label::PerLocale(
                    [("en", "Premise"), ("ru", "Завязка")]
                        .into_iter()
                        .map(|(locale, word)| (locale.to_owned(), word.to_owned()))
                        .collect(),
                ),
            ],
            _ => Vec::new(),
        }
    };
    let mut changed = false;
    for field in &mut config.work_meta_fields {
        let Some(now) = shipped.work_meta_fields.iter().find(|s| s.key == field.key) else {
            continue;
        };
        if field.label != now.label && as_shipped(&field.key).contains(&field.label) {
            field.label = now.label.clone();
            changed = true;
        }
    }
    changed
}

/// The types a field shipped under before, by key. The anchors of an
/// experiment were a text read line by line until a field could be a list
/// (v0.112).
pub(super) const FIELD_TYPES_BEFORE: [(&str, config::MetaFieldType); 1] =
    [("anchors", config::MetaFieldType::Multiline)];

/// A field still of the type it shipped under before follows the shipped
/// type; one the owner retyped is theirs. What works hold is left as it is:
/// a list reads a text stored before it as its lines
/// ([`config::MetaFieldType::entries`]), so nothing is rewritten here.
/// The part of the register a field is read in (v0.151, FR-20 20.2): a
/// shipped field that says it - a track's cover idea, a song's instruments -
/// says it in the stored copy too, where that copy says nothing. A field the
/// owner marked otherwise stays as they marked it.
pub(super) fn register_fields(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for field in &mut config.work_meta_fields {
        let Some(now) = shipped.work_meta_fields.iter().find(|s| s.key == field.key) else {
            continue;
        };
        if field.register.is_none() && now.register.is_some() {
            field.register = now.register;
            changed = true;
        }
    }
    changed
}

pub(super) fn retyped_fields(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for field in &mut config.work_meta_fields {
        let Some(now) = shipped.work_meta_fields.iter().find(|s| s.key == field.key) else {
            continue;
        };
        if field.field_type != now.field_type
            && FIELD_TYPES_BEFORE
                .iter()
                .any(|(key, before)| *key == field.key && *before == field.field_type)
        {
            field.field_type = now.field_type;
            changed = true;
        }
    }
    changed
}

/// The kinds a field of Studio shipped for before v0.117, by key: none is
/// every kind. A song's own facts - its tempo, its key, its mood, who sings,
/// what it says - stood on the overview of its clip, its audio release and
/// every short, where they are facts of something else.
pub(super) const FIELD_KINDS_BEFORE: [(&str, &[&str]); 11] = [
    ("bpm", &[]),
    ("key", &[]),
    ("mood", &[]),
    ("tempo", &[]),
    ("vocal", &[]),
    ("instruments", &[]),
    ("direction", &[]),
    ("language", &["song", "video", "audio", "short"]),
    ("tagline", &["song", "video", "audio", "short"]),
    ("perspective", &["song", "video", "audio", "short"]),
    ("premise", &["song", "video", "audio", "short"]),
];

/// The roles a kind shipped with before v0.117 and ships without now: an
/// audio release took a clip's plot and context when it was made a kind, and
/// has no text of its own.
pub(super) const ROLES_BEFORE: [(&str, &[&str]); 1] = [("audio", &["plot", "context"])];

/// A field still for the kinds it shipped for before, and a kind still with
/// the roles it shipped with before, follow what ships now (v0.117); what
/// the owner changed is theirs. Nothing a work holds is touched: a version
/// written in a role the kind no longer names is still read, and its tab
/// stays while the work holds one.
pub(super) fn card_leftovers(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for field in &mut config.work_meta_fields {
        let Some(now) = shipped.work_meta_fields.iter().find(|s| s.key == field.key) else {
            continue;
        };
        if field.kinds != now.kinds
            && FIELD_KINDS_BEFORE
                .iter()
                .any(|(key, before)| *key == field.key && field.kinds == *before)
        {
            field.kinds.clone_from(&now.kinds);
            changed = true;
        }
    }
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        let roles: Vec<&str> = kind
            .version_roles
            .iter()
            .map(|role| role.key.as_str())
            .collect();
        let now: Vec<&str> = shipped_kind
            .version_roles
            .iter()
            .map(|role| role.key.as_str())
            .collect();
        if roles != now
            && ROLES_BEFORE
                .iter()
                .any(|(key, before)| *key == kind.key && roles == *before)
        {
            kind.version_roles.clone_from(&shipped_kind.version_roles);
            changed = true;
        }
    }
    changed
}

/// The second language of a word, where the stored word still reads exactly
/// as the shipped one does in English (v0.112).
///
/// A profile stored before its words spoke two languages holds "Song",
/// "Released", "Draft" as plain strings, and a kind that arrived later came
/// with both - so a Russian window read "Released" beside a Russian word. A
/// stored word that is the shipped English, letter for letter, was never the
/// owner's own: it takes the shipped pair. A word they wrote - in any
/// language - differs from it and stays one string, said as typed whatever
/// the window speaks.
///
/// Read off the documents as JSON rather than field by field: a word is a
/// label wherever the shipped document holds a map of languages, so a label
/// added to the format tomorrow is covered the day it ships. Entries of a
/// list meet by `key` - or by `at`, a rubric's marks - and never by place.
pub(super) fn second_language(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let (Ok(mut stored), Ok(shipped)) = (
        serde_json::to_value(&*config),
        serde_json::to_value(shipped),
    ) else {
        return false;
    };
    if !pair_words(&mut stored, &shipped) {
        return false;
    }
    match serde_json::from_value(stored) {
        Ok(paired) => {
            *config = paired;
            true
        }
        Err(_) => false,
    }
}

/// Whether a value is a word in several languages: a map of strings that
/// has the source language.
pub(super) fn is_word_map(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(|map| {
        map.contains_key(config::SOURCE_LOCALE) && map.values().all(serde_json::Value::is_string)
    })
}

/// What two entries of a list are met by.
pub(super) fn entry_name(entry: &serde_json::Value) -> Option<String> {
    let entry = entry.as_object()?;
    entry
        .get("key")
        .or_else(|| entry.get("at"))
        .map(serde_json::Value::to_string)
}

/// Walk `stored` beside `shipped`, giving each stored word that is the
/// shipped English the shipped languages it lacks. See [`second_language`].
pub(super) fn pair_words(stored: &mut serde_json::Value, shipped: &serde_json::Value) -> bool {
    use serde_json::Value;
    if is_word_map(shipped) {
        let english = shipped.get(config::SOURCE_LOCALE);
        return match stored {
            Value::String(word) if Some(word.as_str()) == english.and_then(Value::as_str) => {
                *stored = shipped.clone();
                true
            }
            // A word stored in some languages already gains the rest.
            Value::Object(map)
                if map.values().all(Value::is_string)
                    && map.get(config::SOURCE_LOCALE) == english =>
            {
                let mut changed = false;
                for (locale, word) in shipped.as_object().into_iter().flatten() {
                    if !map.contains_key(locale) {
                        map.insert(locale.clone(), word.clone());
                        changed = true;
                    }
                }
                changed
            }
            _ => false,
        };
    }
    match (stored, shipped) {
        (Value::Object(stored), Value::Object(shipped)) => {
            let mut changed = false;
            for (name, value) in stored.iter_mut() {
                if let Some(shipped) = shipped.get(name) {
                    changed |= pair_words(value, shipped);
                }
            }
            changed
        }
        (Value::Array(stored), Value::Array(shipped)) => {
            let mut changed = false;
            for entry in stored.iter_mut() {
                let Some(name) = entry_name(entry) else {
                    continue;
                };
                if let Some(shipped) = shipped
                    .iter()
                    .find(|candidate| entry_name(candidate).as_deref() == Some(name.as_str()))
                {
                    changed |= pair_words(entry, shipped);
                }
            }
            changed
        }
        _ => false,
    }
}

/// Tabs of the work's card that were renamed, old id to new. The Notes tab
/// became the Conception tab in v0.99: it holds what is attached to the work
/// and, for a kind that names one, the conception.
pub(super) const RENAMED_TABS: [(&str, &str); 1] = [("notes", "conception")];

/// The tab a kind opens on, where it still names a tab by its old id - a
/// word the window no longer knows would open the overview instead.
pub(super) fn renamed_tabs(config: &mut ProfileConfig, _shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for kind in &mut config.work_kinds {
        let Some(open_on) = &kind.open_on else {
            continue;
        };
        if let Some((_, new)) = RENAMED_TABS.iter().find(|(old, _)| old == open_on) {
            kind.open_on = Some((*new).to_owned());
            changed = true;
        }
    }
    changed
}

/// Note kinds the line once shipped and no longer does. Their notes became
/// others in the migration of the same version - an idea is a note tagged
/// `idea` since v0.98 (ADR 0064) - so a kind left in a stored copy would be a
/// chip over nothing, and a way to make more of what the line retired.
pub(super) const RETIRED_NOTE_KINDS: [&str; 1] = ["idea"];

/// A retired note kind leaves the stored copy, unless the shipped profile
/// names it again. The plain kinds left then stand in the shipped order, in
/// the places they held among the cards: the first is the chip the Notes
/// screen opens on, and a copy grown kind by kind had the note
/// before the phrase - the screen of a workspace made before v0.98 would open
/// on the notes and a new one on the phrases. It happens once, with the
/// retirement, so an order the owner set afterwards is theirs.
pub(super) fn retired_note_kinds(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let before = config.note_kinds.len();
    config.note_kinds.retain(|kind| {
        !RETIRED_NOTE_KINDS.contains(&kind.key.as_str()) || shipped.note_kind(&kind.key).is_some()
    });
    if config.note_kinds.len() == before {
        return false;
    }
    let places: Vec<usize> = (0..config.note_kinds.len())
        .filter(|&at| !config.note_kinds[at].is_card())
        .collect();
    let mut plain: Vec<_> = places
        .iter()
        .map(|&at| config.note_kinds[at].clone())
        .collect();
    // A kind the line does not ship keeps its place after the shipped ones.
    plain.sort_by_key(|kind| {
        shipped
            .note_kinds
            .iter()
            .position(|one| one.key == kind.key)
            .unwrap_or(usize::MAX)
    });
    for (at, kind) in places.into_iter().zip(plain) {
        config.note_kinds[at] = kind;
    }
    true
}

/// The lines a work's feel is said along (FR-02, v0.124), where the stored
/// profile names none: a space made before them has no moods to pick from,
/// and the picker would stand empty. A profile that has them keeps them as
/// they are - the words on a line are the owner's to change.
pub(super) fn vibe_axes(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    if config.vibe.is_some() {
        return false;
    }
    config.vibe = Some(shipped.vibe());
    true
}

/// The stops of the stage dial: the five the line works by (FR-03, v0.119).
///
/// A stored profile on any other scale - the seven stops shipped until now,
/// the six before them, none at all in a workspace older than the dial -
/// takes the shipped five whole. A stage is kept on the five places, so a
/// stop anywhere else is one no work stands at any more, and a list mixing
/// the two scales would be a dial with twelve stops of which seven are dead.
/// The words of the old stops go with them: they named places that are gone.
///
/// A stored profile already on the five keeps its words - a stop the owner
/// renamed is theirs - and gains only what it lacks: a stop of the five it
/// does not name, and the exact colour of one that names none.
pub(super) fn stages(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let current = if shipped.stages.is_empty() {
        config::default_stages()
    } else {
        shipped.stages.clone()
    };
    let on_the_scale = !config.stages.is_empty()
        && config
            .stages
            .iter()
            .all(|stage| config::STOPS.contains(&stage.percent));
    if !on_the_scale {
        if config.stages == current {
            return false;
        }
        config.stages = current;
        return true;
    }

    let mut changed = false;
    for stop in &current {
        match config
            .stages
            .iter_mut()
            .find(|stage| stage.percent == stop.percent)
        {
            Some(stored) => {
                if stored.hex.is_none() && stop.hex.is_some() {
                    stored.hex.clone_from(&stop.hex);
                    changed = true;
                }
            }
            None => {
                config.stages.push(stop.clone());
                changed = true;
            }
        }
    }
    if changed {
        config.stages.sort_by_key(|stage| stage.percent);
    }
    changed
}

/// The types a style brick can be (0.75), and a type's hint and glyph where
/// the stored copy names none: the hint is what the assistant is told to
/// describe, so a shipped one filling a gap is a gain; one the owner wrote is
/// theirs (ADR 0031). A type and a family newly shipped take their place
/// after the one they follow in the shipped list.
/// The compositions (v0.94) arrive by key, whole: a stored copy written
/// before them has none, and one the owner reshaped keeps their parts and
/// rules. One is carried only where the stored copy can hold it - its kinds
/// with the role, its types and fields named - so the upgrade never writes
/// a document that does not validate.
pub(super) fn compositions(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for composition in &shipped.compose {
        if config.compose.iter().any(|c| c.key == composition.key) {
            continue;
        }
        let holds = composition.kinds.iter().all(|kind| {
            config.kind(kind).is_some_and(|found| {
                found
                    .version_roles
                    .iter()
                    .any(|role| role.key == composition.role)
            })
        }) && composition.parts.iter().all(|part| {
            config
                .style_type(&part.type_key)
                .is_some_and(|style| style.form == config::StyleForm::Phrase)
        }) && composition
            .fields
            .iter()
            .all(|field| config.work_meta_fields.iter().any(|f| f.key == field.field));
        if holds {
            config.compose.push(composition.clone());
            changed = true;
        }
    }
    changed
}

pub(super) fn style_types(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = insert_new_keys(&mut config.style_types, &shipped.style_types, |style| {
        &style.key
    });
    for style in &mut config.style_types {
        let Some(shipped_style) = shipped.style_types.iter().find(|s| s.key == style.key) else {
            continue;
        };
        if style.hint.is_none() && shipped_style.hint.is_some() {
            style.hint = shipped_style.hint.clone();
            changed = true;
        }
        if style.icon.is_none() && shipped_style.icon.is_some() {
            style.icon = shipped_style.icon.clone();
            changed = true;
        }
        // The form, retirement and stand-in arrive on the same terms (v0.87):
        // where the stored type says nothing. A stored type still drawn as
        // pictures while the shipped one is lettering has never been told
        // otherwise - the form did not exist before.
        if style.form == config::StyleForm::Picture && shipped_style.form != style.form {
            style.form = shipped_style.form;
            changed = true;
        }
        // Families arrive by key (v0.90.2), as every keyed word does: a set
        // that grows a family files its new styles under it, and a stored
        // type that already has families would otherwise never learn it.
        changed |= insert_new_keys(&mut style.families, &shipped_style.families, |family| {
            &family.key
        });
        if style.retired.is_none() && shipped_style.retired.is_some() {
            style.retired = shipped_style.retired.clone();
            changed = true;
        }
        if style.canon_kind.is_none() && shipped_style.canon_kind.is_some() {
            style.canon_kind = shipped_style.canon_kind.clone();
            changed = true;
        }
    }
    changed
}

/// A mark's glyph, where the stored mark names none: the built-in marks
/// reached every workspace before marks had glyphs; a glyph the owner chose
/// stays theirs.
pub(super) fn mark_icons(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for mark in &mut config.marks {
        let Some(shipped_mark) = shipped.marks.iter().find(|m| m.key == mark.key) else {
            continue;
        };
        if mark.icon.is_none() && shipped_mark.icon.is_some() {
            mark.icon = shipped_mark.icon.clone();
            changed = true;
        }
    }
    changed
}

/// The canon (0.84): a note kind the workspace already has - its characters,
/// its places - becomes a kind of card by gaining the shipped sections, where
/// the stored kind names none; so do its root flag, the sections its
/// description is written from, and its glyph. A kind whose sections the
/// owner wrote keeps them whole: appending the shipped ones would put two
/// "Looks" on one card. The kinds of relation arrive by key, as every keyed
/// word does.
pub(super) fn canon_vocabulary(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = insert_new_keys(
        &mut config.relation_kinds,
        &shipped.relation_kinds,
        |kind| &kind.key,
    );
    for kind in &mut config.note_kinds {
        let Some(shipped_kind) = shipped.note_kinds.iter().find(|s| s.key == kind.key) else {
            continue;
        };
        if kind.sections.is_empty() && !shipped_kind.sections.is_empty() {
            kind.sections = shipped_kind.sections.clone();
            kind.root = shipped_kind.root;
            if kind.describe_from.is_empty() {
                kind.describe_from = shipped_kind.describe_from.clone();
            }
            changed = true;
        } else if !kind.sections.is_empty() {
            changed |= card_sections(&kind.key, &mut kind.sections, &shipped_kind.sections);
        }
        if kind.icon.is_none() && shipped_kind.icon.is_some() {
            kind.icon = shipped_kind.icon.clone();
            changed = true;
        }
        // The kinds of note shipped in English only until the canon; a word
        // still reading exactly as it shipped was never retyped, and follows
        // the shipped word into every language it now has. A word the owner
        // typed stays theirs.
        if let config::Label::One(word) = &kind.label
            && word == shipped_kind.label.as_str()
            && kind.label != shipped_kind.label
        {
            kind.label = shipped_kind.label.clone();
            changed = true;
        }
    }
    changed
}

/// Sections whose words changed since they shipped: a stored section still
/// reading exactly as it shipped before follows the new ones. By kind of
/// card and section, the lenses and the hint they shipped with.
///
/// v0.88 split what a picture must not show from what a text must not say:
/// bans read by the cover's lens reach a picture's negative, and the
/// channel's bans were mostly about texts - moderation words, the voice.
#[allow(clippy::type_complexity)]
pub(super) const SECTIONS_BEFORE: [(&str, &str, &[config::Lens], Option<(&str, &str)>); 2] = [
    (
        "channel",
        "bans",
        &[config::Lens::Cover, config::Lens::Public],
        Some((
            "→ the negative prompt and the idea generator",
            "→ негатив и генератор идей",
        )),
    ),
    (
        "character",
        "bans",
        &[config::Lens::Cover, config::Lens::Public],
        None,
    ),
];

/// Sections shipped after the canon itself, by kind of card, with the
/// section each arrives after. One arrives only in a kind that has that
/// section - a card kind the canon gave its sections - and never in one whose
/// sections the owner wrote: appending shipped sections there would put two
/// of one thing on a card under two names.
pub(super) const SECTIONS_ARRIVED: [(&str, &str, &str); 9] = [
    ("channel", "picture_bans", "bans"),
    ("character", "picture_bans", "bans"),
    // v0.94: the channel's own phrases of sound, after its house styles.
    ("channel", "sound", "styles"),
    // v0.100.2: the pools a release's text is drawn from, after the
    // template that says what the text holds - each after the one before.
    ("channel", "moves", "template"),
    ("channel", "assemblies", "moves"),
    ("channel", "pinned_forms", "assemblies"),
    // v0.147: the channel's dressing kits, after its mark - shipped without
    // an arrival, so a space made before kept a passport without them.
    ("channel", "branding", "mark"),
    // v0.151: the rules the works are checked against and the pool of
    // themes taken in turn, after the dressing.
    ("channel", "quotas", "branding"),
    ("channel", "bank", "quotas"),
];

/// A card kind's sections the stored copy already has, brought forward
/// (v0.88): a section shipped since arrives after the one it follows
/// ([`SECTIONS_ARRIVED`]); a section still of plain statements takes the shape
/// the shipped one now has - the shape is who reads it, and a stored section
/// was never told otherwise; a hint arrives where the stored section has
/// none; and the lenses and hint still exactly as they shipped before follow
/// the new ones ([`SECTIONS_BEFORE`]). A word the owner wrote stays theirs.
pub(super) fn card_sections(
    kind: &str,
    sections: &mut Vec<config::CanonSection>,
    shipped: &[config::CanonSection],
) -> bool {
    let mut changed = false;
    for (of, key, after) in SECTIONS_ARRIVED {
        if of != kind || sections.iter().any(|section| section.key == key) {
            continue;
        }
        let (Some(at), Some(arriving)) = (
            sections.iter().position(|section| section.key == after),
            shipped.iter().find(|section| section.key == key),
        ) else {
            continue;
        };
        sections.insert(at + 1, arriving.clone());
        changed = true;
    }
    for section in sections.iter_mut() {
        let Some(shipped_section) = shipped.iter().find(|s| s.key == section.key) else {
            continue;
        };
        if section.shape == config::SectionShape::Facts
            && shipped_section.shape != config::SectionShape::Facts
        {
            section.shape = shipped_section.shape;
            changed = true;
        }
        let before = SECTIONS_BEFORE
            .iter()
            .find(|(of, key, _, _)| *of == kind && *key == section.key);
        if let Some((_, _, lenses, hint)) = before {
            if section.lenses.as_slice() == *lenses && section.lenses != shipped_section.lenses {
                section.lenses = shipped_section.lenses.clone();
                changed = true;
            }
            let as_shipped = match (hint, &section.hint) {
                (Some((en, ru)), Some(stored)) => {
                    stored.in_locale("en") == *en && stored.in_locale("ru") == *ru
                }
                _ => false,
            };
            if as_shipped && section.hint != shipped_section.hint {
                section.hint = shipped_section.hint.clone();
                changed = true;
            }
        }
        if section.hint.is_none() && shipped_section.hint.is_some() {
            section.hint = shipped_section.hint.clone();
            changed = true;
        }
    }
    changed
}

/// What a publication is made of (v0.86): the shape of a door's cover, a
/// kind's cover (v0.88) and frame and the title a work of it takes when it is
/// made from another - each where the stored copy says nothing, matched by
/// key. And a
/// field's kinds, options and starting value where the stored field names
/// none: the variant of an audio release is a choice for audio alone, and a
/// field that arrived by key without them would be a free box on every kind.
pub(super) fn publication_vocabulary(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        if !kind.frame && shipped_kind.frame {
            kind.frame = true;
            changed = true;
        }
        if !kind.cover && shipped_kind.cover {
            kind.cover = true;
            changed = true;
        }
        if kind.made_title.is_none() && shipped_kind.made_title.is_some() {
            kind.made_title = shipped_kind.made_title.clone();
            changed = true;
        }
        for door in &mut kind.release_kinds {
            let Some(shipped_door) = shipped_kind
                .release_kinds
                .iter()
                .find(|candidate| candidate.key == door.key)
            else {
                continue;
            };
            if door.cover_format.is_none() && shipped_door.cover_format.is_some() {
                door.cover_format = shipped_door.cover_format.clone();
                changed = true;
            }
        }
    }
    for field in &mut config.work_meta_fields {
        let Some(shipped_field) = shipped
            .work_meta_fields
            .iter()
            .find(|candidate| candidate.key == field.key)
        else {
            continue;
        };
        if field.kinds.is_empty() && !shipped_field.kinds.is_empty() {
            field.kinds = shipped_field.kinds.clone();
            changed = true;
        }
        if field.options.is_empty()
            && !shipped_field.options.is_empty()
            && field.field_type == shipped_field.field_type
        {
            field.options = shipped_field.options.clone();
            changed = true;
        }
        if field.default.is_none() && shipped_field.default.is_some() {
            field.default = shipped_field.default.clone();
            changed = true;
        }
    }
    changed
}

/// What a release goes out under, as v0.90 says it, where the stored copy
/// says nothing or still says what shipped before (ADR 0051). A field's
/// template still reading `{title}`, or nothing, where the shipped one now
/// reads `{origin}`: since v0.90 a publication's own name is "Song (audio)",
/// kilna's word for it, and the audience sees the song's. A field's tail and
/// its premiere's tail, and a kind's word that it is an excerpt (v0.100.1,
/// ADR 0067), where the stored copy says none. And a field of the work's own
/// media - a length - where the stored one is still carried into what is
/// made from it. Matched by key; a template the owner reworded stays theirs.
pub(super) fn going_out_words(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    const TEMPLATE_BEFORE_V090: &str = "{title}";
    let mut changed = false;
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        for door in &mut kind.release_kinds {
            let Some(shipped_door) = shipped_kind
                .release_kinds
                .iter()
                .find(|candidate| candidate.key == door.key)
            else {
                continue;
            };
            for field in &mut door.fields {
                let Some(shipped_field) = shipped_door
                    .fields
                    .iter()
                    .find(|candidate| candidate.key == field.key)
                else {
                    continue;
                };
                let untouched = field
                    .template
                    .as_deref()
                    .is_none_or(|template| template.trim() == TEMPLATE_BEFORE_V090);
                let reads_origin = shipped_field
                    .template
                    .as_deref()
                    .is_some_and(|template| template.contains("{origin}"));
                if untouched && reads_origin {
                    field.template = shipped_field.template.clone();
                    changed = true;
                }
                if field.suffix.is_none() && shipped_field.suffix.is_some() {
                    field.suffix = shipped_field.suffix.clone();
                    changed = true;
                }
                if field.premiere.is_none() && shipped_field.premiere.is_some() {
                    field.premiere = shipped_field.premiere.clone();
                    changed = true;
                }
            }
        }
        if !kind.excerpt && shipped_kind.excerpt {
            kind.excerpt = true;
            changed = true;
        }
    }
    for field in &mut config.work_meta_fields {
        let own = shipped
            .work_meta_fields
            .iter()
            .any(|candidate| candidate.key == field.key && candidate.own);
        if own && !field.own {
            field.own = true;
            changed = true;
        }
    }
    changed
}

/// Fields shipped after a door was stored, by kind of work and door, with the
/// field each arrives after. One arrives only in a door that has that field,
/// each after the one before it, and never twice: a field of its key already
/// in the door is the owner's, whatever it says.
///
/// v0.100.2: an audio track's two titles for later, kept beside the title it
/// premieres under - "Song (Song premiere 2026)" is the right title for a
/// month and a stale one by spring.
pub(super) const FIELDS_ARRIVED: [(&str, &str, &str, &str); 2] = [
    ("audio", "youtube", "alt_title_1", "title"),
    ("audio", "youtube", "alt_title_2", "alt_title_1"),
];

/// Limits a field shipped with that were wrong, by the field's key and the
/// limit it shipped with: a stored limit still reading exactly so follows
/// the shipped one, a limit the owner typed stays theirs. A YouTube title
/// takes 100 characters, and the titles of a clip and an audio track counted
/// to 205 until v0.100.2 - a count that said a title fits when YouTube
/// would cut it.
pub(super) const LIMITS_BEFORE: [(&str, u32); 1] = [("title", 205)];

/// A door's fields newly shipped ([`FIELDS_ARRIVED`]), and a limit still as
/// it shipped wrong ([`LIMITS_BEFORE`]).
pub(super) fn arriving_fields(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for (kind, shipped_kind) in shared_kinds(config, shipped) {
        for door in &mut kind.release_kinds {
            let Some(shipped_door) = shipped_kind
                .release_kinds
                .iter()
                .find(|candidate| candidate.key == door.key)
            else {
                continue;
            };
            for (of, at, key, after) in FIELDS_ARRIVED {
                if of != kind.key
                    || at != door.key
                    || door.fields.iter().any(|field| field.key == key)
                {
                    continue;
                }
                let (Some(position), Some(arriving)) = (
                    door.fields.iter().position(|field| field.key == after),
                    shipped_door.fields.iter().find(|field| field.key == key),
                ) else {
                    continue;
                };
                door.fields.insert(position + 1, arriving.clone());
                changed = true;
            }
            for field in &mut door.fields {
                let Some(shipped_field) = shipped_door
                    .fields
                    .iter()
                    .find(|candidate| candidate.key == field.key)
                else {
                    continue;
                };
                let as_shipped = LIMITS_BEFORE
                    .iter()
                    .any(|(key, before)| *key == field.key && field.limit == Some(*before));
                if as_shipped && field.limit != shipped_field.limit {
                    field.limit = shipped_field.limit;
                    changed = true;
                }
            }
        }
    }
    changed
}

/// Shipped profile names that changed, old to new. A stored copy still
/// carrying the old one is renamed at the next start; see `carry_forward`.
pub(super) const RENAMED: [(&str, &str); 1] = [("Music", "Studio")];

/// The `score` templates as the four profiles shipped them before v0.61,
/// when the axes were named in the template's own words. A stored copy
/// still reading exactly like one of these follows the shipped wording;
/// see `carry_forward`.
pub(super) const SCORE_TEMPLATES_BEFORE_METHODS: [&str; 5] = [
    // Studio's v0.61 wording read `{role:lyrics}`, which a video does not
    // have; from v0.64 the action is for every kind and reads `{body}`.
    "Here are the lyrics of a song called \"{title}\".

{role:lyrics}

Judge it as a finished song, along the axes below. Be honest rather than kind: a score that flatters is worth nothing.",
    "Here are the lyrics of a song called \"{title}\".\n\n{role:lyrics}\n\nJudge it along these axes: Hook (Does the chorus stay with you after one listen), Lyrics (Imagery, rhyme and whether a line earns its place), Emotion (Does it move a listener who knows nothing about it), Production (Arrangement, mix and how finished it sounds), Originality (Distance from the obvious version of this idea), Visual potential (Is there a clip in it, or only a cover).\n\nBe honest rather than kind: a score that flatters is worth nothing.",
    "Here is a chapter called \"{title}\".\n\n{body}\n\nJudge it along these axes: Pull (Does the reader turn the page, or put the book down here), Prose (Sentence by sentence: rhythm, precision, nothing limp), Character (Do people behave like people rather than like plot requirements), Structure (Does the chapter earn its place and end where it should), Tension (Is something at stake on every page).\n\nBe honest rather than kind: a score that flatters is worth nothing.",
    "Here are the notes for an episode called \"{title}\".\n\n{body}\n\nJudge it along these axes: Hook (Does the cold open earn the next thirty seconds), Clarity (Could a listener explain the point to someone else afterward), Pacing (Where does it drag, and would a listener skip ahead), Insight (Is there a claim here nobody else is making, or just a summary), Delivery (Energy, pauses, whether it sounds read or spoken), Shareability (Is there a moment worth clipping and sending to a friend).\n\nBe honest rather than kind: a score that flatters is worth nothing.",
    "Here is a post called \"{title}\".\n\n{body}\n\nJudge it along these axes: Hook (Does the first paragraph survive contact with a stranger's attention span), Usefulness (Could a reader act on this, or is it just an opinion floating by), Clarity (Sentence by sentence: does every paragraph earn the next one), Angle (Is there a take here, or a restatement of what everyone already thinks), Shareability (Is there a line worth quoting out of context).\n\nBe honest rather than kind: a score that flatters is worth nothing.",
];

/// Templates as they shipped before they read the register of repeats
/// (v0.85, ADR 0044), by the action they belong to. A stored copy still
/// reading exactly like one of these follows the shipped wording, which reads
/// `{register}`; a reworded one stays the owner's.
pub(super) const TEMPLATES_BEFORE_THE_REGISTER: [(&str, &str); 2] = [
    (
        "critique",
        "Here are the lyrics of a song called \"{title}\".\n\n{role:lyrics}\n\nBe specific and be hard on it: which lines are weak, which images are worn out, what would you cut? Do not rewrite it — say what is wrong.",
    ),
    (
        "polish",
        "Here are the lyrics of a song called \"{title}\".\n\n{role:lyrics}\n\nSuggest one revision that keeps the voice but fixes what is weakest. Show the changed lines only, with a sentence on why each changed.",
    ),
];

/// The critiques as they shipped before they read the context of v0.106
/// (v0.112), by the action they belong to: the version before the one read
/// and the score that speaks for it. A stored copy still reading exactly like
/// one of these follows the shipped wording, which reads `{previous_version}`
/// and `{score}`; a reworded one stays the owner's.
pub(super) const TEMPLATES_BEFORE_THE_CONTEXT: [(&str, &str); 4] = [
    (
        "critique",
        "Here are the lyrics of a song called \"{title}\".\n\n{role:lyrics}\n\n{register}\n\nBe specific and be hard on it: which lines are weak, which images are worn out, what would you cut? A word the register bans, or leans on past its limit, is a finding. Do not rewrite it — say what is wrong.",
    ),
    (
        "critique",
        "Here is a chapter called \"{title}\".\n\n{role:text}\n\nWhere would a reader put this down? Be specific about paragraphs. Name what is limp, what is explained twice, and what the chapter is avoiding. Do not rewrite it.",
    ),
    (
        "critique",
        "Here is the script for an episode called \"{title}\".\n\n{role:script}\n\nWhere would a listener stop listening? Be specific about timestamps or sections. Name what rambles, what over-explains, and what the episode is avoiding. Do not rewrite it — say what is wrong.",
    ),
    (
        "critique",
        "Here is a post called \"{title}\".\n\n{role:draft}\n\nWhere would a stranger stop reading? Be specific about paragraphs. Name what is padded, what is obvious, and what the post is avoiding saying. Do not rewrite it — say what is wrong.",
    ),
];

/// The scenarios' templates as they shipped before the research (v0.99), by
/// the action they belong to: they did not read what is starred on the board
/// of research. A copy still reading exactly like one follows the shipped
/// wording, which reads `{starred:research}`; a reworded one stays the
/// owner's.
pub(super) const TEMPLATES_BEFORE_THE_RESEARCH: [(&str, &str); 4] = [
    (
        "scenarios",
        "A song called \"{title}\" is being prepared. This is what its author wrote about it:\n\n{conception}\n\n{cards}",
    ),
    (
        "scenarios",
        "A chapter called \"{title}\" is being prepared. This is what its author wrote about it:\n\n{conception}\n\n{cards}",
    ),
    (
        "scenarios",
        "A episode called \"{title}\" is being prepared. This is what its author wrote about it:\n\n{conception}\n\n{cards}",
    ),
    (
        "scenarios",
        "A post called \"{title}\" is being prepared. This is what its author wrote about it:\n\n{conception}\n\n{cards}",
    ),
];

/// Methods as earlier releases shipped them, by the action they belong to
/// (ADR 0021). A stored method still reading exactly like one of these was
/// never touched by the owner, and follows the shipped wording; one they
/// rewrote stays theirs. Without it a method once stored never changes again:
/// [`prompt_details`] only fills a method that is missing.
pub(super) const METHODS_BEFORE: [(&str, &str); 3] = [
    (
        "release-meta",
        include_str!("methods_before/release-meta-0.86.md"),
    ),
    (
        "release-meta",
        include_str!("methods_before/release-meta-0.100.1.md"),
    ),
    (
        "release-meta",
        include_str!("methods_before/release-meta-0.100.2.md"),
    ),
];

/// A method still reading as an earlier release shipped it moves to the
/// shipped wording; see [`METHODS_BEFORE`].
pub(super) fn rewritten_methods(config: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    let mut changed = false;
    for prompt in &mut config.prompts {
        let Some(stored) = prompt.method() else {
            continue;
        };
        let untouched = METHODS_BEFORE
            .iter()
            .any(|(key, before)| *key == prompt.key && before.trim() == stored);
        if !untouched {
            continue;
        }
        let Some(method) = shipped
            .prompts
            .iter()
            .find(|candidate| candidate.key == prompt.key)
            .and_then(|candidate| candidate.method())
        else {
            continue;
        };
        if method != stored {
            prompt.method = Some(method.to_owned());
            changed = true;
        }
    }
    changed
}

/// Shipped descriptions that changed, old to new, on the same terms as the
/// names: a copy the owner never touched follows, a rewritten one stays.
/// Every wording the file ever shipped maps to the current one, so a copy
/// that skipped a release still arrives.
pub(super) const REDESCRIBED: [(&str, &str); 3] = [
    (
        "Songs with independent lyrics and style drafts, judged on hook and craft, shipped as clips, shorts and audio releases.",
        STUDIO_DESCRIPTION,
    ),
    (
        "Songs and the videos made for them: lyrics and style drafts judged on hook and craft, clips and shorts judged on the cut, shipped as clips, shorts and audio releases.",
        STUDIO_DESCRIPTION,
    ),
    (
        "Songs and the videos made for them: lyrics and style drafts judged on hook and craft, shipped as audio releases; the clips and shorts cut to them are works of their own, judged on the cut.",
        STUDIO_DESCRIPTION,
    ),
];

/// The Studio profile's one-sentence description as it ships since v0.86:
/// a song has no door of its own, and everything that goes out for it is a
/// work made from it (ADR 0030, ADR 0047).
pub(super) const STUDIO_DESCRIPTION: &str = "Songs and what goes out for them: lyrics and style drafts judged on hook and craft; the clips, audio releases and shorts made from them are works of their own, each with its releases, cover and comments.";

/// The `format` a stored document claims — 1 when it says nothing, the way
/// every document written before the field did.
///
/// Only the top-level `format` is read: the rest of the document is stepped
/// over rather than built into a tree, since a stored Studio profile is
/// nearly half a megabyte and every start asks each profile this.
pub(super) fn raw_format(raw: &str) -> u32 {
    serde_json::from_str::<DeclaredFormat>(raw)
        .ok()
        .and_then(|declared| declared.0)
        .map_or(1, |format| format as u32)
}

/// The top-level `format` of a profile document, the last one when it is
/// written twice, as a whole non-negative number or nothing.
pub(super) struct DeclaredFormat(Option<u64>);

impl<'de> Deserialize<'de> for DeclaredFormat {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct TopLevel;

        impl<'de> serde::de::Visitor<'de> for TopLevel {
            type Value = DeclaredFormat;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a profile document")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<DeclaredFormat, A::Error> {
                let mut format = None;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "format" {
                        format = map.next_value::<serde_json::Value>()?.as_u64();
                    } else {
                        map.next_value::<serde::de::IgnoredAny>()?;
                    }
                }
                Ok(DeclaredFormat(format))
            }
        }

        deserializer.deserialize_map(TopLevel)
    }
}

/// Move a tier onto the word the shipped profile now gives it — but only
/// where the stored one still reads exactly as it shipped before.
///
/// Studio's song tiers were *Hold / Audio release / Picture track / Clip*:
/// they named what would be MADE from the song rather than what the song
/// was, and once the clips moved out into works of their own (v0.74) a song
/// was being judged "Clip". The keys are unchanged, so every score already
/// recorded keeps the tier it was given; only the word moves.
///
/// The condition is the one an action's template is held to: a wording the
/// owner has changed is theirs and stays, and a wording still identical to
/// what we shipped is ours to correct. Matched against the *previous*
/// shipped words rather than "is it different from the new one", because
/// the second would rewrite a rename that happens to differ.
pub(super) fn reword_untouched_tiers(stored: &mut ProfileConfig, shipped: &ProfileConfig) -> bool {
    // What Studio shipped before v0.74.2, by key. A tier whose stored word is
    // not one of these was renamed by hand and is left alone.
    const AS_SHIPPED: [(&str, &str); 4] = [
        ("hold", "Hold"),
        ("audio", "Audio release"),
        ("picture", "Picture track"),
        ("clip", "Clip"),
    ];

    let word_for = |key: &str| {
        shipped
            .work_kinds
            .iter()
            .flat_map(|kind| kind.tiers.iter())
            .find(|tier| tier.key == key)
            .map(|tier| tier.label.clone())
    };

    let mut changed = false;
    for tiers in stored.work_kinds.iter_mut().map(|kind| &mut kind.tiers) {
        for tier in tiers.iter_mut() {
            let Some((_, before)) = AS_SHIPPED.iter().find(|(key, _)| *key == tier.key) else {
                continue;
            };
            if tier.label != *before {
                continue;
            }
            let Some(now) = word_for(&tier.key) else {
                continue;
            };
            if tier.label != now {
                tier.label = now;
                changed = true;
            }
        }
    }
    changed
}

/// Entries whose key the stored list does not have yet, each put where the
/// shipped list puts it: after the nearest entry before it there that the
/// stored list has, and at the end when it has none - so a dictionary read in
/// its list's order does not grow a tail of whatever shipped last. The one
/// rule for every keyed list since v0.112: until then most lists appended,
/// and a type shipped later stood after every type the owner had, not after
/// the one the shipped dictionary reads it beside. An order the owner
/// arranged is not rewritten: only the newcomer is placed.
///
/// Returns whether anything was added, so the caller can tell a config that
/// needs writing from one that does not.
pub(super) fn insert_new_keys<T: Clone>(
    stored: &mut Vec<T>,
    shipped: &[T],
    key: impl Fn(&T) -> &String,
) -> bool {
    let mut changed = false;
    for (index, candidate) in shipped.iter().enumerate() {
        if stored
            .iter()
            .any(|existing| key(existing) == key(candidate))
        {
            continue;
        }
        let after = shipped[..index]
            .iter()
            .rev()
            .find_map(|before| stored.iter().position(|e| key(e) == key(before)));
        let at = after.map_or(stored.len(), |position| position + 1);
        stored.insert(at, candidate.clone());
        changed = true;
    }
    changed
}
