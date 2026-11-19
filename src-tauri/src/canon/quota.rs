//! Rules a body of work is checked against (v0.151, FR-20 20.2).
//!
//! A rule is a fact of a section shaped `quotas`: its words are what the rule
//! is called, and its data a machine predicate - a field of the works, a
//! metric measured on it, a comparison, the works it is measured over and how
//! loud a breach is. The numbers are the space's own, written by a person on
//! the passport: a Suno prompt of 400 to 500 characters, no more than four
//! tracks in one playlist a month, at least five angles of a cover. Nothing
//! here knows any of them; [`check`] is the one evaluator every reader asks.

use std::collections::BTreeMap;

use regex::{Regex, RegexBuilder};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::FactStatus;
use super::fact::Fact;
use crate::error::{Error, Result};
use crate::profile::config::{ProfileConfig, ReleaseFieldType, SectionShape};
use crate::work::Work;

/// The keys a rule's data may carry.
pub const DATA_KEYS: &[&str] = &[
    "field", "metric", "op", "value", "pattern", "filter", "scope", "severity", "active",
];

/// What is measured on a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename = "QuotaMetric")]
pub enum Metric {
    /// The characters of its text.
    CharLen,
    /// The words of its text.
    WordCount,
    /// How many works carry each of its values - or, in one work, how many
    /// times its list names each.
    CountPerValue,
    /// How many different values it takes.
    UniqueCount,
    /// How many times its text matches the rule's pattern - or, over many
    /// works, how many of them match.
    MatchCount,
    /// Which part of the works, in percent, have it set - or match the
    /// pattern, when there is one.
    Share,
    /// The words its text must not say.
    ForbiddenTerms,
}

impl Metric {
    pub const ALL: [Metric; 7] = [
        Metric::CharLen,
        Metric::WordCount,
        Metric::CountPerValue,
        Metric::UniqueCount,
        Metric::MatchCount,
        Metric::Share,
        Metric::ForbiddenTerms,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Metric::CharLen => "char_len",
            Metric::WordCount => "word_count",
            Metric::CountPerValue => "count_per_value",
            Metric::UniqueCount => "unique_count",
            Metric::MatchCount => "match_count",
            Metric::Share => "share",
            Metric::ForbiddenTerms => "forbidden_terms",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|one| one.as_str() == raw)
    }
}

/// Which works a rule is measured over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename = "QuotaScope")]
pub enum Scope {
    /// Each work alone.
    Item,
    /// The works of one month or one collection together.
    CollectionPeriod,
    /// Every work of the space together.
    Global,
}

impl Scope {
    pub const ALL: [Scope; 3] = [Scope::Item, Scope::CollectionPeriod, Scope::Global];

    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Item => "item",
            Scope::CollectionPeriod => "collection_period",
            Scope::Global => "global",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|one| one.as_str() == raw)
    }
}

/// How loud a breach is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "QuotaSeverity")]
pub enum Severity {
    Warn,
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Warn => "warn",
            Severity::Error => "error",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "warn" => Some(Severity::Warn),
            "error" => Some(Severity::Error),
            _ => None,
        }
    }
}

/// What a measure is compared with.
#[derive(Debug, Clone, PartialEq)]
pub enum Bound {
    Between(f64, f64),
    Min(f64),
    Max(f64),
    Eq(f64),
    /// The words a text must not say.
    NotContains(Vec<String>),
}

impl Bound {
    fn holds(&self, measured: f64) -> bool {
        match self {
            Bound::Between(low, high) => measured >= *low && measured <= *high,
            Bound::Min(least) => measured >= *least,
            Bound::Max(most) => measured <= *most,
            Bound::Eq(exact) => (measured - exact).abs() < f64::EPSILON,
            Bound::NotContains(_) => true,
        }
    }

