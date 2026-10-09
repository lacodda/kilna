//! Gestures on an experiment's board of trials (ADR 0061).
//!
//! A trial is put on the board, written, heard, judged and varied - each the
//! trial's own operation, logged, undone and replayed as an idea's is. What a
//! kept trial becomes is made by the gestures that make those things: a
//! version, a brick, a work and its link, each remembering the trial - so
//! taking a harvest back is the undo of the version or the brick it made.

use rusqlite::Connection;

use super::gesture;
use crate::db::unit::atomically;
use crate::error::{Error, Result};
use crate::lab::trial::{self, NewTrial, Trial, TrialPatch, Verdict};
use crate::link::NewLink;
use crate::style_brick::{NewStyleBrick, StyleBrick};
use crate::work::version::{NewVersion, Version};
use crate::work::{NewWork, Work};

/// Put a trial on a board. Its series and place are settled before the
/// operation is logged, so a replay lands it where it landed here.
pub fn create(conn: &Connection, mut new: NewTrial) -> Result<Trial> {
    gesture(conn, "trial.create", |act| {
        let series = new
            .series
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_owned();
        if new.position.is_none() {
            new.position = Some(trial::next_position(act, &new.work_id, &series)?);
        }
        new.series = Some(series);
        act.json("trial", &new)?;
        let minted = act.mint();
        trial::create_minted(act, act.profile_id(), new, minted)
    })
}

/// Change a trial, with what the fields held before, so it can be taken
/// back. A change to nothing records nothing.
pub fn update(conn: &Connection, id: &str, patch: TrialPatch) -> Result<Trial> {
    gesture(conn, "trial.update", |act| {
        let before = trial::get(act, id)?.ok_or_else(|| Error::not_found("trial", id))?;
        if patch.changes_nothing(&before) {
            act.unchanged();
            return Ok(before);
        }
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(Some(&before), &patch)?;
        act.stamped();
        trial::update_at(act, id, patch, act.at())
    })
}

/// Keep a trial, drop it, or take the verdict back.
pub fn judge(conn: &Connection, id: &str, verdict: Option<Verdict>) -> Result<Trial> {
    update(
        conn,
        id,
        TrialPatch {
            verdict: Some(verdict),
            ..TrialPatch::default()
        },
    )
}

/// A variation of a trial: its copy, a child of it, with what is to move
/// said in `angle`. It stands in `series` when one is given, beside its
/// parent otherwise, and reworks what the parent reworks.
pub fn vary(conn: &Connection, id: &str, angle: &str, series: Option<&str>) -> Result<Trial> {
    let parent = trial::get(conn, id)?.ok_or_else(|| Error::not_found("trial", id))?;
    create(
        conn,
        NewTrial {
            work_id: parent.work_id.clone(),
            series: Some(
                series
                    .map(str::trim)
                    .filter(|series| !series.is_empty())
                    .unwrap_or(&parent.series)
                    .to_owned(),
            ),
            parent_id: Some(parent.id.clone()),
            angle: Some(angle.trim().to_owned()),
            body: Some(parent.body),
            bricks: Some(parent.bricks),
            reference: Some(parent.reference),
            source_version_id: parent.source_version_id,
            ..NewTrial::default()
        },
    )
}

/// A trial from a work's text: a version of the role the lab keeps trials
/// in - a song's style - put on the board as it stands, to be reworked.
pub fn from_version(
    conn: &Connection,
    work_id: &str,
    version_id: &str,
    series: &str,
) -> Result<Trial> {
    let version = crate::work::version::get(conn, version_id)?
        .ok_or_else(|| Error::not_found("version", version_id))?;
    let angle = match crate::work::get(conn, &version.work_id)? {
        Some(source) => match &version.label {
            Some(label) if !label.trim().is_empty() => {
                format!("{} · {}", source.title, label.trim())
            }
            _ => source.title,
        },
        None => String::new(),
    };
    create(
        conn,
        NewTrial {
            work_id: work_id.to_owned(),
            series: Some(series.to_owned()),
            angle: Some(angle),
            body: Some(version.body),
            source_version_id: Some(version.id),
            ..NewTrial::default()
        },
    )
}

