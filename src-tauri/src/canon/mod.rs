//! The canon: the world a channel's works are made from, kept as facts.
//!
//! A card is a note of a kind the profile names with sections (ADR 0043) - a
//! character, a place, an object, the channel itself. What it knows is not a
//! paragraph but [`fact`]s: short statements, each with the layer it may be
//! told in, how settled it is, where it came from and when it happened in the
//! world, filed under a section of the card. Cards stand in [`link`]s to one
//! another. Where a card [`appears`](appearances) is counted from the works,
//! never written down.
//!
//! The point of the rows is that they can be read *for a task*. A cover may
//! see the public layer of a person's looks; a lyric may see everything, the
//! internal layer without its addresses; the text a release goes out under
//! may see only the public layer and the formulas meant for the public. That
//! reading is one function here, [`seen_by`], and the screen, the MCP tool and
//! every prompt read through it - so the screen cannot show one thing while
//! the generator is handed another.

pub mod appearances;
#[cfg(test)]
mod behaviour;
pub mod fact;
pub mod link;
pub mod proposal;
pub mod view;
pub mod when;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::profile::config::{CanonSection, Lens};

pub use fact::{Fact, FactPatch, NewFact, Source, SourceKind, When};
pub use link::{CanonLink, CanonLinkPatch, NewCanonLink};

/// Who may be told a fact, a relation or that a card exists at all.
///
/// The canon's three layers, the same for every craft: what is said in public;
/// what the works draw on without its addresses and never say outright; and
/// what exists only inside the works themselves. The code reads them, which is
/// why they are not words of the profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum Layer {
    /// Said anywhere: a caption, a reply, a cover. Nothing in it can be
    /// checked by a visit, a call or a search.
    #[default]
    Public,
    /// Full names, dates, circumstances. Never said outside; a work takes
    /// details from it, without the addresses.
    Internal,
    /// What exists only inside the works - a loss, a love. Asked about
    /// directly, it is answered with the formulas meant for that, word for
    /// word.
    InWorks,
}

impl Layer {
    pub const ALL: [Layer; 3] = [Layer::Public, Layer::Internal, Layer::InWorks];

    /// The word the database and the wire carry.
    pub fn as_str(self) -> &'static str {
        match self {
            Layer::Public => "public",
            Layer::Internal => "internal",
            Layer::InWorks => "inWorks",
        }
    }

    /// A stored layer. An unknown word is a row the schema should not have let
    /// in, not a person's mistake.
    pub fn parse(raw: &str) -> Result<Self> {
        Self::from_word(raw).ok_or_else(|| Error::Internal(format!("a stored layer reads `{raw}`")))
    }

    /// A layer as an agent or a caller names it; none for any other word.
    pub fn from_word(raw: &str) -> Option<Self> {
        match raw.trim() {
            "public" => Some(Layer::Public),
            "internal" => Some(Layer::Internal),
            "inWorks" | "in_works" | "in-works" => Some(Layer::InWorks),
            _ => None,
        }
    }
}

/// How settled a fact is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum FactStatus {
    /// True of the world.
    #[default]
    Canon,
    /// A live zone: deliberately left open, to be built on by the works.
    Open,
    /// Written down, not yet settled.
    Draft,
    /// No longer true, with the reason it was retired - kept, so the next
    /// picture does not bring it back.
    Retired,
}

impl FactStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            FactStatus::Canon => "canon",
            FactStatus::Open => "open",
            FactStatus::Draft => "draft",
            FactStatus::Retired => "retired",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        Self::from_word(raw)
            .ok_or_else(|| Error::Internal(format!("a stored status reads `{raw}`")))
    }

    pub fn from_word(raw: &str) -> Option<Self> {
        match raw.trim() {
            "canon" => Some(FactStatus::Canon),
            "open" => Some(FactStatus::Open),
            "draft" => Some(FactStatus::Draft),
            "retired" => Some(FactStatus::Retired),
            _ => None,
        }
    }
}

