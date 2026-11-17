//! The five stops of how finished a work is (FR-03).

use serde::{Deserialize, Serialize};

use super::*;

/// One stop on the way from an idea to a finished work.
///
/// The third thing a work carries about itself, beside the derived status and
/// the hand-raised marks, and it overlaps neither. A status says where the work
/// stands in the *process* and is worked out from facts — it was scored, it was
/// booked, it shipped. A stage says how finished the *work itself* is, which no
/// fact can answer: only the author knows that a song with a full lyric is
/// still three verses of placeholder.
///
/// `percent` is what is stored on the work, and what the dial draws. The key
/// and the label are how a person speaks about it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
pub struct Stage {
    pub key: String,
    pub label: Label,
    /// Where this stop sits, 0..=100.
    pub percent: i64,
    /// One of the palette's own roles, so a stage reads correctly in both
    /// themes.
    #[serde(default)]
    pub colour: MarkColour,
    /// The stop's exact colour, `#RRGGBB` (v0.119). The five stops are told
    /// apart by colour alone - no word stands beside a dial - and the
    /// palette has fewer roles than the scale has stops. Where it is set the
    /// dial draws it; `colour` stays for a stop that names none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub hex: Option<String>,
}

/// The five places a work can stand at (FR-03): a quarter each. One scale
/// for every craft, so a dial reads the same in every profile and a stage
/// can be compared across kinds; the words at each stop are the profile's.
pub const STOPS: [i64; 5] = [0, 25, 50, 75, 100];

/// A stage as it is kept: one of [`STOPS`].
///
/// Whatever arrives - nothing at all, a number from the seven-stop scale a
/// work was given before v0.119, one read out of an old line of the
/// operation log, a parcel written by an earlier release - lands on the stop
/// nearest to it. "Nobody has said" is the first stop: a work that was never
/// given a stage and a bare idea are the same thing to anyone looking for
/// what to pick up, and two states that draw alike were one too many.
pub fn normalize_stage(stage: Option<i64>) -> i64 {
    match stage {
        None => 0,
        Some(value) if value <= 12 => 0,
        Some(value) if value <= 37 => 25,
        Some(value) if value <= 62 => 50,
        Some(value) if value <= 87 => 75,
        Some(_) => 100,
    }
}

