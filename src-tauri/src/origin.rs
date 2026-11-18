//! Where a shipped thing came from, and whether its owner has touched it
//! since (v0.125, ADR 0087).
//!
//! What kilna ships into a space - a profile's actions, statuses, stops,
//! marks, fields, kinds of note and the canon's sections, the vibe's words,
//! the style dictionary's starter set - is the space's own from the moment it
//! lands, and a later build ships newer versions of some of it. One rule says
//! what happens to each entry then, whatever kind of thing it is:
//!
//! - one nobody touched since it was given takes the newer version;
//! - one its owner changed keeps what they wrote, and a newer version waits
//!   beside it to be compared - kept or taken by a person, never by a start;
//! - one its owner deleted stays deleted;
//! - one shipped since arrives.
//!
//! "Nobody touched" is a fingerprint, not a flag: the entry's content against
//! the fingerprint of the version last given to it. Every shipped entry is
//! known by where it came from (`profile:<key>`, and `pack:<id>` once packs
//! of styles arrive), its key there, and that fingerprint.

use serde_json::Value;

/// FNV-1a over a text, as sixteen hex digits. Not a guard against anyone -
/// only "is this still what was written".
pub fn fingerprint(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The fingerprint of a JSON value with its object keys in order: the same
/// content has the same fingerprint however its fields were written.
pub fn digest(value: &Value) -> String {
    let mut text = String::new();
    canonical(value, &mut text);
    fingerprint(&text)
}

fn canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                canonical(&map[key], out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// What a space was given of one shipped entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Given {
    /// The fingerprint of the version last given. Empty for an entry that
    /// was already its owner's when it was first met: no version was given
    /// to it, so whatever it holds is theirs.
    pub digest: String,
    /// The shipped version the owner chose to keep theirs over, by its
    /// fingerprint: not offered again until a newer one ships.
    pub kept: Option<String>,
}

impl Given {
    fn of(digest: &str) -> Self {
        Self {
            digest: digest.to_owned(),
            kept: None,
        }
    }
}

/// What one shipped entry comes to when a build meets a space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Untouched, and already the shipped version.
    Current,
    /// Untouched, and the newer shipped version is written over it.
    Renewed,
    /// Neither there nor ever given: it arrives.
    Added,
    /// Given and deleted since: it stays deleted.
    Deleted,
    /// First met already reading as it ships, or changed back to exactly
    /// that: untouched from here on.
    Adopted,
    /// Its owner's, and nothing newer ships - or they kept theirs over it.
    Kept,
    /// Its owner's, and a shipped version waits beside it to be compared.
    Offered,
}

impl Verdict {
    /// Whether the entry is written with the shipped version.
    pub fn writes(self) -> bool {
        matches!(self, Self::Renewed | Self::Added)
    }
}

/// The rule, for one entry: `current` is the fingerprint of what the space
/// holds (none when it holds nothing by that key), `given` what it was given,
/// `shipped` the fingerprint of the version this build ships.
pub fn decide(current: Option<&str>, given: Option<&Given>, shipped: &str) -> Verdict {
    match (current, given) {
        (None, None) => Verdict::Added,
        (None, Some(_)) => Verdict::Deleted,
        (Some(current), None) if current == shipped => Verdict::Adopted,
        // Met for the first time and not reading as it ships: an older
        // wording or the owner's own, and either way not a start's to
        // overwrite.
        (Some(_), None) => Verdict::Offered,
        (Some(current), Some(given)) if current == given.digest => {
            if shipped == given.digest {
                Verdict::Current
            } else {
                Verdict::Renewed
            }
        }
        (Some(current), Some(_)) if current == shipped => Verdict::Adopted,
        // Changed, and the standard is what it was when it was given: the
        // owner's edit is the news, and nothing newer is there to compare.
        (Some(_), Some(given))
            if shipped == given.digest || given.kept.as_deref() == Some(shipped) =>
        {
            Verdict::Kept
        }
        (Some(_), Some(_)) => Verdict::Offered,
    }
}

