//! What a change would have to be told to put a row back.
//!
//! An editing operation records the patch it applied. To take that patch back
//! you need the same shape filled with what those fields held before — not the
//! whole row: undoing a rename must not also revert a status somebody set in
//! between, and a patch of every column would do exactly that.
//!
//! So the inverse of a patch is the patch's own keys, read off the row as it
//! was. Both are plain JSON objects here rather than the typed `*Patch`
//! structs, because the rule is the same for all of them and a rule written
//! once cannot drift between nine copies of itself.
//!
//! The same rule goes one level down for a field a patch merges into rather
//! than replaces - a work's `meta`, since v0.82. A patch that sets the tempo
//! names only the tempo, so its inverse names only the tempo too: the key it
//! would otherwise record is the whole object, and undoing the tempo would
//! put back the key signature someone set a minute later.

use serde_json::{Map, Value};

/// A patch the log records the inverse of.
///
/// Each patch says which of its fields merge into the row by key rather than
/// replace it whole, because the two need different inverses and nothing in
/// the JSON tells them apart: a work's `meta` and a release's `meta` are both
/// an object of fields, and only the work's merges. Implemented below for
/// every patch in the application rather than beside each one, so the list of
/// which merge is read in one place - and a patch that has not said cannot be
/// recorded at all, since [`before_of`] asks for this trait.
pub trait Patch: serde::Serialize {
    /// Fields whose value is an object merged into the row's by key: a key
    /// with a value sets it, a key with `null` removes it, and a key the
    /// patch does not name is left as it is.
    const MERGED_BY_KEY: &'static [&'static str] = &[];
}

impl Patch for crate::work::WorkPatch {
    const MERGED_BY_KEY: &'static [&'static str] = &["meta"];
}
impl Patch for crate::note::NotePatch {}
impl Patch for crate::comment::CommentPatch {}
impl Patch for crate::style_brick::StyleBrickPatch {}
impl Patch for crate::focus::FocusNotePatch {}
impl Patch for crate::release::ReleasePatch {}
impl Patch for crate::collection::CollectionPatch {}
impl Patch for crate::scene::ScenePatch {}
impl Patch for crate::cut::CutPatch {}
impl Patch for crate::canon::FactPatch {}
impl Patch for crate::canon::CanonLinkPatch {}

/// What the fields a patch names held before it was applied, as the log
/// records it beside the patch - see [`invert`] and [`invert_merging`].
///
/// A row that is not there yields an empty object rather than an error. The
/// change about to be attempted will fail on its own and say so properly; a
/// log helper is not the place to decide that.
pub fn before_of<T: serde::Serialize, P: Patch>(
    before: Option<&T>,
    patch: &P,
) -> crate::error::Result<Value> {
    let (Some(before), Value::Object(patch)) = (before, serde_json::to_value(patch)?) else {
        return Ok(Value::Object(Map::new()));
    };
    let Value::Object(before) = serde_json::to_value(before)? else {
        return Ok(Value::Object(Map::new()));
    };
    Ok(Value::Object(invert_merging(
        &before,
        &patch,
        P::MERGED_BY_KEY,
    )))
}

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
    invert_merging(before, patch, &[])
}

/// [`invert`], for a patch some of whose fields merge by key.
///
/// A field named in `merged_by_key` is inverted one level down by the same
/// rule: the keys the patch sent, each with what the row's object held under
/// it, and `null` for a key it did not have - which the merge reads as
/// "remove it", so a field the patch added is taken away again and every
/// field it did not send is left alone.
pub fn invert_merging(
    before: &Map<String, Value>,
    patch: &Map<String, Value>,
    merged_by_key: &[&str],
) -> Map<String, Value> {
    let mut inverse = Map::new();
    for (key, sent) in patch {
        if let Some((_, column)) = FLAGS_OVER_STAMPS.iter().find(|(flag, _)| flag == key) {
            // The flag's inverse is whether the stamp was there, not the stamp.
            let was_set = before.get(*column).is_some_and(|value| !value.is_null());
            inverse.insert(key.clone(), Value::Bool(was_set));
            continue;
        }
        if let (true, Value::Object(sent)) = (merged_by_key.contains(&key.as_str()), sent) {
            let held = before.get(key).and_then(Value::as_object);
            let was: Map<String, Value> = sent
                .keys()
                .map(|field| {
                    let value = held.and_then(|held| held.get(field)).cloned();
                    (field.clone(), value.unwrap_or(Value::Null))
                })
                .collect();
            inverse.insert(key.clone(), Value::Object(was));
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

    /// A field merged by key inverts to the keys the patch sent, not to the
    /// object it held: undoing the tempo must not put back a key signature
    /// set after it.
    #[test]
    fn a_field_merged_by_key_inverts_only_the_keys_sent() {
        let before = object(json!({
            "title": "Harbour lights",
            "meta": { "bpm": 92, "key": "Am" },
        }));
        let patch = object(json!({ "meta": { "bpm": 120, "mood": "dark" } }));

        assert_eq!(
            invert_merging(&before, &patch, &["meta"]),
            object(json!({ "meta": { "bpm": 92, "mood": null } })),
            "the tempo goes back, the added field is removed, the key is not named"
        );
        assert_eq!(
            invert(&before, &patch),
            object(json!({ "meta": { "bpm": 92, "key": "Am" } })),
            "a field that is replaced whole still inverts to the whole object"
        );
    }

    /// A work that never had fields inverts to removing each one it gained.
    #[test]
    fn a_merged_field_the_row_did_not_have_inverts_to_removals() {
        let before = object(json!({ "title": "Harbour lights" }));
        let patch = object(json!({ "meta": { "bpm": 120 } }));

        assert_eq!(
            invert_merging(&before, &patch, &["meta"]),
            object(json!({ "meta": { "bpm": null } }))
        );
    }

    /// Which patches merge is said by the patch, and only the work's does.
    #[test]
    fn only_the_work_merges_its_meta_by_key() {
        assert_eq!(
            <crate::work::WorkPatch as super::Patch>::MERGED_BY_KEY,
            ["meta"]
        );
        assert!(<crate::release::ReleasePatch as super::Patch>::MERGED_BY_KEY.is_empty());
    }

    #[test]
    fn an_empty_patch_inverts_to_an_empty_patch() {
        let before = object(json!({ "title": "Harbour lights" }));

        assert!(invert(&before, &Map::new()).is_empty());
    }
}
