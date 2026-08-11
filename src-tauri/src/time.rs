use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Current UTC instant as RFC 3339 — the only timestamp format stored.
///
/// Text timestamps sort lexicographically in SQLite, which is what the calendar
/// and the journal rely on.
pub fn now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_rfc3339_and_sorts_chronologically() {
        let first = now();
        let parsed = OffsetDateTime::parse(&first, &Rfc3339);
        assert!(parsed.is_ok(), "`{first}` is not RFC 3339");

        // Same format for every instant, so string order is time order.
        assert!(first.as_str() > "2020-01-01T00:00:00Z");
    }
}
