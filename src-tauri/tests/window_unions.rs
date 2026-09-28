//! Holds the words the window has for the backend's enums to the enums.
//!
//! Until v0.83 the window mirrored several Rust enums as hand-written string
//! unions, and two had come apart: a comment deleted in v0.76 reached the
//! trash as a kind the window's union did not have, and a day's verdict was
//! still spelled `displaces` and `held` from before v0.44 while the backend
//! sent `taken`. This file compared those unions with the enums. Since v0.83
//! the unions are generated from the enums themselves (`tests/bindings.rs`,
//! ADR 0042), so they cannot disagree; what is left to hold is that every
//! member has a word in each language.

use std::path::{Path, PathBuf};

use kilna_lib::trash::Entity;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri always has a parent")
        .to_path_buf()
}

fn wire_name<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("serialises")
        .as_str()
        .expect("serialises to a string")
        .to_owned()
}

#[test]
fn every_kind_the_trash_holds_has_a_word() {
    for locale in ["en", "ru"] {
        let path = repo_root().join(format!("src/i18n/locales/{locale}.json"));
        let source = std::fs::read_to_string(&path).expect("the locale is readable");
        let json: serde_json::Value =
            serde_json::from_str(&source).expect("the locale is valid JSON");
        let words = json["trash"]["entity"]
            .as_object()
            .unwrap_or_else(|| panic!("{locale}.json has no trash.entity section"));
        for entity in Entity::ALL {
            let kind = wire_name(&entity);
            assert!(
                words
                    .get(&kind)
                    .and_then(|word| word.as_str())
                    .is_some_and(|word| !word.is_empty()),
                "{locale}.json has no trash.entity.{kind}: that kind shows in the trash with no word"
            );
        }
    }
}