/// The five stops as the line words them: what a craft gets when it names
/// none of its own, and what every shipped profile carries (FR-03).
///
/// Five, where there were seven: nobody could say what told "Nearly there"
/// from "Polishing", and a scale whose neighbours cannot be told apart is
/// read by nobody. The words are what a dial says when it is pointed at -
/// "25% - started / first drafts" - since no word stands beside one.
///
/// The last stop is "final", not "released": the stage stays a judgement
/// made by hand, and the fact of a release is what the status carries.
pub fn default_stages() -> Vec<Stage> {
    [
        (
            "idea",
            "idea / untouched",
            "идея / ещё не трогали",
            0,
            MarkColour::Plain,
            "#6E737D",
        ),
        (
            "start",
            "started / first drafts",
            "старт / первые наброски",
            25,
            MarkColour::Info,
            "#5CA3FA",
        ),
        (
            "half",
            "halfway / working draft",
            "половина / рабочий черновик",
            50,
            MarkColour::Warn,
            "#E8B341",
        ),
        (
            "almost",
            "almost ready / polishing",
            "почти готово / доводка и полировка",
            75,
            MarkColour::Accent,
            "#B084F7",
        ),
        (
            "final",
            "final / ready to ship",
            "финал, можно выпускать",
            100,
            MarkColour::Good,
            "#5EC486",
        ),
    ]
    .into_iter()
    .map(|(key, en, ru, percent, colour, hex)| Stage {
        key: key.to_owned(),
        label: Label::PerLocale(
            [("en", en), ("ru", ru)]
                .into_iter()
                .map(|(language, word)| (language.to_owned(), word.to_owned()))
                .collect(),
        ),
        percent,
        colour,
        hex: Some(hex.to_owned()),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Studio as it ships.
    fn studio() -> ProfileConfig {
        let conn = crate::db::open_in_memory().unwrap();
        crate::profile::seed(&conn, crate::profile::DEFAULT_KIND).unwrap();
        crate::profile::active(&conn).unwrap().unwrap().config
    }

    #[test]
    fn a_stage_lands_on_the_nearest_of_the_five_stops() {
        // Nothing at all is the first stop: "nobody has said" is gone.
        assert_eq!(normalize_stage(None), 0);
        // The seven stops a work could stand at until v0.119.
        for (before, after) in [
            (0, 0),
            (17, 25),
            (33, 25),
            (50, 50),
            (67, 75),
            (83, 75),
            (100, 100),
        ] {
            assert_eq!(normalize_stage(Some(before)), after, "{before}");
        }
        // The thresholds, each side of each.
        for (before, after) in [
            (12, 0),
            (13, 25),
            (37, 25),
            (38, 50),
            (62, 50),
            (63, 75),
            (87, 75),
            (88, 100),
        ] {
            assert_eq!(normalize_stage(Some(before)), after, "{before}");
        }
        // Off the dial altogether.
        assert_eq!(normalize_stage(Some(-5)), 0);
        assert_eq!(normalize_stage(Some(140)), 100);
        // And what is on the scale stays where it is.
        for stop in STOPS {
            assert_eq!(normalize_stage(Some(stop)), stop);
        }
    }

    #[test]
    fn the_lines_own_stops_are_the_five_each_with_its_colour_and_both_languages() {
        let stops = default_stages();
        assert_eq!(
            stops.iter().map(|stop| stop.percent).collect::<Vec<_>>(),
            STOPS
        );
        assert_eq!(
            stops
                .iter()
                .map(|stop| stop.hex.as_deref().unwrap())
                .collect::<Vec<_>>(),
            ["#6E737D", "#5CA3FA", "#E8B341", "#B084F7", "#5EC486"]
        );
        for stop in &stops {
            let Label::PerLocale(words) = &stop.label else {
                panic!("{} is said in one language", stop.key);
            };
            assert!(words.contains_key("en") && words.contains_key("ru"));
        }
    }

    #[test]
    fn a_stop_off_the_scale_two_on_one_place_and_a_colour_that_is_not_one_are_problems() {
        let mut config = studio();
        config.stages = default_stages();
        assert!(config.problems().is_empty());

        config.stages[1].percent = 17;
        config.stages[2].percent = 75;
        config.stages[4].hex = Some("green".into());
        let codes: Vec<String> = config
            .problems()
            .iter()
            .map(|problem| problem.key.clone())
            .collect();
        assert_eq!(
            codes,
            [
                "problem.stageOffScale",
                "problem.stageSharesStop",
                "problem.stageColour"
            ]
        );
    }

    #[test]
    fn the_lines_of_a_vibe_are_five_colours_five_shapes_and_three_weights() {
        let axes = default_vibe();
        assert_eq!(
            (
                axes.moods.len(),
                axes.characters.len(),
                axes.complexities.len()
            ),
            (5, 5, 3)
        );
        // No mood wears a colour of the stage dial, which stands beside it.
        let stages: Vec<String> = default_stages()
            .into_iter()
            .filter_map(|stop| stop.hex)
            .collect();
        let colours: std::collections::BTreeSet<&str> =
            axes.moods.iter().map(|mood| mood.color.as_str()).collect();
        assert_eq!(colours.len(), 5, "five colours, told apart");
        assert!(
            colours
                .iter()
                .all(|colour| !stages.iter().any(|s| s == colour))
        );
        assert_eq!(
            axes.complexities
                .iter()
                .map(|c| c.strokes)
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        // A line unsaid stands at its middle while another is said.
        let neutral = axes.neutral();
        assert_eq!(
            (
                neutral.mood.as_deref(),
                neutral.character.as_deref(),
                neutral.complexity.as_deref()
            ),
            (Some("contemplative"), Some("steady"), Some("medium"))
        );

        let mut config = studio();
        config.vibe = Some(axes);
        assert!(config.problems().is_empty());
        let axes = config.vibe.as_mut().unwrap();
        axes.moods[0].color = "indigo".into();
        axes.moods[1].key = axes.moods[2].key.clone();
        axes.complexities[2].strokes = 4;
        axes.characters.clear();
        let codes: Vec<String> = config
            .problems()
            .iter()
            .map(|problem| problem.key.clone())
            .collect();
        assert_eq!(
            codes,
            [
                "problem.repeatedKey",
                "problem.vibeEmpty",
                "problem.vibeColour",
                "problem.vibeStrokes"
            ]
        );
    }

    #[test]
    fn a_profile_naming_some_of_the_five_answers_for_the_rest_with_the_lines_own() {
        let mut config = studio();
        config.stages = default_stages();
        config.stages.retain(|stop| stop.percent != 50);
        config.stages[0].label = Label::from("Seed");

        assert_eq!(config.stage_at(0).unwrap().label, "Seed");
        assert_eq!(config.stage_at(50).unwrap().key, "half");
        // A number from an older scale reads as the stop it lands on.
        assert_eq!(config.stage_at(83).unwrap().key, "almost");
    }
}
