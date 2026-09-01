use time::OffsetDateTime;
use time::format_description::well_known::Iso8601;
use time::format_description::well_known::iso8601;

/// The one shape every stored timestamp takes: `2026-09-01T22:31:05.123Z`.
///
/// Fixed-width on purpose. `Rfc3339` omits the fraction when the nanoseconds
/// happen to be zero, so a moment landing exactly on a second came out as
/// `…:05Z` while its neighbours were `…:05.4Z` — and `'Z'` sorts after `'.'`,
/// so that message jumped to the end of its own second. It cost a chat its
/// order on a CI runner fast enough to hit the whole second; the same coin
/// flip was waiting on every machine.
const STAMP: iso8601::EncodedConfig = iso8601::Config::DEFAULT
    .set_time_precision(iso8601::TimePrecision::Second {
        decimal_digits: std::num::NonZeroU8::new(3),
    })
    .encode();

/// Current UTC instant as RFC 3339 — the only timestamp format stored.
///
/// Text timestamps sort lexicographically in SQLite, which is what the calendar
/// and the journal rely on. That only holds while every stamp has the same
/// shape, which is what `STAMP` is for.
pub fn now() -> String {
    OffsetDateTime::now_utc()
        .format(&Iso8601::<STAMP>)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00.000Z"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;
    use time::format_description::well_known::Rfc3339;

    fn stamp(at: OffsetDateTime) -> String {
        at.format(&Iso8601::<STAMP>).unwrap()
    }

    /// The width `now()` actually produces.
    ///
    /// Every assertion below is anchored to this rather than to `STAMP`
    /// directly: a test that formats its own timestamps proves something about
    /// the formatter, not about the function the whole app calls. Verified by
    /// mutation — swapping `now()` back to `Rfc3339` left the first version of
    /// these tests entirely green.
    fn width_of_now() -> usize {
        now().len()
    }

    #[test]
    fn now_is_rfc3339_and_sorts_chronologically() {
        let first = now();
        let parsed = OffsetDateTime::parse(&first, &Rfc3339);
        assert!(parsed.is_ok(), "`{first}` is not RFC 3339");

        // Same format for every instant, so string order is time order.
        assert!(first.as_str() > "2020-01-01T00:00:00Z");
    }

    /// Every stamp is the same width, whatever the nanoseconds happen to be.
    ///
    /// This is the property the whole scheme rests on, and it used to be only
    /// a comment. `Rfc3339` drops the fraction on a whole second, and `'Z'`
    /// sorts after `'.'` — so a message written exactly on the second came
    /// back last within that second. It cost a chat its order in CI; the
    /// promise now has a test.
    #[test]
    fn a_whole_second_is_stamped_the_same_width_as_any_other() {
        let whole = OffsetDateTime::from_unix_timestamp(1_787_868_720).unwrap();

        let on_the_second = stamp(whole);
        let just_after = stamp(whole + Duration::milliseconds(500));

        assert_eq!(
            on_the_second.len(),
            just_after.len(),
            "`{on_the_second}` and `{just_after}` are different widths, so they sort by shape rather than by time"
        );
        // And the shape under test is the one `now()` writes into the database.
        assert_eq!(
            on_the_second.len(),
            width_of_now(),
            "`now()` does not produce the shape these assertions are about"
        );
        assert!(
            on_the_second < just_after,
            "`{on_the_second}` must sort before `{just_after}`"
        );
    }

    /// Order in the string is order in time, across a second boundary too.
    /// `now()` itself never emits a bare-second stamp.
    ///
    /// The one that matters: whatever the clock reads, the stored string is the
    /// same width. Sampling cannot prove it for every instant, so the shape is
    /// asserted directly — a fraction, three digits, then the zone.
    #[test]
    fn now_always_carries_its_fraction() {
        let stamped = now();
        let (_, tail) = stamped.split_at(stamped.len() - 5);
        assert!(
            tail.starts_with('.')
                && tail.ends_with('Z')
                && tail[1..4].chars().all(|c| c.is_ascii_digit()),
            "`{stamped}` does not end in a three-digit fraction; a whole second would sort after its own neighbours"
        );
    }

    #[test]
    fn stamps_sort_in_the_order_the_instants_happened() {
        let base = OffsetDateTime::from_unix_timestamp(1_787_868_720).unwrap();
        let moments = [
            Duration::ZERO,
            Duration::milliseconds(1),
            Duration::milliseconds(500),
            Duration::milliseconds(999),
            Duration::seconds(1),
        ];

        let stamped: Vec<String> = moments.iter().map(|d| stamp(base + *d)).collect();
        let mut sorted = stamped.clone();
        sorted.sort();

        assert_eq!(stamped, sorted, "string order disagrees with time order");
    }
}
