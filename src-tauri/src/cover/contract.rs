//! One rule for whether a chosen brick goes into a picture (v0.134, FR-07
//! 7.2).
//!
//! A brick says what the picture must be for it to apply, what it brings, and
//! what it cannot stand beside - its [`Contract`]. The picture says what it is
//! as tags: who it is of, how much of them shows, whether the ground is flat,
//! whether a title is lettered, whether the camera holds still. The resolver
//! reads the chosen bricks in their order against those tags and what the
//! bricks kept before them bring, and keeps each one whose needs hold and
//! whose conflicts do not meet anything in the picture.
//!
//! A brick left out is not taken out of the record: the owner chose it, and
//! the choice stays for when the picture changes. It is left out of the
//! prompt and said, with why and what would let it in. A new rule - day
//! against night, one style against another - is tags in a set or a pack, not
//! code here.

use std::collections::BTreeSet;

use serde::Serialize;

use super::framing::{Crop, Size, Subject};
use crate::style_brick::Contract;

/// The crops from the least of the subject to the most: a need of
/// `crop:min:bust` holds for a bust, a waist and a full figure.
pub const CROPS: [&str; 5] = ["detail", "head", "bust", "waist", "full"];

/// The tag of a crop - and of a size that fills the frame with a face or the
/// key object, which shows as little as a detail does.
pub fn crop_tag(crop: Crop, size: Size) -> String {
    let word = if size == Size::Macro {
        "detail"
    } else {
        match crop {
            Crop::Detail => "detail",
            Crop::Head => "head",
            Crop::Bust => "bust",
            Crop::Waist => "waist",
            Crop::Full => "full",
        }
    };
    format!("crop:{word}")
}

/// The tag of what a picture is of.
pub fn subject_tag(subject: Subject) -> &'static str {
    match subject {
        Subject::Person => "subject:person",
        Subject::Object => "subject:object",
        Subject::None => "subject:none",
    }
}

/// Where a crop stands in [`CROPS`].
fn rank(word: &str) -> Option<usize> {
    CROPS.iter().position(|one| *one == word)
}

/// Whether a need holds among the picture's tags.
///
/// `crop:min:X` and `crop:max:X` read the order of crops; a picture with no
/// crop said - no frame built - holds them, as nothing in it says otherwise.
/// Any other need is a tag the picture has.
pub fn holds(need: &str, tags: &BTreeSet<String>) -> bool {
    let crop = || {
        tags.iter()
            .find_map(|tag| tag.strip_prefix("crop:").and_then(rank))
    };
    if let Some(least) = need.strip_prefix("crop:min:").and_then(rank) {
        return crop().is_none_or(|shown| shown >= least);
    }
    if let Some(most) = need.strip_prefix("crop:max:").and_then(rank) {
        return crop().is_none_or(|shown| shown <= most);
    }
    tags.contains(need)
}

/// A brick in its place, as the resolver reads it.
#[derive(Debug, Clone, Copy)]
pub struct Chosen<'a> {
    /// The place it is chosen for: `style`, `lighting`, `motion`.
    pub place: &'a str,
    pub contract: &'a Contract,
}

/// What became of a chosen brick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(rename = "ContractVerdict")]
pub enum Verdict {
    Kept,
    /// A need the picture does not meet.
    Unmet {
        need: String,
    },
    /// A tag it cannot stand beside: the picture's own, or one a brick kept
    /// before it brings - then `by` is that brick's index among the chosen.
    Conflict {
        tag: String,
        by: Option<usize>,
    },
}

