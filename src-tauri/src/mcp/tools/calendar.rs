//! What goes out and when: the calendar, the rules a month is checked by, a
//! month's drafts and the text a release goes out under.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::assistant::proposal::{self, PackagedVersion, Proposal};
use crate::error::{Error, Result};
use crate::mcp::call::{Call, arg, find_work, pretty, required};
use crate::mcp::deliver::deliver;
use crate::note::NoteFilter;
use crate::work;
use crate::{profile, release};

/// `calendar`: the releases with a date, the days closed, the planned slots
/// and, when asked, what the neighbouring spaces put out on the same days.
pub(super) fn calendar(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let from = arg(args, "from").map(str::to_owned);
    let releases: Vec<release::ScheduledRelease> = release::calendar(conn, &profile.id)?
        .into_iter()
        .filter(|r| {
            from.as_deref().is_none_or(|day| {
                r.release
                    .scheduled_at
                    .as_deref()
                    .is_some_and(|at| at >= day)
            })
        })
        .collect();
    // A span still running on `from` is part of what lies ahead.
    let closed: Vec<crate::time_off::TimeOff> = crate::time_off::list(conn, &profile.id)?
        .into_iter()
        .filter(|span| {
            from.as_deref()
                .is_none_or(|day| span.ends_on.as_str() >= day)
        })
        .collect();
    // The planned slots (v0.138): a work meant for a day before
    // anything of it can go out.
    let window = config.plan_window();
    let mut plans = Vec::new();
    for plan in crate::work_plan::open(conn, &profile.id, window)? {
        if from
            .as_deref()
            .is_some_and(|day| plan.scheduled_at.as_str() < day)
        {
            continue;
        }
        let Some(work) = work::get(conn, &plan.work_id)? else {
            continue;
        };
        let made = crate::link::derived(conn, &work.id)?.len();
        plans.push(json!({
            "id": plan.id, "work_id": work.id, "title": work.title, "kind": work.kind,
            "stage": work.stage, "scheduled_at": plan.scheduled_at, "note": plan.note,
            "derivative_count": made,
        }));
    }
    // The neighbours on the same days (v0.151, FR-20 20.2).
    let mut days: BTreeSet<String> = releases
        .iter()
        .filter_map(|r| r.release.scheduled_at.as_deref())
        .map(|at| at.get(..10).unwrap_or(at).to_owned())
        .collect();
    days.extend(
        plans
            .iter()
            .filter_map(|plan| plan["scheduled_at"].as_str().map(str::to_owned)),
    );
    let wanted: Option<Vec<String>> = match args.get("cross_spaces") {
        Some(Value::Bool(true)) => Some(Vec::new()),
        Some(Value::Array(names)) => Some(
            names
                .iter()
                .filter_map(Value::as_str)
                .map(|name| name.trim().to_owned())
                .collect(),
        ),
        _ => None,
    };
    let mut answer = json!({ "releases": releases, "closed": closed, "work_plans": plans });
    if let Some(wanted) = wanted {
        let neighbours = session
            .space
            .as_ref()
            .map(|space| space.neighbours.clone())
            .unwrap_or_default();
        let mut found = Vec::new();
        for (name, database) in neighbours
            .iter()
            .filter(|(name, _)| wanted.is_empty() || wanted.contains(name))
        {
            found.extend(neighbour_days(name, database, &days)?);
        }
        answer["cross_spaces"] = Value::Array(found);
    }
    pretty(&answer)
}