/// Take a kept trial into a work: a version of the role the lab keeps trials
/// in, its text the trial's, written from the version the trial reworks when
/// that is one of this work's - a branch beside its original - and, unless
/// asked, not made current: a found sound is put beside the song's style,
/// not over it.
pub fn harvest_into(
    conn: &Connection,
    id: &str,
    work_id: &str,
    label: Option<&str>,
    make_current: bool,
) -> Result<Version> {
    let found = trial::get(conn, id)?.ok_or_else(|| Error::not_found("trial", id))?;
    let role = harvest_role(conn, &found)?;
    let parent = trial::harvest_parent(conn, &found, work_id, &role)?;
    super::version::create(
        conn,
        work_id,
        NewVersion {
            role,
            body: found.body.clone(),
            label: label
                .map(str::trim)
                .filter(|label| !label.is_empty())
                .map(str::to_owned),
            meta: None,
            make_current,
            parent_version_id: parent,
            trial_id: Some(found.id),
        },
    )
}

/// Cut a phrase of the dictionary out of a kept trial: the whole text, or one
/// line of it, as a brick of a type its trials are read against.
pub fn harvest_phrase(conn: &Connection, id: &str, mut new: NewStyleBrick) -> Result<StyleBrick> {
    trial::get(conn, id)?.ok_or_else(|| Error::not_found("trial", id))?;
    let phrase = new
        .description
        .as_deref()
        .map(str::trim)
        .filter(|phrase| !phrase.is_empty())
        .unwrap_or(new.name.trim())
        .to_owned();
    if phrase.is_empty() {
        return Err(Error::refused("style.needsName"));
    }
    // A phrase is named by itself: the dictionary finds it by the words.
    new.name = phrase.clone();
    new.description = Some(phrase);
    new.trial_id = Some(id.to_owned());
    super::style::create(conn, new)
}

/// Make a new work of a kept trial: a work of `kind` whose first text in the
/// role the lab keeps trials in is the trial's, made from the experiment -
/// the work, its text and the link in one unit.
pub fn harvest_work(conn: &Connection, id: &str, kind: &str, title: &str) -> Result<Work> {
    atomically(conn, |conn| {
        let found = trial::get(conn, id)?.ok_or_else(|| Error::not_found("trial", id))?;
        if found.verdict != Some(Verdict::Keep) {
            return Err(Error::refused("trial.notKept"));
        }
        let role = harvest_role(conn, &found)?;
        let config = crate::profile::config_for(conn, &found.profile_id)?;
        let experiment = crate::work::get(conn, &found.work_id)?
            .ok_or_else(|| Error::not_found("work", &found.work_id))?;
        if !config
            .harvest_kinds(&role, &experiment.kind)
            .any(|candidate| candidate.key == kind)
        {
            return Err(Error::refused("trial.wrongKind")
                .param("kind", kind)
                .param("role", role));
        }
        let title = title.trim();
        if title.is_empty() {
            return Err(Error::refused("trial.needsTitle"));
        }
        let created = super::work::create(
            conn,
            NewWork {
                kind: kind.to_owned(),
                title: title.to_owned(),
                ..NewWork::default()
            },
        )?;
        super::version::create(
            conn,
            &created.id,
            NewVersion {
                role,
                body: found.body.clone(),
                make_current: true,
                trial_id: Some(found.id.clone()),
                ..NewVersion::default()
            },
        )?;
        super::link::create(
            conn,
            NewLink {
                work_id: created.id.clone(),
                source_id: found.work_id.clone(),
                role: Some(crate::link::LAB.to_owned()),
                source_version_id: None,
            },
        )?;
        crate::work::get(conn, &created.id)?.ok_or_else(|| Error::not_found("work", &created.id))
    })
}