    /// What the rule wants, as a person reads it beside the measure.
    fn said(&self) -> String {
        match self {
            Bound::Between(low, high) => format!("{}-{}", number(*low), number(*high)),
            Bound::Min(least) => format!(">= {}", number(*least)),
            Bound::Max(most) => format!("<= {}", number(*most)),
            Bound::Eq(exact) => format!("= {}", number(*exact)),
            Bound::NotContains(terms) => terms.join(", "),
        }
    }
}

fn number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value}")
    }
}

/// Which works a rule reads: those whose `field` says one of `equals` - or,
/// with `not`, none of them.
#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub field: String,
    pub equals: Vec<String>,
    pub not: bool,
}

/// One rule, read off its fact.
#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub label: String,
    pub field: String,
    pub metric: Metric,
    pub bound: Bound,
    pub pattern: Option<Regex>,
    pub filter: Option<Filter>,
    pub scope: Scope,
    pub severity: Severity,
    pub active: bool,
}

/// A rule's data as it is written down: checked, and in one spelling.
pub fn checked(data: &Map<String, Value>) -> Result<Map<String, Value>> {
    let parsed = parse("", "", data)?;
    let mut out = Map::new();
    out.insert("field".into(), Value::String(parsed.field.clone()));
    out.insert(
        "metric".into(),
        Value::String(parsed.metric.as_str().into()),
    );
    let (op, value) = match &parsed.bound {
        Bound::Between(low, high) => ("between", serde_json::json!([low, high])),
        Bound::Min(least) => ("min", serde_json::json!(least)),
        Bound::Max(most) => ("max", serde_json::json!(most)),
        Bound::Eq(exact) => ("eq", serde_json::json!(exact)),
        Bound::NotContains(terms) => ("not_contains", serde_json::json!(terms)),
    };
    out.insert("op".into(), Value::String(op.into()));
    out.insert("value".into(), value);
    if let Some(pattern) = &parsed.pattern {
        out.insert("pattern".into(), Value::String(pattern.as_str().to_owned()));
    }
    if let Some(filter) = &parsed.filter {
        let mut kept = Map::new();
        kept.insert("field".into(), Value::String(filter.field.clone()));
        kept.insert("equals".into(), serde_json::json!(filter.equals));
        if filter.not {
            kept.insert("not".into(), Value::Bool(true));
        }
        out.insert("filter".into(), Value::Object(kept));
    }
    out.insert("scope".into(), Value::String(parsed.scope.as_str().into()));
    out.insert(
        "severity".into(),
        Value::String(parsed.severity.as_str().into()),
    );
    out.insert("active".into(), Value::Bool(parsed.active));
    Ok(out)
}

impl Rule {
    /// The rule a fact of a quotas section is; none for one whose data no
    /// longer reads - written before a key was checked, or by hand.
    pub fn of(fact: &Fact) -> Option<Self> {
        parse(&fact.id, &fact.body, &fact.data).ok()
    }

    /// The rule as a prompt is told it: a model writing a work keeps to the
    /// same numbers the check measures.
    pub fn line(&self) -> String {
        let measure = match self.metric {
            Metric::CharLen => "characters",
            Metric::WordCount => "words",
            Metric::CountPerValue => "works per value",
            Metric::UniqueCount => "different values",
            Metric::MatchCount => "matches",
            Metric::Share => "percent of works",
            Metric::ForbiddenTerms => "must not say",
        };
        let over = match self.scope {
            Scope::Item => "each work",
            Scope::CollectionPeriod => "a month or a collection",
            Scope::Global => "all works",
        };
        let mut line = format!(
            "- {}: `{}` {measure} {} ({over})",
            self.label,
            self.field,
            self.bound.said()
        );
        if let Some(pattern) = &self.pattern {
            line.push_str(&format!(", pattern `{}`", pattern.as_str()));
        }
        if let Some(filter) = &self.filter {
            let how = if filter.not { "not " } else { "" };
            line.push_str(&format!(
                ", only where `{}` is {how}{}",
                filter.field,
                filter.equals.join(" or ")
            ));
        }
        if self.severity == Severity::Error {
            line.push_str(" [must]");
        }
        line
    }
}

