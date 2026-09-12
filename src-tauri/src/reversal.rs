//! What a change would have to be told to put a row back.
//!
//! An editing operation records the patch it applied. To take that patch back
//! you need the same shape filled with what those fields held before — not the
//! whole row: undoing a rename must not also revert a status somebody set in
//! between, and a patch of every column would do exactly that.
//!
//! So the inverse of a patch is the patch's own keys, read off the row as it
//! was. Both are plain JSON objects here rather than the typed `*Patch`
//! structs, because the rule is the same for all seven of them and a rule
//! written once cannot drift between six copies of itself.

use serde_json::{Map, Value};

/// Read a patch field that can be cleared: absent means "leave it", `null`
/// means "clear it", a value means "set it".
///
/// Serde reads `null` into an `Option<Option<T>>` as the outer `None` — the
/// same as absent — so without this every "clear it" a patch carried, and
/// every inverse [`invert`] wrote for a field that had no value, quietly
/// became "leave it": a note's title could not be cleared, and undoing "put
/// this work in a collection" left it there. The field says
/// `deserialize_with = "crate::reversal::nullable"` and the two are told
/// apart again. Found by the scene's undo test on 2026-09-12; the older
/// patches had the same hole.
pub fn nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    <Option<T> as serde::Deserialize>::deserialize(deserializer).map(Some)
}

/// A patch field whose name is not the column it sets.
///
/// Almost every patch key matches the row's own field, and inverting is then a
/// lookup by the same name. These are the ones where it is not: a flag that
/// sets a timestamp. Reading `bookmarked` off the row would find nothing and
/// invert to `null`, which a patch reads as "leave it" — so the bookmark would
/// survive its own undo, quietly.
///
/// Listed rather than inferred, because there is no rule to infer from: it is a
/// deliberate difference between what a person asks for ("bookmark this") and
/// what the row keeps ("bookmarked at this moment").
const FLAGS_OVER_STAMPS: [(&str, &str); 1] = [("bookmarked", "bookmarked_at")];

/// The patch that puts back what `patch` changed.
///
/// `before` is the row as it stood, serialised; `patch` is what was asked for.
/// Keys the patch does not mention are left out, so applying the result touches
/// nothing else.
///
/// A key the patch sets but the row has no value for comes back as `null`,
/// which is how every patch in this application spells "clear it" — a work that
/// gained a collection is put back to having none.
pub fn invert(before: &Map<String, Value>, patch: &Map<String, Value>) -> Map<String, Value> {
    let mut inverse = Map::new();
    for key in patch.keys() {
        if let Some((_, column)) = FLAGS_OVER_STAMPS.iter().find(|(flag, _)| flag == key) {
            // The flag's inverse is whether the stamp was there, not the stamp.
            let was_set = before.get(*column).is_some_and(|value| !value.is_null());
            inverse.insert(key.clone(), Value::Bool(was_set));
            continue;
        }
        let was = before.get(key).cloned().unwrap_or(Value::Null);
        inverse.insert(key.clone(), was);
    }
    inverse
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().expect("a JSON object").clone()
    }

    #[derive(serde::Deserialize)]
    struct Patch {
        #[serde(default, deserialize_with = "nullable")]
        title: Option<Option<String>>,
    }

    /// The three states a clearable field can be in, told apart on the way
    /// in: absent, null, a value. Without `nullable`, null reads as absent.
    #[test]
    fn a_null_in_a_patch_is_clear_it_and_an_absent_key_is_leave_it() {
        let absent: Patch = serde_json::from_value(json!({})).unwrap();
        let cleared: Patch = serde_json::from_value(json!({ "title": null })).unwrap();
        let set: Patch = serde_json::from_value(json!({ "title": "x" })).unwrap();
        assert!(absent.title.is_none(), "absent is leave it");
        assert_eq!(cleared.title, Some(None), "null is clear it");
        assert_eq!(set.title, Some(Some("x".into())));
    }

    #[test]
    fn the_inverse_names_only_what_the_patch_named() {
        let before = object(json!({
            "title": "Harbour lights",
            "status": "draft",
            "kind": "song",
        }));
        let patch = object(json!({ "title": "Winter road" }));

        let inverse = invert(&before, &patch);

        assert_eq!(inverse, object(json!({ "title": "Harbour lights" })));
    }

    /// The property the whole thing rests on: undoing a rename must not revert
    /// a status somebody changed in between. Inverting the *patch* rather than
    /// the row is what guarantees it, and this is the test that says so.
    #[test]
    fn a_field_the_patch_left_alone_is_not_touched() {
        let before = object(json!({ "title": "Harbour lights", "status": "draft" }));
        let patch = object(json!({ "title": "Winter road" }));

        let inverse = invert(&before, &patch);

        assert!(
            !inverse.contains_key("status"),
            "the inverse would also rewrite `status`, undoing an edit nobody asked about"
        );
    }

    #[test]
    fn several_fields_at_once_all_come_back() {
        let before = object(json!({ "title": "Harbour lights", "kind": "song" }));
        let patch = object(json!({ "title": "Winter road", "kind": "poem" }));

        let inverse = invert(&before, &patch);

        assert_eq!(
            inverse,
            object(json!({ "title": "Harbour lights", "kind": "song" }))
        );
    }

    /// Setting a field that had no value has an inverse too: clear it again.
    #[test]
    fn a_field_that_had_no_value_comes_back_as_null() {
        let before = object(json!({ "title": "Harbour lights" }));
        let patch = object(json!({ "collection_id": "c1" }));

        let inverse = invert(&before, &patch);

        assert_eq!(inverse, object(json!({ "collection_id": null })));
        assert!(
            inverse.get("collection_id").is_some_and(Value::is_null),
            "the key has to be present and null, not absent — absent means `leave it`"
        );
    }

    /// A flag that sets a stamp inverts to a flag, not to the stamp.
    ///
    /// Inverting `bookmarked` by name finds nothing on the row — the column is
    /// `bookmarked_at` — and yields `null`, which a patch reads as "leave it".
    /// The bookmark would then survive its own undo without a word.
    #[test]
    fn a_flag_inverts_to_whether_the_stamp_was_set() {
        let bookmarked = object(json!({ "bookmarked_at": "2026-09-09T10:00:00.000Z" }));
        let plain = object(json!({ "bookmarked_at": null }));
        let patch = object(json!({ "bookmarked": true }));

        assert_eq!(
            invert(&bookmarked, &patch),
            object(json!({ "bookmarked": true })),
            "a work that was already bookmarked stays bookmarked"
        );
        assert_eq!(
            invert(&plain, &patch),
            object(json!({ "bookmarked": false })),
            "a work that was not bookmarked has the bookmark taken off again"
        );
    }

    #[test]
    fn an_empty_patch_inverts_to_an_empty_patch() {
        let before = object(json!({ "title": "Harbour lights" }));

        assert!(invert(&before, &Map::new()).is_empty());
    }
}