/// An experiment made from a work: a rework of its text. The work's newest
/// text in the role the lab keeps trials in becomes the first trial - the
/// core - in `series`, and the experiment is linked to the work it reworks.
/// The experiment is of the first lab kind that harvests into a role the
/// work has.
pub fn experiment_from(
    conn: &Connection,
    source_id: &str,
    title: &str,
    series: &str,
) -> Result<(Work, Option<Trial>)> {
    atomically(conn, |conn| {
        let source = crate::work::get(conn, source_id)?
            .ok_or_else(|| Error::not_found("work", source_id))?;
        let config = crate::profile::config_for(conn, &source.profile_id)?;
        let (lab_kind, role) = config
            .work_kinds
            .iter()
            .find_map(|kind| {
                let role = kind.lab.as_ref()?.harvest.as_deref()?;
                config
                    .harvest_kinds(role, &kind.key)
                    .any(|target| target.key == source.kind)
                    .then(|| (kind.key.clone(), role.to_owned()))
            })
            .ok_or_else(|| Error::refused("trial.noLabFor").param("kind", source.kind.clone()))?;
        let title = title.trim();
        let created = super::work::create(
            conn,
            NewWork {
                kind: lab_kind,
                title: if title.is_empty() {
                    source.title.clone()
                } else {
                    title.to_owned()
                },
                ..NewWork::default()
            },
        )?;
        super::link::create(
            conn,
            NewLink {
                work_id: created.id.clone(),
                source_id: source.id.clone(),
                role: Some(crate::link::LAB.to_owned()),
                source_version_id: None,
            },
        )?;
        let core = match crate::work::version::latest(conn, &source.id, &role)? {
            Some(version) => Some(from_version(conn, &created.id, &version.id, series)?),
            None => None,
        };
        let work = crate::work::get(conn, &created.id)?
            .ok_or_else(|| Error::not_found("work", &created.id))?;
        Ok((work, core))
    })
}

/// Put the trials of a proposal on a board, in the order they come: a trial
/// that varies one earlier in the same proposal is that one's child, when
/// that one was taken; one that varies a trial no longer on the board varies
/// nothing. `taken` says which, by index. Returns the ids of the trials made.
pub fn put(
    conn: &Connection,
    work_id: &str,
    trials: Vec<crate::lab::answer::Packaged>,
    taken: impl Fn(usize) -> bool,
) -> Result<Vec<String>> {
    use crate::lab::answer::Parent;
    atomically(conn, |conn| {
        let mut made: Vec<Option<String>> = Vec::with_capacity(trials.len());
        for (index, packaged) in trials.into_iter().enumerate() {
            if !taken(index) {
                made.push(None);
                continue;
            }
            let parent = match packaged.parent {
                Some(Parent::Earlier(earlier)) => made.get(earlier).cloned().flatten(),
                Some(Parent::Trial(id)) => trial::get(conn, &id)?
                    .filter(|found| found.work_id == work_id)
                    .map(|found| found.id),
                None => None,
            };
            // A variation reworks what its parent reworks, unless it says.
            let source = match (&packaged.source_version_id, &parent) {
                (Some(source), _) => Some(source.clone()),
                (None, Some(parent)) => trial::get(conn, parent)?.and_then(|p| p.source_version_id),
                (None, None) => None,
            };
            let created = create(
                conn,
                NewTrial {
                    work_id: work_id.to_owned(),
                    series: Some(packaged.series),
                    parent_id: parent,
                    angle: Some(packaged.angle),
                    body: Some(packaged.body),
                    bricks: Some(packaged.bricks),
                    reference: Some(packaged.reference),
                    outcome: Some(packaged.outcome),
                    verdict: packaged.verdict,
                    source_version_id: source,
                    run_first: Some(packaged.run_first),
                    position: None,
                },
            )?;
            made.push(Some(created.id));
        }
        Ok(made.into_iter().flatten().collect())
    })
}

