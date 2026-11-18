//! A shipped profile's entries kept in step with the build, one by one
//! (v0.125, ADR 0087).
//!
//! A profile is copied into a space and is its owner's from then on. The
//! lists in it that kilna goes on shipping - the actions, a kind's statuses,
//! the stage's stops, the marks, the fields, the kinds of note and their
//! sections, the vibe's words - are carried by one rule ([`crate::origin`]):
//! an entry nobody touched takes the newer version, a changed one keeps its
//! owner's words with the newer version offered beside it, a deleted one
//! stays deleted, and one shipped since arrives in its place in the list.
//!
//! What the space was given of each entry is kept in `shipped_entry`, beside
//! the document rather than in it: the document is the owner's to read, edit
//! and pass on, and fingerprints in it would be noise in all three.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::Value;

use super::config::ProfileConfig;
use crate::error::{Error, Result};
use crate::origin::{self, Given, Verdict};

/// One step on the way to a list in the document.
#[derive(Debug, Clone, Copy)]
enum Step {
    /// A field of the object reached so far.
    Field(&'static str),
    /// Into each entry of the list reached so far, by its key: the same
    /// kind in the stored copy and in the shipped one, wherever each stands.
    Each,
}

/// A list in the document whose entries ship one by one, matched by `key`.
struct Collection {
    path: &'static [Step],
    /// Fields of an entry that are a collection of their own: left out of its
    /// fingerprint, and kept as stored when the entry is renewed - a kind of
    /// note renewed does not take its sections back with it.
    nested: &'static [&'static str],
    /// A list its owner writes as a whole: met for the first time holding
    /// entries, the shipped ones it lacks were left out on purpose and stay
    /// out. A kind's statuses, a kind of note's sections, the vibe's words -
    /// a list a person shaped. The actions, marks and fields gain what ships
    /// since, as they always did.
    whole: bool,
}

/// Every list the rule carries, a list before the lists inside its entries:
/// a kind of note that arrives brings its sections, and they are then met
/// as reading as they ship.
const COLLECTIONS: &[Collection] = &[
    Collection {
        path: &[Step::Field("prompts")],
        nested: &[],
        whole: false,
    },
    Collection {
        path: &[
            Step::Field("work_kinds"),
            Step::Each,
            Step::Field("statuses"),
        ],
        nested: &[],
        whole: true,
    },
    Collection {
        path: &[Step::Field("stages")],
        nested: &[],
        whole: false,
    },
    Collection {
        path: &[Step::Field("marks")],
        nested: &[],
        whole: false,
    },
    Collection {
        path: &[Step::Field("work_meta_fields")],
        nested: &[],
        whole: false,
    },
    Collection {
        path: &[Step::Field("note_kinds")],
        nested: &["sections"],
        whole: false,
    },
    Collection {
        path: &[
            Step::Field("note_kinds"),
            Step::Each,
            Step::Field("sections"),
        ],
        nested: &[],
        whole: true,
    },
    Collection {
        path: &[Step::Field("vibe"), Step::Field("moods")],
        nested: &[],
        whole: true,
    },
    Collection {
        path: &[Step::Field("vibe"), Step::Field("characters")],
        nested: &[],
        whole: true,
    },
    Collection {
        path: &[Step::Field("vibe"), Step::Field("complexities")],
        nested: &[],
        whole: true,
    },
];

/// Where the entries of a shipped profile come from.
pub fn origin_of(profile_key: &str) -> String {
    format!("profile:{profile_key}")
}

/// What a space has been given of one profile, entry by entry.
#[derive(Debug, Default, Clone)]
pub struct Ledger {
    given: BTreeMap<(String, String), Given>,
    touched: BTreeSet<(String, String)>,
}