fn parse(id: &str, label: &str, data: &Map<String, Value>) -> Result<Rule> {
    let text = |key: &str| {
        data.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let field = text("field").ok_or_else(|| Error::refused("canon.quotaNeedsField"))?;
    let metric_word = text("metric").unwrap_or_default();
    let metric = Metric::parse(&metric_word)
        .ok_or_else(|| Error::refused("canon.quotaBadMetric").param("metric", metric_word))?;
    let op = text("op").unwrap_or_default();
    let bad_op = || {
        Error::refused("canon.quotaBadOp")
            .param("op", op.clone())
            .param("metric", metric.as_str())
    };
    let bad_value = || Error::refused("canon.quotaBadValue").param("metric", metric.as_str());
    let figure = |value: Option<&Value>| -> Result<f64> {
        let figure = value.and_then(Value::as_f64).ok_or_else(bad_value)?;
        if figure < 0.0 || (metric == Metric::Share && figure > 100.0) {
            return Err(bad_value());
        }
        Ok(figure)
    };
    let value = data.get("value");
    let bound = match (metric, op.as_str()) {
        (Metric::ForbiddenTerms, "not_contains") => {
            let terms: Vec<String> = value
                .and_then(Value::as_array)
                .ok_or_else(bad_value)?
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|term| !term.is_empty())
                .map(str::to_owned)
                .collect();
            if terms.is_empty() {
                return Err(bad_value());
            }
            Bound::NotContains(terms)
        }
        (Metric::ForbiddenTerms, _) | (_, "not_contains") => return Err(bad_op()),
        (_, "between") => {
            let pair = value.and_then(Value::as_array).ok_or_else(bad_value)?;
            if pair.len() != 2 {
                return Err(bad_value());
            }
            let (low, high) = (figure(pair.first())?, figure(pair.get(1))?);
            if low > high {
                return Err(bad_value());
            }
            Bound::Between(low, high)
        }
        (_, "min") => Bound::Min(figure(value)?),
        (_, "max") => Bound::Max(figure(value)?),
        (_, "eq") => Bound::Eq(figure(value)?),
        _ => return Err(bad_op()),
    };
    let pattern = match text("pattern") {
        Some(pattern) => Some(
            RegexBuilder::new(&pattern)
                .case_insensitive(true)
                .size_limit(1 << 20)
                .build()
                .map_err(|_| Error::refused("canon.quotaBadPattern").param("pattern", pattern))?,
        ),
        None if metric == Metric::MatchCount => {
            return Err(Error::refused("canon.quotaNeedsPattern"));
        }
        None => None,
    };
    let filter = match data.get("filter") {
        None | Some(Value::Null) => None,
        Some(Value::Object(filter)) => {
            let field = filter
                .get("field")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|field| !field.is_empty())
                .ok_or_else(|| Error::refused("canon.quotaBadFilter"))?;
            let equals: Vec<String> = match filter.get("equals") {
                Some(Value::String(one)) => vec![one.trim().to_owned()],
                Some(Value::Array(many)) => many
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|one| one.trim().to_owned())
                    .collect(),
                _ => Vec::new(),
            };
            let equals: Vec<String> = equals.into_iter().filter(|one| !one.is_empty()).collect();
            if equals.is_empty() {
                return Err(Error::refused("canon.quotaBadFilter"));
            }
            Some(Filter {
                field: field.to_owned(),
                equals,
                not: filter.get("not").and_then(Value::as_bool) == Some(true),
            })
        }
        Some(_) => return Err(Error::refused("canon.quotaBadFilter")),
    };
    let scope = match text("scope") {
        None => Scope::Item,
        Some(word) => Scope::parse(&word)
            .ok_or_else(|| Error::refused("canon.quotaBadScope").param("scope", word))?,
    };
    // A share is a part of many works: of one work alone it says nothing.
    if metric == Metric::Share && scope == Scope::Item {
        return Err(Error::refused("canon.quotaNeedsSet"));
    }
    let severity = match text("severity") {
        None => Severity::Warn,
        Some(word) => Severity::parse(&word)
            .ok_or_else(|| Error::refused("canon.quotaBadSeverity").param("severity", word))?,
    };
    Ok(Rule {
        id: id.to_owned(),
        label: label.to_owned(),
        field,
        metric,
        bound,
        pattern,
        filter,
        scope,
        severity,
        active: data.get("active").and_then(Value::as_bool) != Some(false),
    })
}