/// What the space has been given of the entry once `verdict` is carried out.
pub fn after(verdict: Verdict, given: Option<&Given>, shipped: &str) -> Option<Given> {
    match verdict {
        Verdict::Renewed | Verdict::Added | Verdict::Adopted => Some(Given::of(shipped)),
        Verdict::Current | Verdict::Deleted => given.cloned(),
        Verdict::Kept | Verdict::Offered => Some(given.cloned().unwrap_or_else(|| Given::of(""))),
    }
}

/// What the space has been given of an entry it was never given and does
/// not hold, where its absence is its owner's choice: deleted, from the start.
pub fn left_out() -> Given {
    Given::of("")
}

/// What the space has been given once its owner keeps theirs over the
/// shipped version.
pub fn kept(given: Option<&Given>, shipped: &str) -> Given {
    Given {
        digest: given.map(|g| g.digest.clone()).unwrap_or_default(),
        kept: Some(shipped.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn given(digest: &str) -> Given {
        Given::of(digest)
    }

    #[test]
    fn the_same_content_has_one_fingerprint_whatever_the_order_of_its_fields() {
        let a = json!({ "key": "draft", "label": { "en": "Draft", "ru": "Черновик" } });
        let b = json!({ "label": { "ru": "Черновик", "en": "Draft" }, "key": "draft" });
        assert_eq!(digest(&a), digest(&b));
        assert_ne!(digest(&a), digest(&json!({ "key": "draft" })));
        // An array keeps its order: a list reordered is a list changed.
        assert_ne!(digest(&json!([1, 2])), digest(&json!([2, 1])));
    }

    #[test]
    fn an_untouched_entry_takes_the_newer_version_and_a_current_one_stays() {
        assert_eq!(decide(Some("a"), Some(&given("a")), "a"), Verdict::Current);
        assert_eq!(decide(Some("a"), Some(&given("a")), "b"), Verdict::Renewed);
        assert_eq!(
            after(Verdict::Renewed, Some(&given("a")), "b"),
            Some(given("b"))
        );
    }

    #[test]
    fn a_changed_entry_is_kept_and_a_newer_standard_offered_beside_it() {
        // Changed, nothing newer: the edit stands, nothing to compare.
        assert_eq!(decide(Some("mine"), Some(&given("a")), "a"), Verdict::Kept);
        // Changed, and the standard moved on: offered, never written.
        assert_eq!(
            decide(Some("mine"), Some(&given("a")), "b"),
            Verdict::Offered
        );
        assert!(!Verdict::Offered.writes());
        // Kept over that version: not offered again until a newer ships.
        let kept_b = kept(Some(&given("a")), "b");
        assert_eq!(decide(Some("mine"), Some(&kept_b), "b"), Verdict::Kept);
        assert_eq!(decide(Some("mine"), Some(&kept_b), "c"), Verdict::Offered);
        // Taken: given the standard, untouched from there.
        assert_eq!(decide(Some("b"), Some(&given("b")), "b"), Verdict::Current);
        // Changed back to exactly the standard: untouched from here on.
        assert_eq!(decide(Some("b"), Some(&given("a")), "b"), Verdict::Adopted);
    }

    #[test]
    fn a_deleted_entry_stays_deleted_and_a_new_one_arrives() {
        assert_eq!(decide(None, Some(&given("a")), "b"), Verdict::Deleted);
        assert_eq!(decide(None, None, "b"), Verdict::Added);
        assert!(Verdict::Added.writes());
    }

    #[test]
    fn an_entry_first_met_is_adopted_as_it_ships_or_left_its_owners() {
        assert_eq!(decide(Some("a"), None, "a"), Verdict::Adopted);
        assert_eq!(after(Verdict::Adopted, None, "a"), Some(given("a")));
        // An older wording or the owner's: offered, and given no version, so
        // a later start never reads it as untouched and writes over it.
        assert_eq!(decide(Some("old"), None, "a"), Verdict::Offered);
        let claimed = after(Verdict::Offered, None, "a").unwrap();
        assert_eq!(claimed.digest, "");
        assert_eq!(decide(Some("old"), Some(&claimed), "a"), Verdict::Offered);
        assert_eq!(decide(Some("old"), Some(&claimed), "b"), Verdict::Offered);
    }
}
