//! Time inside the world: the words a fact is dated in, and where it sorts.
//!
//! A fact is dated the way a person dates things - "winter 2022/23", "every
//! summer from 2008 to 2019", "9 June 2025" - and the timeline has to put it
//! somewhere. So a fact carries both: the words, shown as written, and a
//! partial date (`2022`, `2022-12`, `2022-12-01`) the line sorts by. When only
//! the words are given, the sort key is read off them here - a year, and the
//! month or the day when the words say one clearly. What cannot be read is
//! left unplaced rather than guessed.

/// Whether `key` is a partial date the timeline sorts by: a year, a month of
/// a year, or a day.
pub fn is_sort_key(key: &str) -> bool {
    let parts: Vec<&str> = key.split('-').collect();
    let digits =
        |part: &str, len: usize| part.len() == len && part.bytes().all(|b| b.is_ascii_digit());
    match parts.as_slice() {
        [year] => digits(year, 4),
        [year, month] => digits(year, 4) && digits(month, 2) && in_range(month, 1, 12),
        [year, month, day] => {
            digits(year, 4)
                && digits(month, 2)
                && digits(day, 2)
                && in_range(month, 1, 12)
                && in_range(day, 1, 31)
        }
        _ => false,
    }
}

fn in_range(number: &str, low: u32, high: u32) -> bool {
    number
        .parse::<u32>()
        .is_ok_and(|n| (low..=high).contains(&n))
}

/// Months by the first letters of their names, English and Russian. Stems
/// rather than words: "июня", "июнь" and "June" all start the same way.
const MONTHS: [(&str, u32); 25] = [
    ("jan", 1),
    ("feb", 2),
    ("mar", 3),
    ("apr", 4),
    ("may", 5),
    ("jun", 6),
    ("jul", 7),
    ("aug", 8),
    ("sep", 9),
    ("oct", 10),
    ("nov", 11),
    ("dec", 12),
    ("янв", 1),
    ("фев", 2),
    ("мар", 3),
    ("апр", 4),
    ("мая", 5),
    ("май", 5),
    ("июн", 6),
    ("июл", 7),
    ("авг", 8),
    ("сен", 9),
    ("окт", 10),
    ("ноя", 11),
    ("дек", 12),
];

/// Seasons, placed at their first month.
const SEASONS: [(&str, u32); 11] = [
    ("spring", 3),
    ("summer", 6),
    ("autumn", 9),
    ("fall", 9),
    ("winter", 12),
    ("весн", 3),
    ("лето", 6),
    ("летом", 6),
    ("летн", 6),
    ("осен", 9),
    ("зим", 12),
];

/// The sort key the words of a date say, when they say one.
///
/// A year is the least it needs; a month or a season beside the year narrows
/// it, and a day number before the month narrows it to the day. A span sorts
/// at its start. A numeric date is read day first, the way it is written in
/// most of the world this app is used in (`9.06.2025`).
pub fn sort_key_of(label: &str) -> Option<String> {
    let lower = label.to_lowercase();

    // An ISO month or day as written. A bare year is left to the reading
    // below, which looks for a month or a season beside it.
    for word in lower.split(|c: char| c.is_whitespace() || c == ',') {
        if word.contains('-') && is_sort_key(word) && in_range(&word[..4], 1000, 2999) {
            return Some(word.to_owned());
        }
    }

    // A numeric date, day first: 9.06.2025, 09/06/2025.
    for word in lower.split(|c: char| c.is_whitespace() || c == ',') {
        let parts: Vec<&str> = word.split(['.', '/']).collect();
        if let [day, month, year] = parts.as_slice() {
            if year.len() == 4 && year.bytes().all(|b| b.is_ascii_digit()) {
                if let (Ok(day), Ok(month)) = (day.parse::<u32>(), month.parse::<u32>()) {
                    let key = format!("{year}-{month:02}-{day:02}");
                    if is_sort_key(&key) {
                        return Some(key);
                    }
                }
            }
        }
    }

    let words: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect();
    let year_at = words.iter().position(|word| {
        word.len() == 4 && word.bytes().all(|b| b.is_ascii_digit()) && in_range(word, 1000, 2999)
    })?;
    let year = &words[year_at];

    // A month named before the year, with a day before the month.
    for (index, word) in words[..year_at].iter().enumerate().rev() {
        if let Some(month) = MONTHS
            .iter()
            .find(|(stem, _)| word.starts_with(stem))
            .map(|(_, month)| *month)
        {
            let day = index
                .checked_sub(1)
                .and_then(|at| words[at].parse::<u32>().ok())
                .filter(|day| (1..=31).contains(day));
            return Some(match day {
                Some(day) => format!("{year}-{month:02}-{day:02}"),
                None => format!("{year}-{month:02}"),
            });
        }
        if let Some(month) = SEASONS
            .iter()
            .find(|(stem, _)| word.starts_with(stem))
            .map(|(_, month)| *month)
        {
            return Some(format!("{year}-{month:02}"));
        }
    }

    Some(year.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_partial_date_is_a_year_a_month_or_a_day() {
        for key in ["2022", "2022-12", "2025-06-09"] {
            assert!(is_sort_key(key), "{key}");
        }
        for key in [
            "22",
            "2022-13",
            "2022-1",
            "2025-06-32",
            "2025/06/09",
            "winter",
        ] {
            assert!(!is_sort_key(key), "{key}");
        }
    }

    #[test]
    fn the_words_of_a_date_give_the_key_they_clearly_say() {
        let cases = [
            ("2020", Some("2020")),
            ("с 2012", Some("2012")),
            ("2008–2019", Some("2008")),
            ("осень 2022", Some("2022-09")),
            ("зима 2022/23", Some("2022-12")),
            ("winter 2022/23", Some("2022-12")),
            ("9 июня 2025", Some("2025-06-09")),
            ("June 2025", Some("2025-06")),
            ("9 June 2025", Some("2025-06-09")),
            ("9.06.2025", Some("2025-06-09")),
            ("2025-06-09", Some("2025-06-09")),
            ("в детстве", None),
            ("every summer", None),
        ];
        for (label, key) in cases {
            assert_eq!(sort_key_of(label).as_deref(), key, "{label}");
        }
    }

    #[test]
    fn a_number_that_is_not_a_year_is_not_read_as_one() {
        assert_eq!(sort_key_of("11 songs on the fridge"), None);
        assert_eq!(sort_key_of("track 0317"), None);
    }
}