/// The role a trial's experiment keeps its trials in.
fn harvest_role(conn: &Connection, found: &Trial) -> Result<String> {
    let (_, lab) = trial::a_lab(conn, &found.profile_id, &found.work_id)?;
    lab.harvest.ok_or_else(|| Error::refused("trial.noHarvest"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::trash::Entity;

    fn experiment(conn: &Connection, profile_id: &str) -> Work {
        fixtures::work(conn, profile_id, "experiment", "Breaks under guitars")
    }

    fn put(conn: &Connection, work_id: &str, body: &str) -> Trial {
        create(
            conn,
            NewTrial {
                work_id: work_id.to_owned(),
                series: Some("sweep".into()),
                body: Some(body.to_owned()),
                ..NewTrial::default()
            },
        )
        .unwrap()
    }

    /// A trial lands after the last of its series; a verdict is one
    /// operation, undone to the one before; saying it twice records nothing.
    #[test]
    fn a_trial_is_placed_judged_and_taken_back() {
        let (conn, profile_id) = fixtures::workspace();
        let lab = experiment(&conn, &profile_id);
        let first = put(&conn, &lab.id, "amen break");
        let second = put(&conn, &lab.id, "jungle break");
        assert_eq!((first.position, second.position), (1, 2));

        let before = crate::operation::count(&conn).unwrap();
        judge(&conn, &first.id, Some(Verdict::Keep)).unwrap();
        judge(&conn, &first.id, Some(Verdict::Keep)).unwrap();
        assert_eq!(crate::operation::count(&conn).unwrap(), before + 1);

        judge(&conn, &first.id, Some(Verdict::Drop)).unwrap();
        let offer = crate::undo::last(&conn).unwrap().unwrap();
        crate::undo::undo(&conn, &offer.operation_id).unwrap();
        assert_eq!(
            trial::get(&conn, &first.id).unwrap().unwrap().verdict,
            Some(Verdict::Keep)
        );
    }

    /// Only an experiment has a board, and a trial varies only a trial on
    /// the same board - never itself, through its own children.
    #[test]
    fn a_board_is_an_experiments_and_lineage_stays_on_it() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let refused = create(
            &conn,
            NewTrial {
                work_id: song.id.clone(),
                ..NewTrial::default()
            },
        )
        .unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "trial.noLab");

        let one = experiment(&conn, &profile_id);
        let other = fixtures::work(&conn, &profile_id, "experiment", "Another");
        let core = put(&conn, &one.id, "amen break");
        let stranger = put(&conn, &other.id, "jungle");
        let refused = create(
            &conn,
            NewTrial {
                work_id: one.id.clone(),
                parent_id: Some(stranger.id.clone()),
                ..NewTrial::default()
            },
        )
        .unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "trial.parentNotOnBoard");

        let child = vary(&conn, &core.id, "slower", None).unwrap();
        assert_eq!(child.parent_id.as_deref(), Some(core.id.as_str()));
        assert_eq!(child.body, core.body);
        assert_eq!(child.series, core.series);
        let refused = update(
            &conn,
            &core.id,
            TrialPatch {
                parent_id: Some(Some(child.id.clone())),
                ..TrialPatch::default()
            },
        )
        .unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "trial.parentIsDescendant");
    }

    /// A kept trial goes into a song as a version of its style, beside the
    /// current one and written from the version it reworks; a trial not kept
    /// goes nowhere; the board shows where it went.
    #[test]
    fn a_kept_trial_is_harvested_beside_the_songs_style() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let style = fixtures::version(&conn, &song.id, "style", "indie rock, female vocal");
        let lab = experiment(&conn, &profile_id);
        let rework = from_version(&conn, &lab.id, &style.id, "rework").unwrap();
        assert_eq!(rework.source_version_id.as_deref(), Some(style.id.as_str()));
        let variation = vary(&conn, &rework.id, "breaks under it", None).unwrap();
        assert_eq!(
            variation.source_version_id.as_deref(),
            Some(style.id.as_str()),
            "a variation reworks what its parent reworks"
        );

        let refused = harvest_into(&conn, &variation.id, &song.id, None, false).unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "trial.notKept");

        judge(&conn, &variation.id, Some(Verdict::Keep)).unwrap();
        let harvested = harvest_into(&conn, &variation.id, &song.id, None, false).unwrap();
        assert_eq!(harvested.role, "style");
        assert_eq!(
            harvested.parent_version_id.as_deref(),
            Some(style.id.as_str())
        );
        assert_eq!(harvested.trial_id.as_deref(), Some(variation.id.as_str()));
        let song = crate::work::get(&conn, &song.id).unwrap().unwrap();
        assert_eq!(
            song.current_version_id.as_deref(),
            Some(style.id.as_str()),
            "a found sound is put beside the style, not over it"
        );

        let board = trial::board(&conn, &lab.id).unwrap();
        let card = board
            .trials
            .iter()
            .find(|card| card.trial.id == variation.id)
            .unwrap();
        assert!(matches!(
            card.harvest.as_slice(),
            [trial::Harvest::Version { work_id, .. }] if *work_id == song.id
        ));
    }

    /// A phrase is cut from a kept trial into a type its trials are read
    /// against, and nowhere else.
    #[test]
    fn a_phrase_is_cut_from_a_kept_trial() {
        let (conn, profile_id) = fixtures::workspace();
        let lab = experiment(&conn, &profile_id);
        let kept = put(&conn, &lab.id, "amen break, everything clipping");
        judge(&conn, &kept.id, Some(Verdict::Keep)).unwrap();

        let refused = harvest_phrase(
            &conn,
            &kept.id,
            NewStyleBrick {
                type_key: "image-style".into(),
                name: "everything clipping".into(),
                ..NewStyleBrick::default()
            },
        )
        .unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "trial.notAPhraseType");

        let brick = harvest_phrase(
            &conn,
            &kept.id,
            NewStyleBrick {
                type_key: "knob".into(),
                name: String::new(),
                description: Some("everything clipping".into()),
                ..NewStyleBrick::default()
            },
        )
        .unwrap();
        assert_eq!(brick.name, "everything clipping");
        assert_eq!(brick.trial_id.as_deref(), Some(kept.id.as_str()));
    }

    /// A work made of a trial is a song of its own: its style is the trial's,
    /// it is linked to the experiment by the lab, and nothing it is made of
    /// reaches across that link - its folder and its origin are its own.
    #[test]
    fn a_work_made_of_a_trial_stands_on_its_own() {
        let (conn, profile_id) = fixtures::workspace();
        let lab = experiment(&conn, &profile_id);
        let kept = put(&conn, &lab.id, "amen break, fuzz bass");
        judge(&conn, &kept.id, Some(Verdict::Keep)).unwrap();
        let made = harvest_work(&conn, &kept.id, "song", "Found in the lab").unwrap();
        assert_eq!(made.kind, "song");
        let current = crate::work::version::get(&conn, made.current_version_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(current.role, "style");
        assert_eq!(current.trial_id.as_deref(), Some(kept.id.as_str()));

        let links = crate::link::for_work(&conn, &made.id).unwrap();
        assert_eq!(links.sources.len(), 1);
        assert_eq!(links.sources[0].role, crate::link::LAB);
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        assert!(
            crate::publication::origin(&conn, &config, &made.id)
                .unwrap()
                .is_none(),
            "a song found in an experiment is made of nothing a folder could name"
        );
        assert!(crate::link::descendants(&conn, &lab.id).unwrap().is_empty());

        let refused = harvest_work(&conn, &kept.id, "video", "A clip").unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "trial.wrongKind");
    }

    /// An experiment made from a song starts from the song's newest style as
    /// its core, and is linked to it by the lab: the song is not released
    /// because of it, nor is it among the song's publications.
    #[test]
    fn an_experiment_made_from_a_song_starts_from_its_style() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        fixtures::version(&conn, &song.id, "style", "old style");
        let newest = fixtures::version(&conn, &song.id, "style", "indie rock, female vocal");
        let (made, core) = experiment_from(&conn, &song.id, "", "rework").unwrap();
        assert_eq!(made.kind, "experiment");
        assert_eq!(made.title, "Harbour lights");
        let core = core.expect("the song had a style");
        assert_eq!(core.source_version_id.as_deref(), Some(newest.id.as_str()));
        assert_eq!(core.body, "indie rock, female vocal");
        assert_eq!(core.series, "rework");
        assert!(
            crate::link::descendants(&conn, &song.id)
                .unwrap()
                .is_empty()
        );
        let derived = crate::link::for_work(&conn, &song.id).unwrap().derived;
        assert_eq!(derived.len(), 1, "the card still shows the link");

        let video = fixtures::video(&conn, &profile_id, "A clip");
        let refused = experiment_from(&conn, &video.id, "", "rework").unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "trial.noLabFor");
    }

    /// A trial in the trash takes its harvest's memory with it and gives it
    /// back on the way out; an experiment in the trash takes its board.
    #[test]
    fn the_trash_keeps_what_a_trial_became() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let lab = experiment(&conn, &profile_id);
        let kept = put(&conn, &lab.id, "amen break");
        let child = vary(&conn, &kept.id, "slower", None).unwrap();
        judge(&conn, &kept.id, Some(Verdict::Keep)).unwrap();
        let harvested = harvest_into(&conn, &kept.id, &song.id, None, false).unwrap();

        let entry = crate::actions::trash::discard(&conn, Entity::Trial, &kept.id).unwrap();
        let orphan = crate::work::version::get(&conn, &harvested.id)
            .unwrap()
            .unwrap();
        assert_eq!(orphan.trial_id, None);
        assert_eq!(
            trial::get(&conn, &child.id).unwrap().unwrap().parent_id,
            None
        );
        crate::actions::trash::restore(&conn, &entry).unwrap();
        let back = crate::work::version::get(&conn, &harvested.id)
            .unwrap()
            .unwrap();
        assert_eq!(back.trial_id.as_deref(), Some(kept.id.as_str()));

        let entry = crate::actions::trash::discard(&conn, Entity::Work, &lab.id).unwrap();
        assert!(trial::get(&conn, &kept.id).unwrap().is_none());
        crate::actions::trash::restore(&conn, &entry).unwrap();
        assert_eq!(trial::for_work(&conn, &lab.id).unwrap().len(), 2);
        let back = crate::work::version::get(&conn, &harvested.id)
            .unwrap()
            .unwrap();
        assert_eq!(back.trial_id.as_deref(), Some(kept.id.as_str()));
    }

    /// A version whose parent went to the trash after it comes back from the
    /// trash naming no parent, rather than failing on the foreign key.
    #[test]
    fn a_version_comes_back_without_a_parent_that_is_gone() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let first = fixtures::version(&conn, &song.id, "lyrics", "one");
        let second = crate::work::version::create(
            &conn,
            &song.id,
            NewVersion {
                role: "lyrics".into(),
                body: "two".into(),
                parent_version_id: Some(first.id.clone()),
                ..NewVersion::default()
            },
        )
        .unwrap();
        fixtures::version(&conn, &song.id, "lyrics", "three");
        let child_entry =
            crate::actions::trash::discard(&conn, Entity::Version, &second.id).unwrap();
        crate::actions::trash::discard(&conn, Entity::Version, &first.id).unwrap();
        crate::actions::trash::restore(&conn, &child_entry).unwrap();
        let back = crate::work::version::get(&conn, &second.id)
            .unwrap()
            .unwrap();
        assert_eq!(back.parent_version_id, None);
    }

    /// Undoing a trial put on the board takes it to the trash, under its
    /// words and its experiment.
    #[test]
    fn a_trial_put_on_the_board_is_undone_into_the_trash() {
        let (conn, profile_id) = fixtures::workspace();
        let lab = experiment(&conn, &profile_id);
        let one = put(&conn, &lab.id, "amen break");
        let offer = crate::undo::last(&conn).unwrap().unwrap();
        assert_eq!(offer.action, "undo.trial.create");
        crate::undo::undo(&conn, &offer.operation_id).unwrap();
        assert!(trial::get(&conn, &one.id).unwrap().is_none());
        let entry = crate::trash::list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.entity_id == one.id)
            .expect("the trial is in the trash");
        assert_eq!(entry.label, "amen break");
        assert_eq!(entry.work_id.as_deref(), Some(lab.id.as_str()));
    }

    /// A trial is read against the dictionary as the style it would become,
    /// and written from it closed by the experiment's own fields.
    #[test]
    fn a_trial_is_read_and_written_as_the_sound_it_becomes() {
        let (conn, profile_id) = fixtures::workspace();
        crate::style_set::seed(&conn).unwrap();
        let lab = experiment(&conn, &profile_id);
        let check = crate::register::check::version_text(
            &conn,
            &profile_id,
            "chopped amen break, a tag nobody wrote down",
            false,
            &lab,
            "style",
        )
        .unwrap();
        assert_eq!(check.phrases.len(), 1);
        assert_eq!(check.unknown.len(), 1);

        let amen = crate::style_brick::list(&conn, &profile_id, &Default::default())
            .unwrap()
            .into_iter()
            .find(|brick| brick.description.as_deref() == Some("chopped amen break"))
            .unwrap();
        super::super::work::update(
            &conn,
            &lab.id,
            crate::work::WorkPatch {
                meta: Some(
                    serde_json::json!({ "bpm": 170 })
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
                ..crate::work::WorkPatch::default()
            },
        )
        .unwrap();
        let written = crate::phrase::compose::write(
            &conn,
            &profile_id,
            &crate::phrase::compose::ComposeRequest {
                composition: "sound".into(),
                bricks: vec![amen.id],
                work_id: Some(lab.id.clone()),
            },
        )
        .unwrap();
        assert!(
            written.text.starts_with("chopped amen break"),
            "{}",
            written.text
        );
        assert!(written.text.contains("170"), "{}", written.text);
    }

    /// The board marks a trial whose text lost an anchor of the experiment,
    /// and says where a kept trial may go.
    #[test]
    fn the_board_marks_a_lost_anchor() {
        let (conn, profile_id) = fixtures::workspace();
        let lab = experiment(&conn, &profile_id);
        super::super::work::update(
            &conn,
            &lab.id,
            crate::work::WorkPatch {
                meta: Some(
                    serde_json::json!({ "anchors": "fuzz bass\nAmen break\n" })
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
                ..crate::work::WorkPatch::default()
            },
        )
        .unwrap();
        put(&conn, &lab.id, "amen break, fuzz bass");
        put(&conn, &lab.id, "amen break, sub bass");
        let board = trial::board(&conn, &lab.id).unwrap();
        assert_eq!(board.anchors, vec!["fuzz bass", "Amen break"]);
        assert!(board.trials[0].lost_anchors.is_empty());
        assert_eq!(board.trials[1].lost_anchors, vec!["fuzz bass"]);
        assert_eq!(board.harvest_role.as_deref(), Some("style"));
        assert_eq!(board.harvest_kinds, vec!["song"]);
        assert_eq!(board.composition.as_deref(), Some("sound"));
    }
}