/// Read the chosen bricks in their order against the picture's tags: one
/// verdict each, in the same order. A kept brick's `provides` join the tags
/// the bricks after it are read against, so the order is the priority - the
/// style before the light, the light before the grade.
pub fn resolve(context: &BTreeSet<String>, chosen: &[Chosen<'_>]) -> Vec<Verdict> {
    let mut tags = context.clone();
    // Which chosen brick brought each tag a kept brick added.
    let mut brought: Vec<(String, usize)> = Vec::new();
    let mut kept: Vec<(usize, Chosen<'_>)> = Vec::new();
    let mut verdicts = Vec::with_capacity(chosen.len());
    for (index, one) in chosen.iter().enumerate() {
        let contract = one.contract;
        if let Some(need) = contract.needs.iter().find(|need| !holds(need, &tags)) {
            verdicts.push(Verdict::Unmet { need: need.clone() });
            continue;
        }
        if let Some(tag) = contract.conflicts.iter().find(|tag| tags.contains(*tag)) {
            let by = brought
                .iter()
                .find(|(given, _)| given == tag)
                .map(|(_, at)| *at);
            verdicts.push(Verdict::Conflict {
                tag: tag.clone(),
                by,
            });
            continue;
        }
        // A brick kept before that cannot stand what this one brings.
        if let Some((at, tag)) = kept.iter().find_map(|(at, other)| {
            contract
                .provides
                .iter()
                .find(|tag| other.contract.conflicts.contains(tag))
                .map(|tag| (*at, tag))
        }) {
            verdicts.push(Verdict::Conflict {
                tag: tag.clone(),
                by: Some(at),
            });
            continue;
        }
        for tag in &contract.provides {
            if tags.insert(tag.clone()) {
                brought.push((tag.clone(), index));
            }
        }
        kept.push((index, *one));
        verdicts.push(Verdict::Kept);
    }
    verdicts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|tag| (*tag).to_owned()).collect()
    }

    fn contract(needs: &[&str], provides: &[&str], conflicts: &[&str]) -> Contract {
        let words = |list: &[&str]| list.iter().map(|one| (*one).to_owned()).collect();
        Contract {
            needs: words(needs),
            provides: words(provides),
            conflicts: words(conflicts),
        }
    }

    #[test]
    fn a_crop_need_reads_the_order_of_crops() {
        for (shown, holds_bust) in [
            ("detail", false),
            ("head", false),
            ("bust", true),
            ("waist", true),
            ("full", true),
        ] {
            let picture = tags(&[&format!("crop:{shown}")]);
            assert_eq!(holds("crop:min:bust", &picture), holds_bust, "{shown}");
        }
        assert!(holds("crop:max:head", &tags(&["crop:detail"])));
        assert!(!holds("crop:max:head", &tags(&["crop:waist"])));
        assert!(
            holds("crop:min:full", &tags(&[])),
            "no frame says nothing against it"
        );
        assert_eq!(
            crop_tag(Crop::Full, Size::Macro),
            "crop:detail",
            "a macro is a detail"
        );
    }

    #[test]
    fn a_brick_whose_need_fails_is_left_out_and_said() {
        let pose = contract(&["subject:person", "crop:min:waist"], &[], &[]);
        let verdicts = resolve(
            &tags(&["subject:person", "crop:head"]),
            &[Chosen {
                place: "pose",
                contract: &pose,
            }],
        );
        assert_eq!(
            verdicts,
            [Verdict::Unmet {
                need: "crop:min:waist".into()
            }]
        );
        let verdicts = resolve(
            &tags(&["subject:object"]),
            &[Chosen {
                place: "pose",
                contract: &pose,
            }],
        );
        assert_eq!(
            verdicts,
            [Verdict::Unmet {
                need: "subject:person".into()
            }]
        );
    }

    #[test]
    fn what_a_kept_brick_brings_is_read_by_the_ones_after_it() {
        let noir = contract(&[], &["color:mono"], &[]);
        let neon = contract(&[], &["color:vivid"], &["color:mono"]);
        let verdicts = resolve(
            &tags(&[]),
            &[
                Chosen {
                    place: "style",
                    contract: &noir,
                },
                Chosen {
                    place: "grading",
                    contract: &neon,
                },
            ],
        );
        assert_eq!(
            verdicts,
            [
                Verdict::Kept,
                Verdict::Conflict {
                    tag: "color:mono".into(),
                    by: Some(0)
                }
            ]
        );
    }

    #[test]
    fn a_conflict_is_read_both_ways() {
        // The first kept brick cannot stand what the second brings.
        let tripod = contract(&[], &["camera:static"], &["camera:moving"]);
        let push = contract(&[], &["camera:moving"], &[]);
        let verdicts = resolve(
            &tags(&[]),
            &[
                Chosen {
                    place: "motion",
                    contract: &tripod,
                },
                Chosen {
                    place: "motion",
                    contract: &push,
                },
            ],
        );
        assert_eq!(
            verdicts[1],
            Verdict::Conflict {
                tag: "camera:moving".into(),
                by: Some(0)
            }
        );
    }

    #[test]
    fn a_picture_s_own_tag_keeps_out_what_conflicts_with_it() {
        let push = contract(&[], &["camera:moving"], &["camera:static"]);
        let verdicts = resolve(
            &tags(&["camera:static"]),
            &[Chosen {
                place: "motion",
                contract: &push,
            }],
        );
        assert_eq!(
            verdicts,
            [Verdict::Conflict {
                tag: "camera:static".into(),
                by: None
            }]
        );
        let free = Contract::default();
        assert_eq!(
            resolve(
                &tags(&["camera:static"]),
                &[Chosen {
                    place: "style",
                    contract: &free
                }]
            ),
            [Verdict::Kept],
            "a brick with no contract goes anywhere"
        );
    }
}
