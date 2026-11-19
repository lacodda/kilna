//! The texts of a work a term of another scope is looked for in (v0.151,
//! FR-20 20.2).
//!
//! A track with no words still repeats itself: the third "Night drive" of a
//! month, the fourth cover with a lighthouse, the fifth sound of kora over
//! a sub bass. A term scoped to a title, a picture or a sound is looked for
//! in the matching text of each work made from nothing - the work a title, a
//! cover and a sound are first given to; what is made from it (an audio, a
//! short) carries them on and is not counted again.

use std::collections::BTreeMap;

use rusqlite::{Connection, params};

use super::TermScope;
use crate::error::Result;
use crate::profile::config::ProfileConfig;
use crate::work::Work;

/// The places a scoped term is looked for in, besides the sung text.
pub const PLACES: [TermScope; 3] = [
    TermScope::Title,
    TermScope::VisualMotif,
    TermScope::SoundCombo,
];

/// A work made from nothing, with its texts by place.
#[derive(Debug, Clone)]
pub struct Scoped {
    pub work_id: String,
    pub title: String,
    pub kind: String,
    pub texts: BTreeMap<TermScope, String>,
}

/// The texts of every work of the space made from nothing.
pub fn texts(conn: &Connection, profile_id: &str) -> Result<Vec<Scoped>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let mut statement = conn.prepare(
        "SELECT w.id FROM work w
          WHERE w.profile_id = ?1
            AND NOT EXISTS (SELECT 1 FROM work_link l WHERE l.work_id = w.id)
          ORDER BY w.created_at, w.rowid",
    )?;
    let ids = statement
        .query_map(params![profile_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let Some(work) = crate::work::get(conn, &id)? else {
            continue;
        };
        let texts = PLACES
            .into_iter()
            .map(|place| Ok((place, text_of(conn, &config, &work, place)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        out.push(Scoped {
            work_id: work.id,
            title: work.title,
            kind: work.kind,
            texts,
        });
    }
    Ok(out)
}

/// What `work` says in `place`: its title; what its picture shows; what its
/// sound is made of. The sung text is the register's own reading, elsewhere.
pub fn text_of(
    conn: &Connection,
    config: &ProfileConfig,
    work: &Work,
    place: TermScope,
) -> Result<String> {
    let mut parts: Vec<String> = Vec::new();
    match place {
        TermScope::Title => parts.push(work.title.clone()),
        TermScope::VisualMotif => {
            let cover = &work.cover;
            parts.push(cover.idea.clone());
            parts.push(cover.scene.clone());
            if let Some(sent) = &cover.sent {
                parts.push(sent.picture.clone());
            }
            parts.push(work.frame.still.clone());
        }
        TermScope::SoundCombo => {
            for composition in config
                .compose
                .iter()
                .filter(|one| one.kinds.is_empty() || one.kinds.contains(&work.kind))
            {
                if let Some(version) =
                    crate::work::version::for_role(conn, &work.id, &composition.role)?
                {
                    parts.push(version.body);
                }
            }
        }
        TermScope::Lyrics | TermScope::All => {}
    }
    for field in config
        .fields_of(&work.kind)
        .into_iter()
        .filter(|field| field.register == Some(place))
    {
        match work.meta.get(&field.key) {
            Some(serde_json::Value::String(text)) => parts.push(text.clone()),
            Some(serde_json::Value::Array(values)) => parts.extend(
                values
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned),
            ),
            _ => {}
        }
    }
    Ok(parts
        .into_iter()
        .map(|part| part.trim().to_owned())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use serde_json::json;

    use crate::register::guard::{self, RepeatLevel};
    use crate::register::{NewTerm, Strictness, TermScope};
    use crate::work::{self, NewWork};

    fn channel() -> (Connection, String) {
        let conn = crate::db::open_in_memory().unwrap();
        crate::profile::seed(&conn, "track-channel").unwrap();
        let profile_id = crate::profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn track(
        conn: &Connection,
        profile_id: &str,
        title: &str,
        cover_idea: &str,
        day: Option<&str>,
    ) -> String {
        let made = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "track".into(),
                title: title.into(),
                meta: Some(
                    json!({ "cover_idea": cover_idea })
                        .as_object()
                        .cloned()
                        .unwrap(),
                ),
                ..NewWork::default()
            },
        )
        .unwrap();
        if let Some(day) = day {
            crate::fixtures::release(conn, &made.id, "youtube", Some(day));
        }
        made.id
    }

    fn term(
        conn: &Connection,
        profile_id: &str,
        word: &str,
        scope: TermScope,
        most: Option<u32>,
        days: Option<u32>,
    ) {
        crate::register::create(
            conn,
            profile_id,
            NewTerm {
                word: word.into(),
                strictness: Some(Strictness::Limit),
                scope,
                max_per_month: most,
                cooldown_days: days,
                ..NewTerm::default()
            },
        )
        .unwrap();
    }

    /// A track with no words is read by its title and its picture: a term
    /// scoped to them finds what a lyric term never would, and says where.
    #[test]
    fn an_instrumental_repeats_itself_in_its_title_and_its_picture() {
        let (conn, profile_id) = channel();
        track(
            &conn,
            &profile_id,
            "Night drive",
            "a lighthouse in fog",
            Some("2026-06-01"),
        );
        let fresh = track(
            &conn,
            &profile_id,
            "Night swim",
            "a lighthouse at dawn",
            None,
        );
        term(&conn, &profile_id, "night", TermScope::Title, None, None);
        term(
            &conn,
            &profile_id,
            "lighthouse",
            TermScope::VisualMotif,
            None,
            None,
        );
        term(&conn, &profile_id, "fog", TermScope::Lyrics, None, None);

        let read = guard::of_work(&conn, &fresh, "2026-11-19")
            .unwrap()
            .unwrap();
        let said: Vec<(&str, TermScope, RepeatLevel)> = read
            .findings
            .iter()
            .map(|found| (found.word.as_str(), found.scope, found.level))
            .collect();
        assert!(
            said.contains(&("night", TermScope::Title, RepeatLevel::Orange)),
            "{said:?}"
        );
        assert!(
            said.contains(&("lighthouse", TermScope::VisualMotif, RepeatLevel::Orange)),
            "{said:?}"
        );
        assert_eq!(
            said.len(),
            2,
            "a lyric term finds nothing in a track with no lyric: {said:?}"
        );

        // The register counts the works by the place a term is scoped to.
        let night = crate::register::list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .find(|one| one.word == "night")
            .unwrap();
        assert_eq!(crate::register::uses(&conn, &night.id).unwrap().len(), 2);
    }

    /// A term's limits make a repeat loud: a neighbour within its cooldown,
    /// or more works of the song's month than it allows.
    #[test]
    fn a_cooldown_and_a_months_limit_turn_a_repeat_red() {
        let (conn, profile_id) = channel();
        track(&conn, &profile_id, "Night drive", "", Some("2026-11-02"));
        track(&conn, &profile_id, "Night bus", "", Some("2026-11-20"));
        let fresh = track(&conn, &profile_id, "Night swim", "", Some("2026-11-25"));
        term(&conn, &profile_id, "night", TermScope::Title, Some(2), None);

        let read = guard::of_work(&conn, &fresh, "2026-08-01")
            .unwrap()
            .unwrap();
        assert_eq!(
            read.level,
            Some(RepeatLevel::Red),
            "three in November, two allowed: {read:?}"
        );

        let (conn, profile_id) = channel();
        track(&conn, &profile_id, "Night drive", "", Some("2026-11-02"));
        let fresh = track(&conn, &profile_id, "Night swim", "", None);
        term(
            &conn,
            &profile_id,
            "night",
            TermScope::Title,
            None,
            Some(30),
        );
        let near = guard::of_work(&conn, &fresh, "2026-11-19")
            .unwrap()
            .unwrap();
        assert_eq!(near.level, Some(RepeatLevel::Red), "{near:?}");
        let far = guard::of_work(&conn, &fresh, "2027-03-01")
            .unwrap()
            .unwrap();
        assert_eq!(far.level, Some(RepeatLevel::Orange), "{far:?}");
    }
}
