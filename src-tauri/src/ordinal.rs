//! A work's number among the works of its kind made from the same origin -
//! the 2 of "Song (short 2)" (ADR 0070).
//!
//! The number used to live only in the title, written once when the short was
//! made. A folder named after it (`shorts/v2`) then had nothing to read but a
//! title a person may rename, so the number became a fact of the work: given
//! when a work is made from another, read by `{n}` in a title and in a folder.
//!
//! **Past the greatest, never into a gap** - ADR 0059's rule for codes, and
//! for the same reason: a number worked out from an order would change when a
//! sibling before it was deleted, and the folder named after it would be
//! another work's. The greatest is read off the live siblings and off the
//! siblings in the trash, whose snapshots keep the row and its links.

use std::collections::BTreeSet;

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::profile::config::{Label, ProfileConfig, SOURCE_LOCALE};
use crate::work::Work;

/// What a work of `kind` made from `source` is all made from: the source
/// itself when it never goes out (a song), or what the source stands on (the
/// song, for a short cut from its clip). The family a number counts in.
pub fn root_of(conn: &Connection, config: &ProfileConfig, source: &Work) -> Result<Work> {
    Ok(if config.vocabulary(&source.kind).has_doors() {
        crate::publication::origin(conn, config, &source.id)?.unwrap_or_else(|| source.clone())
    } else {
        source.clone()
    })
}

/// The live works of `kind` made from `root`, anywhere down the chain.
pub fn siblings(conn: &Connection, root: &Work, kind: &str) -> Result<Vec<Work>> {
    let mut found = Vec::new();
    for (id, _) in crate::link::descendants(conn, &root.id)? {
        if let Some(work) = crate::work::get(conn, &id)?
            && work.kind == kind
        {
            found.push(work);
        }
    }
    Ok(found)
}