/// What a neighbouring space puts out or plans on `days`, in the compact
/// fields `calendar`'s `cross_spaces` gives (v0.151, FR-20 20.2): the space,
/// the day, the title, the cover's idea, the genre and the mood - nothing of
/// its texts, its canon or its people. Read from its database opened for
/// reading only; a space that cannot be read says nothing.
fn neighbour_days(
    name: &str,
    database: &std::path::Path,
    days: &BTreeSet<String>,
) -> Result<Vec<Value>> {
    if days.is_empty() {
        return Ok(Vec::new());
    }
    let Ok(conn) = crate::db::open_read_only(database) else {
        return Ok(Vec::new());
    };
    let Some(profile) = profile::active(&conn)? else {
        return Ok(Vec::new());
    };
    let mut dated: Vec<(String, String)> = Vec::new();
    for entry in release::calendar(&conn, &profile.id)? {
        if let Some(at) = entry.release.scheduled_at.as_deref() {
            dated.push((
                at.get(..10).unwrap_or(at).to_owned(),
                entry.release.work_id.clone(),
            ));
        }
    }
    let config = profile::config_for(&conn, &profile.id)?;
    for plan in crate::work_plan::open(&conn, &profile.id, config.plan_window())? {
        dated.push((plan.scheduled_at.clone(), plan.work_id.clone()));
    }
    let mut out = Vec::new();
    for (day, work_id) in dated {
        if !days.contains(&day) {
            continue;
        }
        let Some(work) = work::get(&conn, &work_id)? else {
            continue;
        };
        let text = |key: &str| {
            work.meta
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        let cover_idea = text("cover_idea")
            .or_else(|| (!work.cover.idea.trim().is_empty()).then(|| work.cover.idea.clone()));
        out.push(json!({
            "space_name": name,
            "date": day,
            "title": work.title,
            "cover_idea": cover_idea,
            "genre": text("genre"),
            "mood": text("mood"),
        }));
    }
    Ok(out)
}

/// `quotas`: the space's rules checked over a work, a collection, a month or
/// the whole space.
pub(super) fn quotas(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        args,
        ..
    } = *call;
    let work_id = match arg(args, "work") {
        Some(named) => Some(find_work(conn, &profile.id, named)?.id),
        None => None,
    };
    let collection_id = match arg(args, "collection") {
        Some(named) => {
            let named = named.trim();
            let found = crate::collection::list(conn, &profile.id)?
                .into_iter()
                .find(|one| one.id == named || one.title.trim() == named)
                .ok_or_else(|| Error::not_found("collection", named))?;
            Some(found.id)
        }
        None => None,
    };
    let subject = crate::canon::quota::QuotaSubject {
        work_id,
        collection_id,
        month: arg(args, "month").map(str::to_owned),
    };
    pretty(&crate::canon::quota::check(conn, &subject)?)
}