/// What is checked: one work, the works of a collection, those of a month,
/// or - all absent - the whole space.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct QuotaSubject {
    #[serde(default)]
    pub work_id: Option<String>,
    #[serde(default)]
    pub collection_id: Option<String>,
    /// `YYYY-MM`: the works going out or planned that month.
    #[serde(default)]
    pub month: Option<String>,
}

/// One breach of one rule.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct QuotaFinding {
    pub rule_id: String,
    pub label: String,
    pub field: String,
    pub metric: Metric,
    pub severity: Severity,
    pub scope: Scope,
    /// The work it is about; none for a finding about the works together.
    pub work_id: Option<String>,
    pub title: Option<String>,
    /// What was measured: a length, a count, a share in percent.
    pub measured: f64,
    /// The value counted or the words found, when the finding is about one.
    pub value: Option<String>,
    /// What the rule wants, as written: `400-500`, `<= 4`, `>= 5`.
    pub expected: String,
}

/// What a check found.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct QuotaReport {
    /// The rules that were asked - the active ones that apply to the subject.
    pub rules: u32,
    /// The works they were measured on.
    pub works: u32,
    pub findings: Vec<QuotaFinding>,
}

/// The rules of the space: every settled fact of every quotas section of its
/// cards, the ones turned off included - [`check`] leaves those out.
pub fn rules(conn: &Connection, profile_id: &str, config: &ProfileConfig) -> Result<Vec<Rule>> {
    let mut found = Vec::new();
    for kind in config.note_kinds.iter().filter(|kind| kind.is_card()) {
        for section in kind
            .sections
            .iter()
            .filter(|s| s.shape == SectionShape::Quotas)
        {
            let mut statement = conn.prepare(
                "SELECT f.id FROM canon_fact f JOIN note n ON n.id = f.note_id
                  WHERE f.profile_id = ?1 AND n.kind = ?2 AND f.section = ?3
                  ORDER BY n.created_at, f.position, f.rowid",
            )?;
            let ids = statement
                .query_map(params![profile_id, kind.key, section.key], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for id in ids {
                let Some(fact) = super::fact::get(conn, &id)? else {
                    continue;
                };
                if fact.status == FactStatus::Retired {
                    continue;
                }
                if let Some(rule) = Rule::of(&fact) {
                    found.push(rule);
                }
            }
        }
    }
    Ok(found)
}

/// Check `subject` against the space's rules.
///
/// One work is measured by the rules of each work. A collection or a month
/// is measured by those too, work by work, then by the rules of a period
/// over its works together and by the rules of the space over all of them.
/// The whole space is measured by the rules of each work and of the space.
pub fn check(conn: &Connection, subject: &QuotaSubject) -> Result<QuotaReport> {
    check_with(conn, subject, &Proposed::new())
}

/// What a proposal would write, by work and by field - a title, an overview
/// field, a text's role - read before what the work says now (v0.153): the
/// checklist a month's proposals are shown with before "Apply all".
pub type Proposed = BTreeMap<String, Map<String, Value>>;

/// [`check`] as `subject` would stand with `proposed` written.
pub fn check_with(
    conn: &Connection,
    subject: &QuotaSubject,
    proposed: &Proposed,
) -> Result<QuotaReport> {
    let profile_id = crate::actions::active_profile_id(conn)?;
    let config = crate::profile::config_for(conn, &profile_id)?;
    let rules: Vec<Rule> = rules(conn, &profile_id, &config)?
        .into_iter()
        .filter(|rule| rule.active)
        .collect();

    let (set, period) = if let Some(id) = &subject.work_id {
        let work = crate::work::get(conn, id)?.ok_or_else(|| Error::not_found("work", id))?;
        (vec![work], false)
    } else if let Some(id) = &subject.collection_id {
        (
            works_where(conn, &profile_id, "collection_id = ?2", id)?,
            true,
        )
    } else if let Some(month) = &subject.month {
        let month = month.trim();
        if month.len() != 7 || month.as_bytes().get(4) != Some(&b'-') {
            return Err(Error::refused("canon.quotaBadMonth").param("month", month));
        }
        (month_works(conn, &profile_id, month)?, true)
    } else {
        (all_works(conn, &profile_id)?, false)
    };
    let whole = subject.work_id.is_none();
    let everything = if whole && !rules.iter().any(|rule| rule.scope == Scope::Global) {
        Vec::new()
    } else if whole && period {
        all_works(conn, &profile_id)?
    } else if whole {
        set.clone()
    } else {
        Vec::new()
    };

    let mut reader = Reader {
        conn,
        config: &config,
        proposed,
        cache: BTreeMap::new(),
    };
    let mut findings = Vec::new();
    let mut asked = 0u32;
    for rule in &rules {
        let over: Option<&[Work]> = match rule.scope {
            Scope::Item => {
                asked += 1;
                for work in &set {
                    findings.extend(reader.one(rule, work)?);
                }
                None
            }
            Scope::CollectionPeriod if period => Some(&set),
            Scope::Global if whole => Some(&everything),
            _ => None,
        };
        if let Some(over) = over {
            asked += 1;
            findings.extend(reader.many(rule, over)?);
        }
    }
    Ok(QuotaReport {
        rules: asked,
        works: u32::try_from(set.len()).unwrap_or(u32::MAX),
        findings,
    })
}

/// Every work of the space.
fn all_works(conn: &Connection, profile_id: &str) -> Result<Vec<Work>> {
    works_where(conn, profile_id, "?2 = ?2", "")
}

fn works_where(
    conn: &Connection,
    profile_id: &str,
    clause: &str,
    value: &str,
) -> Result<Vec<Work>> {
    let mut statement = conn.prepare(&format!(
        "SELECT id FROM work WHERE profile_id = ?1 AND {clause} ORDER BY position, created_at, rowid"
    ))?;
    let ids = statement
        .query_map(params![profile_id, value], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    load(conn, ids)
}

/// The works going out in `month` - a release dated in it, out or booked -
/// and the works planned on one of its days.
fn month_works(conn: &Connection, profile_id: &str, month: &str) -> Result<Vec<Work>> {
    let mut statement = conn.prepare(
        "SELECT w.id FROM work w
          WHERE w.profile_id = ?1
            AND (EXISTS (SELECT 1 FROM release r WHERE r.work_id = w.id
                          AND substr(coalesce(r.scheduled_at, r.released_at), 1, 7) = ?2)
              OR EXISTS (SELECT 1 FROM work_plan p WHERE p.work_id = w.id
                          AND substr(p.scheduled_at, 1, 7) = ?2))
          ORDER BY w.position, w.created_at, w.rowid",
    )?;
    let ids = statement
        .query_map(params![profile_id, month], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    load(conn, ids)
}

fn load(conn: &Connection, ids: Vec<String>) -> Result<Vec<Work>> {
    let mut works = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(work) = crate::work::get(conn, &id)? {
            works.push(work);
        }
    }
    Ok(works)
}

/// What a field of a work says: one text, or a list of values.
#[derive(Debug, Clone, PartialEq)]
enum Said {
    Text(String),
    List(Vec<String>),
}

impl Said {
    fn text(&self) -> String {
        match self {
            Said::Text(text) => text.trim().to_owned(),
            Said::List(values) => values.join(", "),
        }
    }

    fn values(&self) -> Vec<String> {
        match self {
            Said::Text(text) if text.trim().is_empty() => Vec::new(),
            Said::Text(text) => vec![text.trim().to_owned()],
            Said::List(values) => values
                .iter()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
                .collect(),
        }
    }
}

/// Reads fields of works for the rules, each once.
struct Reader<'a> {
    conn: &'a Connection,
    config: &'a ProfileConfig,
    proposed: &'a Proposed,
    cache: BTreeMap<(String, String), Option<Said>>,
}

impl Reader<'_> {
    /// What `field` of `work` says; none when the work's kind has no such
    /// field, so a rule about it does not read the work at all.
    fn said(&mut self, work: &Work, field: &str) -> Result<Option<Said>> {
        let key = (work.id.clone(), field.to_owned());
        if let Some(found) = self.cache.get(&key) {
            return Ok(found.clone());
        }
        let found = match said(self.conn, self.config, work, field)? {
            // A field the work's kind has, as the proposal would write it.
            Some(now) => Some(
                match self
                    .proposed
                    .get(&work.id)
                    .and_then(|fields| fields.get(field))
                {
                    Some(value) => from_json(Some(value)),
                    None => now,
                },
            ),
            None => None,
        };
        self.cache.insert(key, found.clone());
        Ok(found)
    }

    /// Whether the rule's filter keeps `work`.
    fn kept(&mut self, rule: &Rule, work: &Work) -> Result<bool> {
        let Some(filter) = &rule.filter else {
            return Ok(true);
        };
        let values = self
            .said(work, &filter.field)?
            .map(|said| said.values())
            .unwrap_or_default();
        let hit = values.iter().any(|value| {
            filter
                .equals
                .iter()
                .any(|wanted| wanted.to_lowercase() == value.to_lowercase())
        });
        Ok(hit != filter.not)
    }

    /// The breaches of `rule` by one work measured alone.
    fn one(&mut self, rule: &Rule, work: &Work) -> Result<Vec<QuotaFinding>> {
        let Some(said) = self.said(work, &rule.field)? else {
            return Ok(Vec::new());
        };
        if !self.kept(rule, work)? {
            return Ok(Vec::new());
        }
        let finding = |measured: f64, value: Option<String>| QuotaFinding {
            rule_id: rule.id.clone(),
            label: rule.label.clone(),
            field: rule.field.clone(),
            metric: rule.metric,
            severity: rule.severity,
            scope: rule.scope,
            work_id: Some(work.id.clone()),
            title: Some(work.title.clone()),
            measured,
            value,
            expected: rule.bound.said(),
        };
        let text = said.text();
        let measured = match rule.metric {
            Metric::CharLen => count(text.chars().count()),
            Metric::WordCount => count(text.split_whitespace().count()),
            Metric::MatchCount => count(
                rule.pattern
                    .as_ref()
                    .map_or(0, |pattern| pattern.find_iter(&text).count()),
            ),
            Metric::UniqueCount => count(distinct(&said.values()).len()),
            Metric::CountPerValue => {
                return Ok(per_value(&said.values())
                    .into_iter()
                    .filter(|(_, (_, times))| !rule.bound.holds(count(*times)))
                    .map(|(_, (shown, times))| finding(count(times), Some(shown)))
                    .collect());
            }
            Metric::ForbiddenTerms => {
                let Bound::NotContains(terms) = &rule.bound else {
                    return Ok(Vec::new());
                };
                let lowered = text.to_lowercase();
                let found: Vec<String> = terms
                    .iter()
                    .filter(|term| lowered.contains(&term.to_lowercase()))
                    .cloned()
                    .collect();
                if found.is_empty() {
                    return Ok(Vec::new());
                }
                return Ok(vec![finding(count(found.len()), Some(found.join(", ")))]);
            }
            Metric::Share => return Ok(Vec::new()),
        };
        Ok(if rule.bound.holds(measured) {
            Vec::new()
        } else {
            vec![finding(measured, None)]
        })
    }

    /// The breaches of `rule` by `works` measured together.
    fn many(&mut self, rule: &Rule, works: &[Work]) -> Result<Vec<QuotaFinding>> {
        let mut read: Vec<(&Work, Said)> = Vec::new();
        for work in works {
            if let Some(said) = self.said(work, &rule.field)?
                && self.kept(rule, work)?
            {
                read.push((work, said));
            }
        }
        let finding = |measured: f64, value: Option<String>, work: Option<&Work>| QuotaFinding {
            rule_id: rule.id.clone(),
            label: rule.label.clone(),
            field: rule.field.clone(),
            metric: rule.metric,
            severity: rule.severity,
            scope: rule.scope,
            work_id: work.map(|work| work.id.clone()),
            title: work.map(|work| work.title.clone()),
            measured,
            value,
            expected: rule.bound.said(),
        };
        let measured = match rule.metric {
            // A measure of one text over many works is still a measure of
            // each: the period only says which works.
            Metric::CharLen | Metric::WordCount | Metric::ForbiddenTerms => {
                let mut found = Vec::new();
                for (work, _) in &read {
                    found.extend(self.one(rule, work)?);
                }
                return Ok(found);
            }
            Metric::MatchCount => count(
                read.iter()
                    .filter(|(_, said)| {
                        rule.pattern
                            .as_ref()
                            .is_some_and(|pattern| pattern.is_match(&said.text()))
                    })
                    .count(),
            ),
            Metric::UniqueCount => {
                let all: Vec<String> = read.iter().flat_map(|(_, said)| said.values()).collect();
                count(distinct(&all).len())
            }
            Metric::CountPerValue => {
                // A work counts once for a value, however often it names it.
                let all: Vec<String> = read
                    .iter()
                    .flat_map(|(_, said)| distinct(&said.values()))
                    .collect();
                return Ok(per_value(&all)
                    .into_iter()
                    .filter(|(_, (_, times))| !rule.bound.holds(count(*times)))
                    .map(|(_, (shown, times))| finding(count(times), Some(shown), None))
                    .collect());
            }
            Metric::Share => {
                if read.is_empty() {
                    return Ok(Vec::new());
                }
                let hits = read
                    .iter()
                    .filter(|(_, said)| match &rule.pattern {
                        Some(pattern) => pattern.is_match(&said.text()),
                        None => !said.values().is_empty(),
                    })
                    .count();
                (count(hits) * 100.0 / count(read.len()) * 10.0).round() / 10.0
            }
        };
        Ok(if rule.bound.holds(measured) {
            Vec::new()
        } else {
            vec![finding(measured, None, None)]
        })
    }
}