/// The numbers given to works of `kind` made from `root` that lie in the
/// trash: a work put back finds its number still its own.
fn in_the_trash(conn: &Connection, root: &Work, kind: &str) -> Result<Vec<i64>> {
    let mut family: BTreeSet<String> = crate::link::descendants(conn, &root.id)?
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    family.insert(root.id.clone());

    let mut statement =
        conn.prepare("SELECT snapshot FROM deletion WHERE profile_id = ?1 AND entity = 'work'")?;
    let snapshots = statement
        .query_map(params![root.profile_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut numbers = Vec::new();
    for raw in snapshots {
        // A snapshot that does not read holds no number; it is not a reason
        // to refuse making a work.
        let Ok(snapshot) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let rows = |table: &str| {
            snapshot
                .get(table)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        let links = rows("work_link");
        for row in rows("work") {
            let text = |key: &str| row.get(key).and_then(Value::as_str);
            let (Some(id), Some(number)) = (text("id"), row.get("ordinal").and_then(Value::as_i64))
            else {
                continue;
            };
            if text("kind") != Some(kind) {
                continue;
            }
            let made_here = links.iter().any(|link| {
                link.get("work_id").and_then(Value::as_str) == Some(id)
                    && link.get("role").and_then(Value::as_str) != Some(crate::link::LAB)
                    && link
                        .get("source_id")
                        .and_then(Value::as_str)
                        .is_some_and(|source| family.contains(source))
            });
            if made_here {
                numbers.push(number);
            }
        }
    }
    Ok(numbers)
}

/// The number the next work of `kind` made from `root` is given: one past the
/// greatest given so far - live or in the trash - and never below the count of
/// live siblings, so a family made before numbers existed does not start
/// again at one. `except` is the work being numbered, when it already stands
/// in the family: it is not its own elder.
pub fn next(conn: &Connection, root: &Work, kind: &str, except: Option<&str>) -> Result<i64> {
    let siblings: Vec<Work> = siblings(conn, root, kind)?
        .into_iter()
        .filter(|sibling| Some(sibling.id.as_str()) != except)
        .collect();
    let greatest = siblings
        .iter()
        .filter_map(|work| work.ordinal)
        .chain(in_the_trash(conn, root, kind)?)
        .max()
        .unwrap_or(0);
    Ok(greatest.max(siblings.len() as i64) + 1)
}

/// Refuse a number a work cannot hold: one below one, or one a live sibling
/// already holds - two works under one number would share one folder.
pub fn check(conn: &Connection, work: &Work, number: i64) -> Result<()> {
    if number < 1 {
        return Err(Error::refused("work.ordinalBelowOne").param("number", number));
    }
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let Some(root) = crate::publication::origin(conn, &config, &work.id)? else {
        return Err(
            Error::refused("work.ordinalMadeFromNothing").param("title", work.title.clone())
        );
    };
    if let Some(holder) = siblings(conn, &root, &work.kind)?
        .into_iter()
        .find(|sibling| sibling.id != work.id && sibling.ordinal == Some(number))
    {
        return Err(Error::refused("work.ordinalTaken")
            .param("number", number)
            .param("title", holder.title));
    }
    Ok(())
}

/// Give the work a link says was made from another its number, when it has
/// none: past the greatest of its kind in the family the link puts it in. A
/// link of the lab is not "made from" and gives nothing. Returns the number
/// given.
pub fn give_if_missing(conn: &Connection, link: &crate::link::Link) -> Result<Option<i64>> {
    if link.role == crate::link::LAB {
        return Ok(None);
    }
    let Some(work) = crate::work::get(conn, &link.work_id)? else {
        return Ok(None);
    };
    if work.ordinal.is_some() {
        return Ok(None);
    }
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let Some(root) = crate::publication::origin(conn, &config, &work.id)? else {
        return Ok(None);
    };
    let number = next(conn, &root, &work.kind, Some(&work.id))?;
    write(conn, &work.id, number)?;
    Ok(Some(number))
}

/// Write a number given elsewhere - by a gesture, or by the log it left.
pub fn write(conn: &Connection, work_id: &str, number: i64) -> Result<()> {
    conn.execute(
        "UPDATE work SET ordinal = ?2 WHERE id = ?1",
        params![work_id, number],
    )?;
    Ok(())
}

/// The number a work's title says it has, under its kind's `made_title`: the
/// 2 of "Song (short 2)", the 1 of "Song (short)". `None` when the title is
/// not one the template makes - a work renamed by hand.
fn number_in_title(config: &ProfileConfig, work: &Work, root: &Work) -> Option<i64> {
    let vocabulary = config.vocabulary(&work.kind);
    let locales: Vec<&str> = match vocabulary.made_title.as_ref()? {
        Label::One(_) => vec![SOURCE_LOCALE],
        Label::PerLocale(words) => words.keys().map(String::as_str).collect(),
    };
    (1..=999).find_map(|number: usize| {
        locales
            .iter()
            .any(|locale| vocabulary.title_made_from(&root.title, locale, number) == work.title)
            .then_some(number as i64)
    })
}

/// Give every work made from another before v0.101 the number its title
/// carries - "Song (short 2)" is 2 - and a work whose title carries none the
/// next one past the greatest, in the order the works were made.
///
/// Runs when a workspace opens and touches only works made from another that
/// have no number, so it does its work once. Not written to the operation log,
/// for ADR 0030's reason: it is what this version means by a number, not
/// something a person did. Returns how many works were numbered.
pub fn upgrade(conn: &mut Connection) -> Result<usize> {
    let pending: Vec<(String, String)> = conn
        .prepare(
            "SELECT DISTINCT w.id, w.profile_id FROM work w
               JOIN work_link l ON l.work_id = w.id AND l.role <> ?1
              WHERE w.ordinal IS NULL
              ORDER BY w.created_at, w.title, w.rowid",
        )?
        .query_map(params![crate::link::LAB], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    if pending.is_empty() {
        return Ok(0);
    }

    let tx = conn.transaction()?;
    // Two passes: the numbers the titles carry first, so a work whose title
    // says nothing does not take a number a later sibling's title claims.
    let mut untitled = Vec::new();
    let mut numbered = 0;
    for (id, profile_id) in &pending {
        let config = crate::profile::config_for(&tx, profile_id)?;
        let Some(work) = crate::work::get(&tx, id)? else {
            continue;
        };
        let Some(root) = crate::publication::origin(&tx, &config, id)? else {
            continue;
        };
        let claimed = number_in_title(&config, &work, &root).filter(|number| {
            !siblings(&tx, &root, &work.kind)
                .map(|all| all.iter().any(|sibling| sibling.ordinal == Some(*number)))
                .unwrap_or(true)
        });
        match claimed {
            Some(number) => {
                tx.execute(
                    "UPDATE work SET ordinal = ?2 WHERE id = ?1",
                    params![id, number],
                )?;
                numbered += 1;
            }
            None => untitled.push((work, root)),
        }
    }
    for (work, root) in untitled {
        let number = next(&tx, &root, &work.kind, Some(&work.id))?;
        tx.execute(
            "UPDATE work SET ordinal = ?2 WHERE id = ?1",
            params![work.id, number],
        )?;
        numbered += 1;
    }
    tx.commit()?;
    Ok(numbered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions;
    use crate::fixtures;

    fn a_song(conn: &Connection, profile_id: &str, title: &str) -> Work {
        fixtures::work(conn, profile_id, "song", title)
    }

    /// The number a derived work is given is the one its title says, and a
    /// short cut from the audio counts among the shorts cut from the clip.
    #[test]
    fn a_made_work_is_numbered_as_its_title_says() {
        let (conn, profile_id) = fixtures::workspace();
        let song = a_song(&conn, &profile_id, "Tide");
        let clip = actions::work::derive(&conn, &song.id, "video", None, None, None).unwrap();
        let audio = actions::work::derive(&conn, &song.id, "audio", None, None, None).unwrap();
        let first = actions::work::derive(&conn, &clip.work.id, "short", None, None, None).unwrap();
        let second =
            actions::work::derive(&conn, &audio.work.id, "short", None, None, None).unwrap();

        assert_eq!(clip.work.ordinal, Some(1), "the first clip of the song");
        assert_eq!(first.work.title, "Tide (short)");
        assert_eq!(first.work.ordinal, Some(1));
        assert_eq!(second.work.title, "Tide (short 2)");
        assert_eq!(
            second.work.ordinal,
            Some(2),
            "a short of the audio counts among the shorts of the clip"
        );
    }

    /// A number is never given twice: a sibling deleted leaves a gap, and the
    /// next short goes past the greatest - the one in the trash included.
    #[test]
    fn a_number_is_given_past_the_greatest_never_into_a_gap() {
        let (conn, profile_id) = fixtures::workspace();
        let song = a_song(&conn, &profile_id, "Tide");
        let clip = actions::work::derive(&conn, &song.id, "video", None, None, None).unwrap();
        let mut shorts = Vec::new();
        for _ in 0..3 {
            shorts.push(
                actions::work::derive(&conn, &clip.work.id, "short", None, None, None)
                    .unwrap()
                    .work,
            );
        }

        // The last one goes to the trash: its number is still its own.
        actions::trash::discard(&conn, crate::trash::Entity::Work, &shorts[2].id).unwrap();
        let next = actions::work::derive(&conn, &clip.work.id, "short", None, None, None)
            .unwrap()
            .work;
        assert_eq!(
            next.ordinal,
            Some(4),
            "the number in the trash is not given again"
        );
        assert_eq!(next.title, "Tide (short 4)");
    }

    /// A number set by hand is refused below one and when a sibling holds it.
    #[test]
    fn a_number_a_sibling_holds_is_refused() {
        let (conn, profile_id) = fixtures::workspace();
        let song = a_song(&conn, &profile_id, "Tide");
        let clip = actions::work::derive(&conn, &song.id, "video", None, None, None).unwrap();
        let first = actions::work::derive(&conn, &clip.work.id, "short", None, None, None)
            .unwrap()
            .work;
        let second = actions::work::derive(&conn, &clip.work.id, "short", None, None, None)
            .unwrap()
            .work;

        let taken = check(&conn, &second, 1).expect_err("the first short holds 1");
        assert_eq!(taken.refusal().map(|r| r.code), Some("work.ordinalTaken"));
        assert!(check(&conn, &second, 0).is_err(), "nothing is number zero");
        assert!(
            check(&conn, &second, 7).is_ok(),
            "a free number is the work's"
        );
        assert!(
            check(&conn, &first, 1).is_ok(),
            "its own number is free to it"
        );
        assert!(
            check(&conn, &song, 1).is_err(),
            "a work made from nothing has no family to be numbered in"
        );
    }

    /// The works made before numbers existed take the number their titles
    /// carry; one renamed by hand takes the next past the greatest.
    #[test]
    fn the_works_made_before_take_the_number_their_title_carries() {
        let (mut conn, profile_id) = fixtures::workspace();
        let song = a_song(&conn, &profile_id, "Tide");
        let clip = actions::work::derive(&conn, &song.id, "video", None, None, None).unwrap();
        let mut shorts = Vec::new();
        for _ in 0..3 {
            shorts.push(
                actions::work::derive(&conn, &clip.work.id, "short", None, None, None)
                    .unwrap()
                    .work,
            );
        }
        // As a workspace from before v0.101 has them: no numbers, the second
        // short made after the third, and one renamed by hand.
        conn.execute("UPDATE work SET ordinal = NULL", []).unwrap();
        conn.execute(
            "UPDATE work SET title = 'The hook' WHERE id = ?1",
            params![shorts[0].id],
        )
        .unwrap();
        conn.execute(
            "UPDATE work SET created_at = '2099-01-01T00:00:00.000Z' WHERE id = ?1",
            params![shorts[1].id],
        )
        .unwrap();

        let numbered = upgrade(&mut conn).unwrap();
        assert_eq!(numbered, 4, "the clip and three shorts");

        let ordinal = |id: &str| crate::work::get(&conn, id).unwrap().unwrap().ordinal;
        assert_eq!(ordinal(&shorts[1].id), Some(2), "\"Tide (short 2)\" is 2");
        assert_eq!(ordinal(&shorts[2].id), Some(3), "\"Tide (short 3)\" is 3");
        assert_eq!(
            ordinal(&shorts[0].id),
            Some(4),
            "a title that says nothing takes the next past the greatest"
        );
        assert_eq!(ordinal(&song.id), None, "a song is made from nothing");
        assert_eq!(upgrade(&mut conn).unwrap(), 0, "and it runs once");
    }
}