impl Ledger {
    pub fn load(conn: &Connection, profile_id: &str) -> Result<Self> {
        let mut statement = conn.prepare(
            "SELECT collection, source_key, digest, kept FROM shipped_entry WHERE profile_id = ?1",
        )?;
        let given = statement
            .query_map(params![profile_id], |row| {
                Ok((
                    (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                    Given {
                        digest: row.get(2)?,
                        kept: row.get(3)?,
                    },
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Self {
            given,
            touched: BTreeSet::new(),
        })
    }

    /// Whether anything of a list has been given yet.
    fn knows(&self, collection: &str) -> bool {
        self.given.keys().any(|(name, _)| name == collection)
    }

    pub fn get(&self, collection: &str, key: &str) -> Option<&Given> {
        self.given.get(&(collection.to_owned(), key.to_owned()))
    }

    fn set(&mut self, collection: &str, key: &str, given: Option<Given>) {
        let at = (collection.to_owned(), key.to_owned());
        if self.given.get(&at) == given.as_ref() {
            return;
        }
        match given {
            Some(given) => self.given.insert(at.clone(), given),
            None => self.given.remove(&at),
        };
        self.touched.insert(at);
    }

    /// Write what changed. Not a gesture: no person made it, and the log
    /// records what people do - the document it is about is logged when a
    /// person changes it.
    pub fn save(&mut self, conn: &Connection, profile_id: &str, origin: &str) -> Result<()> {
        for at in std::mem::take(&mut self.touched) {
            match self.given.get(&at) {
                Some(given) => conn.execute(
                    "INSERT INTO shipped_entry (profile_id, origin, collection, source_key, digest, kept)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT (profile_id, collection, source_key)
                     DO UPDATE SET origin = ?2, digest = ?5, kept = ?6",
                    params![profile_id, origin, at.0, at.1, given.digest, given.kept],
                )?,
                None => conn.execute(
                    "DELETE FROM shipped_entry WHERE profile_id = ?1 AND collection = ?2 AND source_key = ?3",
                    params![profile_id, at.0, at.1],
                )?,
            };
        }
        Ok(())
    }
}

/// What the rule made of one shipped entry.
#[derive(Debug, Clone)]
pub struct Seen {
    /// The list it is in: `prompts`, `work_kinds/song/statuses`,
    /// `note_kinds/character/sections`.
    pub collection: String,
    pub key: String,
    pub verdict: Verdict,
    /// What the space holds by that key, before the rule.
    pub mine: Option<Value>,
    /// The version this build ships.
    pub standard: Value,
}

/// A list in both documents: its name, and where it stands in each.
struct Place {
    name: String,
    stored: String,
    shipped: String,
}

fn key_of(entry: &Value) -> Option<&str> {
    entry.get("key").and_then(Value::as_str)
}

/// Every place `path` leads to in the shipped document that the stored copy
/// has too. A list the stored copy lacks at the end of the way is made empty,
/// so what ships in it arrives; anything else it lacks on the way - a kind of
/// work it does not have, the vibe before its lines were carried - is not
/// the rule's to make.
fn places(path: &[Step], doc: &mut Value, ship: &Value) -> Vec<Place> {
    let mut out = Vec::new();
    walk(
        path,
        doc,
        ship,
        String::new(),
        String::new(),
        String::new(),
        &mut out,
    );
    out
}

fn walk(
    path: &[Step],
    doc: &mut Value,
    ship: &Value,
    name: String,
    stored: String,
    shipped: String,
    out: &mut Vec<Place>,
) {
    let Some((step, rest)) = path.split_first() else {
        let has_list = doc.pointer(&stored).is_some_and(Value::is_array);
        if has_list && ship.pointer(&shipped).is_some_and(Value::is_array) {
            out.push(Place {
                name,
                stored,
                shipped,
            });
        }
        return;
    };
    match step {
        Step::Field(field) => {
            let shipped = format!("{shipped}/{field}");
            let Some(there) = ship.pointer(&shipped) else {
                return;
            };
            let parent = match doc.pointer_mut(&stored) {
                Some(Value::Object(parent)) => parent,
                _ => return,
            };
            if !parent.contains_key(*field) {
                if !(rest.is_empty() && there.is_array()) {
                    return;
                }
                parent.insert((*field).to_owned(), Value::Array(Vec::new()));
            }
            let name = if name.is_empty() {
                (*field).to_owned()
            } else {
                format!("{name}/{field}")
            };
            walk(
                rest,
                doc,
                ship,
                name,
                format!("{stored}/{field}"),
                shipped,
                out,
            );
        }
        Step::Each => {
            let Some(entries) = ship.pointer(&shipped).and_then(Value::as_array) else {
                return;
            };
            for (shipped_at, entry) in entries.iter().enumerate() {
                let Some(key) = key_of(entry) else {
                    continue;
                };
                let Some(stored_at) = doc
                    .pointer(&stored)
                    .and_then(Value::as_array)
                    .and_then(|list| list.iter().position(|e| key_of(e) == Some(key)))
                else {
                    continue;
                };
                walk(
                    rest,
                    doc,
                    ship,
                    format!("{name}/{key}"),
                    format!("{stored}/{stored_at}"),
                    format!("{shipped}/{shipped_at}"),
                    out,
                );
            }
        }
    }
}

/// An entry's fingerprint, without the lists inside it that ship on their own.
fn fingerprint(entry: &Value, nested: &[&str]) -> String {
    if nested.is_empty() {
        return origin::digest(entry);
    }
    let mut bare = entry.clone();
    if let Some(object) = bare.as_object_mut() {
        for field in nested {
            object.remove(*field);
        }
    }
    origin::digest(&bare)
}

/// The shipped entry as it is written over a stored one: the lists inside it
/// that ship on their own stay as the space has them.
fn renewed(standard: &Value, mine: Option<&Value>, nested: &[&str]) -> Value {
    let mut next = standard.clone();
    if let (Some(next), Some(mine)) = (next.as_object_mut(), mine.and_then(Value::as_object)) {
        for field in nested {
            match mine.get(*field) {
                Some(kept) => next.insert((*field).to_owned(), kept.clone()),
                None => next.remove(*field),
            };
        }
    }
    next
}

/// Carry the rule through `doc` against `ship`, writing what it writes and
/// noting what it gave in `ledger`. Every shipped entry is reported.
fn sync(doc: &mut Value, ship: &Value, ledger: &mut Ledger) -> Vec<Seen> {
    let mut seen = Vec::new();
    for collection in COLLECTIONS {
        for place in places(collection.path, doc, ship) {
            let Some(shipped) = ship.pointer(&place.shipped).and_then(Value::as_array) else {
                continue;
            };
            let first_met = collection.whole && !ledger.knows(&place.name);
            let holds = doc
                .pointer(&place.stored)
                .and_then(Value::as_array)
                .is_some_and(|list| !list.is_empty());
            // A list its owner writes whole, met empty: what first fills it is
            // the steps' to say - a kind of note gains its sections with the
            // rest of what a card is, or not at all - and the rule meets it
            // once it holds something.
            if first_met && !holds {
                continue;
            }
            for (index, standard) in shipped.iter().enumerate() {
                let Some(key) = key_of(standard) else {
                    continue;
                };
                let Some(list) = doc.pointer_mut(&place.stored).and_then(Value::as_array_mut)
                else {
                    continue;
                };
                let at = list.iter().position(|e| key_of(e) == Some(key));
                let mine = at.map(|at| list[at].clone());
                let standard_digest = fingerprint(standard, collection.nested);
                let current = mine.as_ref().map(|m| fingerprint(m, collection.nested));
                let given = ledger.get(&place.name, key).cloned();
                let verdict = if first_met && current.is_none() {
                    Verdict::Deleted
                } else {
                    origin::decide(current.as_deref(), given.as_ref(), &standard_digest)
                };
                match (verdict, at) {
                    (Verdict::Renewed, Some(at)) => {
                        list[at] = renewed(standard, mine.as_ref(), collection.nested);
                    }
                    (Verdict::Added, _) => {
                        // After the nearest entry it follows in the shipped
                        // list that the space still has, as an entry shipped
                        // in the middle of a list belongs there.
                        let after = shipped[..index].iter().rev().find_map(|before| {
                            let before = key_of(before)?;
                            list.iter().position(|e| key_of(e) == Some(before))
                        });
                        let at = after.map_or(list.len(), |position| position + 1);
                        list.insert(at, standard.clone());
                    }
                    _ => {}
                }
                // Left out of a list its owner wrote whole: given as deleted,
                // so it stays out once the rest of the list is known.
                let next = if first_met && current.is_none() {
                    Some(origin::left_out())
                } else {
                    origin::after(verdict, given.as_ref(), &standard_digest)
                };
                ledger.set(&place.name, key, next);
                seen.push(Seen {
                    collection: place.name.clone(),
                    key: key.to_owned(),
                    verdict,
                    mine,
                    standard: standard.clone(),
                });
            }
        }
    }
    seen
}

/// Carry the shipped profile's entries into the stored copy. Returns what
/// the rule made of each; the copy is rewritten only when it wrote one.
pub fn carry(
    config: &mut ProfileConfig,
    shipped: &ProfileConfig,
    ledger: &mut Ledger,
) -> Result<Vec<Seen>> {
    let mut doc = serde_json::to_value(&*config)?;
    let ship = serde_json::to_value(shipped)?;
    let seen = sync(&mut doc, &ship, ledger);
    if seen.iter().any(|one| one.verdict.writes()) {
        *config = serde_json::from_value(doc)?;
    }
    Ok(seen)
}

/// Record what a profile just stored was given, without writing to it: for a
/// space made a moment ago, from what ships or from a type (ADR 0079). An
/// entry reading as it ships is given; one reading otherwise is the type's
/// own word, and is kept over the standard rather than offered against it;
/// one a whole list leaves out stays out. What ships and is missing is left
/// to the carry-forward, which adds it.
pub fn settle(conn: &Connection, profile_id: &str) -> Result<()> {
    let Some((key, stored, shipped)) = both(conn, profile_id)? else {
        return Ok(());
    };
    let before = Ledger::load(conn, profile_id)?;
    let mut after = before.clone();
    let mut doc = serde_json::to_value(&stored)?;
    let ship = serde_json::to_value(&shipped)?;
    let mut ledger = before.clone();
    for one in sync(&mut doc, &ship, &mut after) {
        let first = before.get(&one.collection, &one.key).is_none();
        let next = match one.verdict {
            Verdict::Adopted | Verdict::Deleted if first => {
                after.get(&one.collection, &one.key).cloned()
            }
            Verdict::Offered if first => {
                let nested = collection_of(&one.collection).map_or(&[][..], |c| c.nested);
                Some(origin::kept(None, &fingerprint(&one.standard, nested)))
            }
            _ => continue,
        };
        ledger.set(&one.collection, &one.key, next);
    }
    ledger.save(conn, profile_id, &origin_of(&key))
}

/// A shipped version waiting beside its owner's: what Settings shows to be
/// kept or taken.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct ShippedOffer {
    /// The list it is in: `prompts`, `work_kinds/song/statuses`, `marks`,
    /// `note_kinds/character/sections`, `vibe/moods`.
    pub collection: String,
    pub key: String,
    /// The space's entry.
    #[ts(type = "Record<string, unknown>")]
    pub mine: Value,
    /// The version this build ships.
    #[ts(type = "Record<string, unknown>")]
    pub standard: Value,
}

/// One offer, named: which entry of which list.
#[derive(Debug, Clone, serde::Deserialize, ts_rs::TS)]
pub struct OfferRef {
    pub collection: String,
    pub key: String,
}

/// The profile row and the shipped profile of its key, when kilna ships one.
fn both(
    conn: &Connection,
    profile_id: &str,
) -> Result<Option<(String, ProfileConfig, ProfileConfig)>> {
    let Some(key) = super::key_for_id(conn, profile_id)? else {
        return Err(Error::not_found("profile", profile_id));
    };
    let Some(shipped) = super::builtin()?.into_iter().find(|p| p.key == key) else {
        return Ok(None);
    };
    let stored = super::config_for(conn, profile_id)?;
    Ok(Some((key, stored, shipped.config)))
}

/// The shipped versions waiting beside their owner's in a profile.
pub fn offers(conn: &Connection, profile_id: &str) -> Result<Vec<ShippedOffer>> {
    let Some((_, stored, shipped)) = both(conn, profile_id)? else {
        return Ok(Vec::new());
    };
    let mut ledger = Ledger::load(conn, profile_id)?;
    let mut doc = serde_json::to_value(&stored)?;
    let ship = serde_json::to_value(&shipped)?;
    Ok(sync(&mut doc, &ship, &mut ledger)
        .into_iter()
        .filter(|one| one.verdict == Verdict::Offered)
        .filter_map(|one| {
            Some(ShippedOffer {
                collection: one.collection,
                key: one.key,
                mine: one.mine?,
                standard: one.standard,
            })
        })
        .collect())
}

/// The offer `wanted` names, as the rule sees it now - refused when nothing
/// waits there any more.
fn find(
    conn: &Connection,
    profile_id: &str,
    wanted: &OfferRef,
) -> Result<(String, ProfileConfig, Value, String)> {
    let Some((key, stored, shipped)) = both(conn, profile_id)? else {
        return Err(Error::refused("profile.noOffer"));
    };
    let ship = serde_json::to_value(&shipped)?;
    let mut doc = serde_json::to_value(&stored)?;
    let mut ledger = Ledger::load(conn, profile_id)?;
    let one = sync(&mut doc, &ship, &mut ledger)
        .into_iter()
        .find(|one| {
            one.collection == wanted.collection
                && one.key == wanted.key
                && one.verdict == Verdict::Offered
        })
        .ok_or_else(|| Error::refused("profile.noOffer"))?;
    let nested = collection_of(&one.collection).map_or(&[][..], |c| c.nested);
    let digest = fingerprint(&one.standard, nested);
    Ok((key, stored, one.standard, digest))
}

/// The collection a list's name belongs to: `work_kinds/song/statuses` is
/// one of `work_kinds/*/statuses`.
fn collection_of(name: &str) -> Option<&'static Collection> {
    let parts: Vec<&str> = name.split('/').collect();
    COLLECTIONS.iter().find(|collection| {
        collection.path.len() == parts.len()
            && collection
                .path
                .iter()
                .zip(&parts)
                .all(|(step, part)| match step {
                    Step::Field(field) => field == part,
                    Step::Each => true,
                })
    })
}

/// Take the shipped version over the owner's, as a change to the document
/// they could have made by hand - logged, and taken back by an undo, after
/// which the offer waits again. What the space was given is left alone: the
/// entry now reads as it ships, which the rule meets as untouched, and an
/// undo puts back an entry that differs from what it was given.
pub fn take(conn: &Connection, profile_id: &str, wanted: &OfferRef) -> Result<super::Profile> {
    let (_, stored, standard, _) = find(conn, profile_id, wanted)?;
    let collection =
        collection_of(&wanted.collection).ok_or_else(|| Error::refused("profile.noOffer"))?;
    let mut doc = serde_json::to_value(&stored)?;
    let pointer =
        pointer_to(&doc, &wanted.collection).ok_or_else(|| Error::refused("profile.noOffer"))?;
    let list = doc
        .pointer_mut(&pointer)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| Error::refused("profile.noOffer"))?;
    let at = list
        .iter()
        .position(|e| key_of(e) == Some(wanted.key.as_str()))
        .ok_or_else(|| Error::refused("profile.noOffer"))?;
    list[at] = renewed(&standard, Some(&list[at].clone()), collection.nested);
    let config: ProfileConfig = serde_json::from_value(doc)?;
    let profile = crate::actions::profile::update_config(conn, profile_id, &config)?;
    Ok(profile)
}

/// Keep the owner's entries over the shipped versions waiting beside them:
/// those versions are not offered again, a newer one is.
pub fn keep(conn: &Connection, profile_id: &str, wanted: &[OfferRef]) -> Result<()> {
    let mut ledger = Ledger::load(conn, profile_id)?;
    let mut origin = None;
    for one in wanted {
        let (key, _, _, digest) = find(conn, profile_id, one)?;
        let given = ledger.get(&one.collection, &one.key).cloned();
        ledger.set(
            &one.collection,
            &one.key,
            Some(origin::kept(given.as_ref(), &digest)),
        );
        origin = Some(origin_of(&key));
    }
    if let Some(origin) = origin {
        ledger.save(conn, profile_id, &origin)?;
    }
    Ok(())
}

/// Where a list named like `work_kinds/song/statuses` stands in a document.
fn pointer_to(doc: &Value, name: &str) -> Option<String> {
    let collection = collection_of(name)?;
    let parts: Vec<&str> = name.split('/').collect();
    let mut pointer = String::new();
    for (step, part) in collection.path.iter().zip(parts) {
        match step {
            Step::Field(field) => pointer = format!("{pointer}/{field}"),
            Step::Each => {
                let at = doc
                    .pointer(&pointer)?
                    .as_array()?
                    .iter()
                    .position(|e| key_of(e) == Some(part))?;
                pointer = format!("{pointer}/{at}");
            }
        }
    }
    Some(pointer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile::{self, DEFAULT_KIND};

    fn studio() -> ProfileConfig {
        profile::builtin()
            .unwrap()
            .into_iter()
            .find(|p| p.key == DEFAULT_KIND)
            .unwrap()
            .config
    }

    fn status_label(config: &ProfileConfig, kind: &str, key: &str) -> String {
        config
            .kind(kind)
            .unwrap()
            .statuses
            .iter()
            .find(|s| s.key == key)
            .map(|s| s.label.as_str().to_owned())
            .unwrap_or_default()
    }

    fn relabel(config: &mut ProfileConfig, kind: &str, key: &str, label: &str) {
        let kind = config
            .work_kinds
            .iter_mut()
            .find(|k| k.key == kind)
            .unwrap();
        let status = kind.statuses.iter_mut().find(|s| s.key == key).unwrap();
        status.label = label.into();
    }

    fn verdict(seen: &[Seen], collection: &str, key: &str) -> Verdict {
        seen.iter()
            .find(|one| one.collection == collection && one.key == key)
            .map(|one| one.verdict)
            .unwrap_or_else(|| panic!("{collection}/{key} was not met"))
    }

    #[test]
    fn a_copy_as_it_ships_is_adopted_entry_by_entry_and_then_current() {
        let ships = studio();
        let mut stored = ships.clone();
        let mut ledger = Ledger::default();
        let seen = carry(&mut stored, &ships, &mut ledger).unwrap();
        assert!(
            seen.iter().all(|one| one.verdict == Verdict::Adopted),
            "{seen:?}"
        );
        for list in [
            "prompts",
            "work_kinds/song/statuses",
            "stages",
            "marks",
            "work_meta_fields",
            "note_kinds",
            "vibe/moods",
        ] {
            assert!(
                seen.iter().any(|one| one.collection == list),
                "{list} is carried"
            );
        }
        assert!(seen.iter().any(|one| one.collection.ends_with("/sections")));
        let again = carry(&mut stored, &ships, &mut ledger).unwrap();
        assert!(again.iter().all(|one| one.verdict == Verdict::Current));
        assert_eq!(
            serde_json::to_value(&stored).unwrap(),
            serde_json::to_value(&ships).unwrap()
        );
    }

    #[test]
    fn an_untouched_entry_takes_a_newer_version_and_a_changed_one_is_offered_it() {
        let first = studio();
        let mut stored = first.clone();
        let mut ledger = Ledger::default();
        carry(&mut stored, &first, &mut ledger).unwrap();

        // The owner rewords one status, and nothing newer ships: theirs
        // stands, and nothing is offered.
        relabel(&mut stored, "song", "draft", "Sketch");
        let quiet = carry(&mut stored, &first, &mut ledger).unwrap();
        assert_eq!(
            verdict(&quiet, "work_kinds/song/statuses", "draft"),
            Verdict::Kept
        );

        // The next build rewords that status and another.
        let mut next = first.clone();
        relabel(&mut next, "song", "draft", "Rough draft");
        relabel(&mut next, "song", "scored", "Judged");
        let seen = carry(&mut stored, &next, &mut ledger).unwrap();
        assert_eq!(
            verdict(&seen, "work_kinds/song/statuses", "scored"),
            Verdict::Renewed
        );
        assert_eq!(status_label(&stored, "song", "scored"), "Judged");
        assert_eq!(
            verdict(&seen, "work_kinds/song/statuses", "draft"),
            Verdict::Offered
        );
        assert_eq!(
            status_label(&stored, "song", "draft"),
            "Sketch",
            "the owner's word stays"
        );
    }

    #[test]
    fn a_deleted_entry_stays_deleted_and_one_shipped_since_takes_its_place() {
        let first = studio();
        let mut stored = first.clone();
        let mut ledger = Ledger::default();
        carry(&mut stored, &first, &mut ledger).unwrap();

        let gone = stored.marks[0].key.clone();
        stored.marks.remove(0);
        let mut next = first.clone();
        let mut arriving = next.prompts[1].clone();
        arriving.key = "arriving".into();
        next.prompts.insert(2, arriving);

        let seen = carry(&mut stored, &next, &mut ledger).unwrap();
        assert_eq!(verdict(&seen, "marks", &gone), Verdict::Deleted);
        assert!(
            stored.marks.iter().all(|m| m.key != gone),
            "it does not come back"
        );
        assert_eq!(verdict(&seen, "prompts", "arriving"), Verdict::Added);
        assert_eq!(
            stored.prompts[2].key, "arriving",
            "after the entry it follows in the shipped list"
        );
    }

    #[test]
    fn a_kind_of_note_renewed_keeps_its_sections_as_the_space_has_them() {
        let first = studio();
        let mut stored = first.clone();
        let mut ledger = Ledger::default();
        carry(&mut stored, &first, &mut ledger).unwrap();

        let kind = first
            .note_kinds
            .iter()
            .position(|k| k.sections.len() > 1)
            .expect("a kind of note with sections");
        let key = first.note_kinds[kind].key.clone();
        let section = stored.note_kinds[kind].sections[1].key.clone();
        stored.note_kinds[kind].sections.remove(1);
        let mut next = first.clone();
        next.note_kinds[kind].label = "Renamed".into();

        let seen = carry(&mut stored, &next, &mut ledger).unwrap();
        assert_eq!(verdict(&seen, "note_kinds", &key), Verdict::Renewed);
        assert_eq!(stored.note_kinds[kind].label.as_str(), "Renamed");
        assert!(
            stored.note_kinds[kind]
                .sections
                .iter()
                .all(|s| s.key != section),
            "the section its owner took out stays out"
        );
        assert_eq!(
            verdict(&seen, &format!("note_kinds/{key}/sections"), &section),
            Verdict::Deleted
        );
    }

    #[test]
    fn a_list_its_owner_wrote_whole_keeps_out_what_it_lacked_when_first_met() {
        let ships = studio();
        let mut stored = ships.clone();
        let song = stored
            .work_kinds
            .iter_mut()
            .find(|k| k.key == "song")
            .unwrap();
        let left_out = song.statuses.pop().unwrap().key;
        let mut ledger = Ledger::default();

        let seen = carry(&mut stored, &ships, &mut ledger).unwrap();
        assert_eq!(
            verdict(&seen, "work_kinds/song/statuses", &left_out),
            Verdict::Deleted
        );
        let again = carry(&mut stored, &ships, &mut ledger).unwrap();
        assert_eq!(
            verdict(&again, "work_kinds/song/statuses", &left_out),
            Verdict::Deleted
        );
        assert!(
            stored
                .kind("song")
                .unwrap()
                .statuses
                .iter()
                .all(|s| s.key != left_out)
        );

        // The actions are not written whole: one shipped and missing arrives.
        let mut stored = ships.clone();
        let missing = stored.prompts.remove(0).key;
        let seen = carry(&mut stored, &ships, &mut Ledger::default()).unwrap();
        assert_eq!(verdict(&seen, "prompts", &missing), Verdict::Added);
    }

    #[test]
    fn an_offer_is_taken_as_an_edit_undone_as_one_and_kept_until_a_newer_ships() {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn, DEFAULT_KIND).unwrap();
        let id = profile::id_for_key(&conn, DEFAULT_KIND).unwrap().unwrap();
        assert!(
            offers(&conn, &id).unwrap().is_empty(),
            "a space as it ships offers nothing"
        );

        // A copy whose status reads otherwise than it ships, met before
        // anything was given: offered.
        let mut config = profile::config_for(&conn, &id).unwrap();
        relabel(&mut config, "song", "draft", "Sketch");
        profile::update_config(&conn, &id, &config).unwrap();
        conn.execute(
            "DELETE FROM shipped_entry WHERE collection = 'work_kinds/song/statuses' AND source_key = 'draft'",
            [],
        )
        .unwrap();
        let offered = offers(&conn, &id).unwrap();
        assert_eq!(offered.len(), 1, "{offered:?}");
        assert_eq!(offered[0].collection, "work_kinds/song/statuses");
        assert_eq!(offered[0].mine["label"], "Sketch");

        let wanted = OfferRef {
            collection: offered[0].collection.clone(),
            key: "draft".into(),
        };
        take(&conn, &id, &wanted).unwrap();
        let shipped_label = status_label(&studio(), "song", "draft");
        let label_now = |conn: &Connection| {
            status_label(&profile::config_for(conn, &id).unwrap(), "song", "draft")
        };
        assert_eq!(label_now(&conn), shipped_label);
        assert!(offers(&conn, &id).unwrap().is_empty());
        assert_eq!(
            take(&conn, &id, &wanted)
                .unwrap_err()
                .refusal()
                .map(|r| r.code),
            Some("profile.noOffer"),
            "taken once, nothing waits"
        );

        // Undone: the owner's word is back, and the offer with it.
        let last = crate::undo::last(&conn)
            .unwrap()
            .expect("taking can be undone");
        crate::undo::undo(&conn, &last.operation_id).unwrap();
        assert_eq!(label_now(&conn), "Sketch");
        assert_eq!(offers(&conn, &id).unwrap().len(), 1);

        // Kept: not offered again, and the owner's word stays at a start.
        keep(&conn, &id, std::slice::from_ref(&wanted)).unwrap();
        assert!(offers(&conn, &id).unwrap().is_empty());
        profile::seed(&conn, DEFAULT_KIND).unwrap();
        assert_eq!(label_now(&conn), "Sketch");
        assert!(offers(&conn, &id).unwrap().is_empty());
    }
}