#[allow(clippy::cast_precision_loss)]
fn count(n: usize) -> f64 {
    n as f64
}

/// The values once each, compared without case.
fn distinct(values: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for value in values {
        if !kept
            .iter()
            .any(|one| one.to_lowercase() == value.to_lowercase())
        {
            kept.push(value.clone());
        }
    }
    kept
}

/// How many times each value is named, compared without case: by its lower
/// case, the first spelling and the count.
fn per_value(values: &[String]) -> BTreeMap<String, (String, usize)> {
    let mut counted: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for value in values {
        counted
            .entry(value.to_lowercase())
            .or_insert_with(|| (value.clone(), 0))
            .1 += 1;
    }
    counted
}

/// What `field` of `work` says.
///
/// A field is named as a person names it on the passport: `title`, `kind`,
/// `status`, `stage` and `tags` of any work; `cover`, `cover_scene` and
/// `cover_prompt` of one with a cover; a role of its texts - `suno_prompt`,
/// `lyrics`; one of its overview fields - `genre`, `angle`; a field of the
/// release it goes out as - `playlists`, or `youtube.title` for one door's.
/// None when the work's kind has no such field.
fn said(
    conn: &Connection,
    config: &ProfileConfig,
    work: &Work,
    field: &str,
) -> Result<Option<Said>> {
    let kind = config.vocabulary(&work.kind);
    let text = |value: &str| Some(Said::Text(value.to_owned()));
    match field {
        "title" => return Ok(text(&work.title)),
        "kind" => return Ok(text(&work.kind)),
        "status" => return Ok(text(&work.status)),
        "stage" => return Ok(text(&work.stage.to_string())),
        "tags" => return Ok(Some(Said::List(work.tags.clone()))),
        "cover" if kind.cover => return Ok(text(&work.cover.idea)),
        "cover_scene" if kind.cover => return Ok(text(&work.cover.scene)),
        "cover_prompt" if kind.cover => {
            let sent = work.cover.sent.as_ref().map(|sent| sent.picture.as_str());
            return Ok(text(sent.unwrap_or(&work.cover.scene)));
        }
        _ => {}
    }
    if let Some((door, key)) = field.split_once('.') {
        let Some(place) = kind.release_kinds.iter().find(|one| one.key == door) else {
            return Ok(None);
        };
        let Some(one) = place.fields.iter().find(|one| one.key == key) else {
            return Ok(None);
        };
        let release = crate::release::of_work(conn, &work.id)?;
        return Ok(Some(match release.filter(|release| release.kind == door) {
            Some(release) => of_release(release.meta.get(key), one.field_type),
            None => Said::Text(String::new()),
        }));
    }
    if kind.version_roles.iter().any(|role| role.key == field) {
        let body = crate::work::version::for_role(conn, &work.id, field)?
            .map(|version| version.body)
            .unwrap_or_default();
        return Ok(Some(Said::Text(body)));
    }
    if config
        .fields_of(&work.kind)
        .iter()
        .any(|one| one.key == field)
    {
        return Ok(Some(from_json(work.meta.get(field))));
    }
    if let Some(one) = kind
        .release_kinds
        .iter()
        .flat_map(|place| place.fields.iter())
        .find(|one| one.key == field)
    {
        let release = crate::release::of_work(conn, &work.id)?;
        return Ok(Some(match release {
            Some(release) => of_release(release.meta.get(field), one.field_type),
            None => Said::Text(String::new()),
        }));
    }
    Ok(None)
}