/// `propose_batch_prompts`: a month's drafts written in one pass, as one
/// proposal.
pub(super) fn propose_batch_prompts(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let collection_id = match arg(args, "collection") {
        Some(named) => {
            let named = named.trim();
            let found = crate::collection::list(conn, &profile.id)?
                .into_iter()
                .find(|one| one.id == named || one.title.trim() == named)
                .ok_or_else(|| Error::not_found("collection", named))?;
            Some(found.id)
        }
        None => None,
    };
    let month = arg(args, "month").map(|month| month.trim().to_owned());
    if let Some(month) = &month {
        crate::period::month_of(month)?;
    }
    // The bank's items by name, case aside, across the space's banks.
    let mut items_by_name: Vec<(String, String)> = Vec::new();
    for kind in config.note_kinds.iter().filter(|kind| kind.is_card()) {
        for section in kind
            .sections
            .iter()
            .filter(|section| section.shape == crate::profile::config::SectionShape::Bank)
        {
            for card in crate::note::list(
                conn,
                &profile.id,
                &NoteFilter {
                    kind: Some(kind.key.clone()),
                    ..NoteFilter::default()
                },
            )? {
                for fact in crate::canon::fact::in_section(conn, &card.id, &section.key)? {
                    items_by_name.push((fact.body.trim().to_lowercase(), fact.id));
                }
            }
        }
    }
    let mut items = Vec::new();
    let mut left_out: Vec<String> = Vec::new();
    let mut body = String::new();
    for raw in args
        .get("works")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let raw = raw.as_object().cloned().unwrap_or_default();
        let found = find_work(conn, &profile.id, required(&raw, "work")?)?;
        let (fields, unknown) = proposal::fields_from(
            raw.get("fields")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
            config,
        );
        left_out.extend(
            unknown
                .into_iter()
                .map(|key| format!("{} ({key})", found.title)),
        );
        let mut versions = Vec::new();
        for version in raw
            .get("versions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let version = version.as_object().cloned().unwrap_or_default();
            let role = required(&version, "role")?;
            config.require_role(&found.kind, role)?;
            versions.push(PackagedVersion {
                role: role.to_owned(),
                body: required(&version, "body")?.to_owned(),
                label: None,
                from: None,
            });
        }
        let mut bank = Vec::new();
        for name in raw
            .get("bank")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let name = name.as_str().unwrap_or_default().trim();
            match items_by_name
                .iter()
                .find(|(known, _)| *known == name.to_lowercase())
            {
                Some((_, id)) => bank.push(id.clone()),
                None => left_out.push(format!("{} (bank: {name})", found.title)),
            }
        }
        let title = arg(&raw, "title").map(str::to_owned);
        body.push_str(&format!(
            "### {}\n",
            title.as_deref().unwrap_or(&found.title)
        ));
        for (key, value) in &fields {
            let said = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            body.push_str(&format!("- {key}: {said}\n"));
        }
        for version in &versions {
            body.push_str(&format!(
                "\n{}:\n\n```\n{}\n```\n",
                version.role, version.body
            ));
        }
        body.push('\n');
        items.push(proposal::MonthItem {
            work_id: found.id.clone(),
            work_title: found.title.clone(),
            title,
            fields,
            versions,
            bank,
        });
    }
    if items.is_empty() {
        return Err(Error::refused("proposal.monthEmpty"));
    }
    let mut terms = Vec::new();
    for raw in args
        .get("register")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let raw = raw.as_object().cloned().unwrap_or_default();
        let word = required(&raw, "word")?.trim().to_owned();
        let scope = match arg(&raw, "scope") {
            None => crate::register::TermScope::Lyrics,
            Some(said) => serde_json::from_value(Value::String(said.to_owned()))
                .map_err(|_| Error::refused("mcp.badTermScope").param("scope", said))?,
        };
        terms.push(proposal::MonthTerm {
            word,
            scope,
            note: arg(&raw, "note").map(str::to_owned),
        });
    }
    let count = items.len();
    deliver(
        conn,
        &profile.id,
        session,
        None,
        Proposal::Month {
            collection_id,
            month,
            items,
            terms,
        },
        body.trim_end(),
        None,
    )?;
    let mut said = format!(
        "Proposed {count} works of the month in one proposal. It waits in the chat named after you, with the checklist the quotas give; one click applies all of it."
    );
    if !left_out.is_empty() {
        said.push_str(&format!(" Left out: {}.", left_out.join("; ")));
    }
    Ok(said)
}

/// `propose_release`: what one release goes out under, field by field.
pub(super) fn propose_release(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        args,
        ..
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    // A publication goes out once (ADR 0051): the work names its
    // release.
    let chosen = release::of_work(conn, &found.id)?.ok_or_else(|| {
        Error::refused("mcp.workHasNoRelease").param("title", found.title.clone())
    })?;
    let declared = crate::release_meta::declared(conn, &chosen.id)?;
    let raw = args
        .get("fields")
        .cloned()
        .ok_or_else(|| Error::refused("mcp.missingArg").param("key", "fields"))?;
    let proposal = proposal::release_from_value(&json!({ "fields": raw }), &chosen.id, &declared)
        .map_err(|why| Error::refused("mcp.releaseFieldsUnread").param("why", why))?;
    let proposal = crate::release_meta::public(conn, proposal)?;
    let Proposal::Release {
        fields, unknown, ..
    } = &proposal
    else {
        return Err(Error::Internal(
            "a release proposal read as something else".into(),
        ));
    };
    let mut body = String::new();
    for field in &declared {
        if let Some(text) = fields.get(&field.key).and_then(Value::as_str) {
            body.push_str(&format!("**{}**\n\n{text}\n\n", field.label));
        }
    }
    let taken = fields.len();
    let left: Vec<String> = unknown.clone();
    deliver(
        conn,
        &profile.id,
        session,
        Some(&found),
        proposal,
        body.trim_end(),
        arg(args, "note"),
    )?;
    let mut said = format!(
        "Proposed {taken} field{} for the {} release of “{}”; it waits in the chat on the work, each field beside what is written now.",
        if taken == 1 { "" } else { "s" },
        chosen.kind,
        found.title
    );
    if !left.is_empty() {
        said.push_str(&format!(
            " Left out, as the kind of release has no such field: {}.",
            left.join(", ")
        ));
    }
    Ok(said)
}
