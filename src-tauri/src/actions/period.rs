//! A period laid out in one gesture (v0.152, FR-20 20.3): a collection for
//! each month and a draft for each day, planned on it.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::collection::{self, NewCollection};
use crate::db::unit::atomically;
use crate::error::{Error, Result};
use crate::work::NewWork;
use crate::work_plan::NewWorkPlan;

/// One draft to make: the day it is planned on and its working title.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct PeriodDraft {
    /// `YYYY-MM-DD`.
    pub date: String,
    pub title: String,
}

/// One month to lay out: the collection it gathers in and its drafts.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct PeriodMonthLayout {
    /// `YYYY-MM`.
    pub month: String,
    /// The collection's title: a collection of the kind with this title is
    /// used when the space has one, made otherwise.
    pub title: String,
    pub drafts: Vec<PeriodDraft>,
}

/// What laying a period out makes.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct PeriodLayout {
    /// The kind of work each draft is.
    pub kind: String,
    /// The kind of collection a month gathers in; none gathers nothing.
    #[serde(default)]
    pub collection_kind: Option<String>,
    pub months: Vec<PeriodMonthLayout>,
}

/// What was made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct PeriodMade {
    pub collections: u32,
    pub works: u32,
}

/// Lay a period out: for each month its collection - found by kind and title,
/// or made - and for each day a draft of `kind` in it, planned on that day.
/// One unit, so one undo takes the whole period back; a day refused - a day
/// that is not one - refuses the period, and nothing is made.
pub fn apply(conn: &Connection, layout: PeriodLayout) -> Result<PeriodMade> {
    let profile_id = super::active_profile_id(conn)?;
    let config = crate::profile::config_for(conn, &profile_id)?;
    config.require_kind(&layout.kind)?;
    if let Some(kind) = &layout.collection_kind
        && !config
            .collection_kinds
            .iter()
            .any(|known| known.key == *kind)
    {
        return Err(Error::refused("period.unknownCollectionKind").param("kind", kind.clone()));
    }
    for month in &layout.months {
        crate::period::month_of(&month.month)?;
        for draft in &month.drafts {
            if !crate::time::is_date(&draft.date) || !draft.date.starts_with(&month.month) {
                return Err(Error::refused("period.badDay").param("day", draft.date.clone()));
            }
            if draft.title.trim().is_empty() {
                return Err(Error::refused("period.needsTitle").param("day", draft.date.clone()));
            }
        }
    }
    atomically(conn, |tx| {
        let mut made = PeriodMade {
            collections: 0,
            works: 0,
        };
        for month in &layout.months {
            if month.drafts.is_empty() {
                continue;
            }
            let gathered = match &layout.collection_kind {
                Some(kind) => {
                    let title = month.title.trim();
                    let found = collection::list(tx, &profile_id)?
                        .into_iter()
                        .find(|one| one.kind == *kind && one.title.trim() == title);
                    Some(match found {
                        Some(found) => found.id,
                        None => {
                            made.collections += 1;
                            let mut meta = serde_json::Map::new();
                            meta.insert(
                                "month".into(),
                                serde_json::Value::String(month.month.clone()),
                            );
                            super::collection::create(
                                tx,
                                NewCollection {
                                    kind: kind.clone(),
                                    title: title.to_owned(),
                                    description: None,
                                    meta: Some(meta),
                                    // A month means to hold what the
                                    // rhythm's target says (FR-20 20.1).
                                    target_size: config
                                        .rhythm
                                        .as_ref()
                                        .and_then(|rhythm| rhythm.monthly_target)
                                        .map(i64::from),
                                    due_on: None,
                                },
                            )?
                            .id
                        }
                    })
                }
                None => None,
            };
            for draft in &month.drafts {
                let work = super::work::create(
                    tx,
                    NewWork {
                        kind: layout.kind.clone(),
                        title: draft.title.trim().to_owned(),
                        collection_id: gathered.clone(),
                        ..NewWork::default()
                    },
                )?;
                super::work_plan::plan(
                    tx,
                    NewWorkPlan {
                        work_id: work.id,
                        scheduled_at: draft.date.clone(),
                        note: None,
                    },
                )?;
                made.works += 1;
            }
        }
        Ok(made)
    })
}
