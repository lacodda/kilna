//! The checklist a month's proposal is read against before "Apply all"
//! (v0.153, FR-20 20.4): the space's quotas over the month as it would stand,
//! the bank's items it takes that are resting or locked, and the register's
//! spent terms its texts say - each read from this space's own data, no
//! number of any channel in the code.

use rusqlite::Connection;
use serde::Serialize;

use crate::assistant::proposal::MonthItem;
use crate::canon::bank::{self, BankState};
use crate::canon::quota::{self, QuotaReport, QuotaSubject};
use crate::error::Result;
use crate::register::TermScope;
use crate::register::matching::{Matcher, Stems};

/// An item of the bank a proposed work takes while it is not free.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct BankTaken {
    pub work_id: String,
    pub title: String,
    pub item: String,
    pub state: BankState,
    /// Resting: the month it is free again.
    pub until: Option<String>,
}

/// A spent term of the register a proposed text says, in the place it is
/// looked for in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct TermSaid {
    pub work_id: String,
    pub title: String,
    pub word: String,
    pub scope: TermScope,
}

/// What a month's proposal is read against before it is applied.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct MonthCheck {
    pub quotas: QuotaReport,
    pub bank: Vec<BankTaken>,
    pub register: Vec<TermSaid>,
}

/// Read a month's proposal: its works as they would stand.
pub fn check(
    conn: &Connection,
    collection_id: Option<String>,
    month: Option<String>,
    items: &[MonthItem],
) -> Result<MonthCheck> {
    let proposed: quota::Proposed = items
        .iter()
        .map(|item| (item.work_id.clone(), item.proposed()))
        .collect();
    let quotas = quota::check_with(
        conn,
        &QuotaSubject {
            work_id: None,
            collection_id,
            month,
        },
        &proposed,
    )?;

    let today = bank::today();
    let mut taken = Vec::new();
    for item in items {
        let title = item
            .title
            .clone()
            .unwrap_or_else(|| item.work_title.clone());
        for fact_id in &item.bank {
            let Some(fact) = crate::canon::fact::get(conn, fact_id)? else {
                continue;
            };
            let (state, until) = bank::state(&fact.data, today);
            if state != BankState::Available {
                taken.push(BankTaken {
                    work_id: item.work_id.clone(),
                    title: title.clone(),
                    item: fact.body.clone(),
                    state,
                    until,
                });
            }
        }
    }

    let profile_id = crate::actions::active_profile_id(conn)?;
    let config = crate::profile::config_for(conn, &profile_id)?;
    let spent = crate::register::spent(conn, &profile_id)?;
    let mut stems = Stems::default();
    let matcher = Matcher::new(&spent, &mut stems);
    let mut said = Vec::new();
    for item in items {
        let Some(work) = crate::work::get(conn, &item.work_id)? else {
            continue;
        };
        let title = item.title.clone().unwrap_or_else(|| work.title.clone());
        // Each proposed text in the place the register reads it: the title,
        // a sung role, a composition's role, a field marked for a place.
        let mut texts: Vec<(TermScope, String)> = Vec::new();
        if let Some(renamed) = &item.title {
            texts.push((TermScope::Title, renamed.clone()));
        }
        let kind = config.vocabulary(&work.kind);
        for version in &item.versions {
            if kind
                .version_roles
                .iter()
                .any(|role| role.key == version.role && role.sung)
            {
                texts.push((TermScope::Lyrics, version.body.clone()));
            }
            if config.compose.iter().any(|one| {
                one.role == version.role && (one.kinds.is_empty() || one.kinds.contains(&work.kind))
            }) {
                texts.push((TermScope::SoundCombo, version.body.clone()));
            }
        }
        for field in config.fields_of(&work.kind) {
            let (Some(place), Some(value)) = (field.register, item.fields.get(&field.key)) else {
                continue;
            };
            if let Some(text) = value.as_str() {
                texts.push((place, text.to_owned()));
            }
        }
        for (place, text) in texts {
            for hit in matcher.find(&text, &mut stems) {
                let term = &spent[hit.term];
                if !term.scope.reads(place) {
                    continue;
                }
                let found = TermSaid {
                    work_id: work.id.clone(),
                    title: title.clone(),
                    word: term.word.clone(),
                    scope: place,
                };
                if !said.contains(&found) {
                    said.push(found);
                }
            }
        }
    }

    Ok(MonthCheck {
        quotas,
        bank: taken,
        register: said,
    })
}