/// The values `field` of `work` says, each once - what a bank bound to the
/// field reads to find the items a work takes (v0.151). Empty when the
/// work's kind has no such field or it says nothing.
pub fn values_of(
    conn: &Connection,
    config: &ProfileConfig,
    work: &Work,
    field: &str,
) -> Result<Vec<String>> {
    Ok(said(conn, config, work, field)?
        .map(|said| distinct(&said.values()))
        .unwrap_or_default())
}

/// A release's field: a box of tags is a list however it is kept - a
/// line of them, comma by comma, as the platform takes them.
fn of_release(value: Option<&Value>, shape: ReleaseFieldType) -> Said {
    match (from_json(value), shape) {
        (Said::Text(text), ReleaseFieldType::Tags) => Said::List(
            text.split([',', '\n'])
                .map(str::trim)
                .filter(|one| !one.is_empty())
                .map(str::to_owned)
                .collect(),
        ),
        (said, _) => said,
    }
}

fn from_json(value: Option<&Value>) -> Said {
    match value {
        None | Some(Value::Null) => Said::Text(String::new()),
        Some(Value::String(text)) => Said::Text(text.clone()),
        Some(Value::Array(values)) => Said::List(
            values
                .iter()
                .map(|value| match value {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .collect(),
        ),
        Some(other) => Said::Text(other.to_string()),
    }
}

#[cfg(test)]
mod tests;