/// Whether `lens` may read a fact of `layer` and `status`, filed under
/// `section`, of a card of `card_layer`.
///
/// The one rule the screen, the MCP tool and every prompt read through. A work
/// reads everything that is still true or still being built - a draft and a
/// live zone included, the retired left out. A cover and a public text read
/// only what is settled and public, of a card that is public itself, in a
/// section the craft gave them.
pub fn seen_by(
    lens: Lens,
    card_layer: Layer,
    section: Option<&CanonSection>,
    layer: Layer,
    status: FactStatus,
) -> bool {
    if status == FactStatus::Retired {
        return false;
    }
    if lens == Lens::Work {
        return true;
    }
    card_layer == Layer::Public
        && layer == Layer::Public
        && status == FactStatus::Canon
        && section.is_some_and(|section| section.read_by(lens))
}

/// The lenses that read a fact, in a fixed order - what the screen dims by.
pub fn lenses_of(
    card_layer: Layer,
    section: Option<&CanonSection>,
    layer: Layer,
    status: FactStatus,
) -> Vec<Lens> {
    Lens::ALL
        .into_iter()
        .filter(|lens| seen_by(*lens, card_layer, section, layer, status))
        .collect()
}

/// A stable fingerprint of text: FNV-1a, 64 bits, as hex.
///
/// What a description's basis is compared by. Not a security measure - it
/// only has to say "these are not the facts it was written from" - and a
/// dependency for twenty lines would be the wrong trade.
pub fn fingerprint(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(lenses: &[Lens]) -> CanonSection {
        let mut section = CanonSection::new("looks", "Looks");
        section.lenses = lenses.to_vec();
        section
    }

    #[test]
    fn a_work_reads_every_layer_but_not_what_was_retired() {
        let looks = section(&[]);
        for layer in Layer::ALL {
            assert!(seen_by(
                Lens::Work,
                Layer::Internal,
                Some(&looks),
                layer,
                FactStatus::Draft
            ));
        }
        assert!(!seen_by(
            Lens::Work,
            Layer::Public,
            Some(&looks),
            Layer::Public,
            FactStatus::Retired
        ));
    }

    #[test]
    fn a_cover_reads_only_the_settled_public_layer_of_its_sections() {
        let looks = section(&[Lens::Cover]);
        let bio = section(&[Lens::Public]);
        let yes = |section: &CanonSection| {
            seen_by(
                Lens::Cover,
                Layer::Public,
                Some(section),
                Layer::Public,
                FactStatus::Canon,
            )
        };
        assert!(yes(&looks));
        assert!(!yes(&bio), "a section not given to the cover");
        assert!(!seen_by(
            Lens::Cover,
            Layer::Public,
            Some(&looks),
            Layer::Internal,
            FactStatus::Canon
        ));
        assert!(!seen_by(
            Lens::Cover,
            Layer::Public,
            Some(&looks),
            Layer::Public,
            FactStatus::Draft
        ));
    }

    #[test]
    fn a_card_that_publicly_does_not_exist_is_seen_by_no_public_task() {
        let public = section(&[Lens::Public, Lens::Cover]);
        for lens in [Lens::Public, Lens::Cover] {
            assert!(!seen_by(
                lens,
                Layer::Internal,
                Some(&public),
                Layer::Public,
                FactStatus::Canon
            ));
        }
    }

    #[test]
    fn a_fact_under_a_section_the_profile_dropped_is_read_by_the_work_alone() {
        assert!(seen_by(
            Lens::Work,
            Layer::Public,
            None,
            Layer::Public,
            FactStatus::Canon
        ));
        assert!(!seen_by(
            Lens::Public,
            Layer::Public,
            None,
            Layer::Public,
            FactStatus::Canon
        ));
    }

    #[test]
    fn the_fingerprint_is_stable_and_tells_texts_apart() {
        assert_eq!(fingerprint("a"), fingerprint("a"));
        assert_ne!(fingerprint("a"), fingerprint("b"));
        assert_eq!(fingerprint(""), "cbf29ce484222325");
    }

    #[test]
    fn a_layer_reads_back_the_word_it_is_stored_as() {
        for layer in Layer::ALL {
            assert_eq!(Layer::parse(layer.as_str()).unwrap(), layer);
            assert_eq!(
                serde_json::to_value(layer).unwrap(),
                serde_json::json!(layer.as_str())
            );
        }
        assert!(Layer::parse("secret").is_err());
    }
}
