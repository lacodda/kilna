use super::*;
use serde_json::json;

/// Which roles are a time the work itself was written.
///
/// Three answers, and the third is the one that was wrong: a style prompt
/// comments on nothing, so the old rule counted it, and a song written
/// twice read as having many versions. The profile now says outright.
#[test]
fn a_role_says_whether_it_is_a_draft_of_the_work() {
    let draft = VersionRole::new("lyrics", "Lyrics");
    assert!(
        draft.counts_as_a_version(),
        "a plain body is a time the work was written"
    );

    let mut critique = VersionRole::new("critique", "Critique");
    critique.comments_on = Some("lyrics".into());
    assert!(
        !critique.counts_as_a_version(),
        "something written ABOUT another role is not a draft of the work"
    );

    let mut style = VersionRole::new("style", "Style prompt");
    style.counts_as_version = Some(false);
    assert!(
        !style.counts_as_a_version(),
        "the profile's explicit `false` wins over commenting on nothing"
    );

    let mut counted = VersionRole::new("counted", "Counted");
    counted.comments_on = Some("lyrics".into());
    counted.counts_as_version = Some(true);
    assert!(
        counted.counts_as_a_version(),
        "the profile's explicit `true` wins over commenting on something"
    );
}

fn config() -> ProfileConfig {
    serde_json::from_value(json!({
        "work_kinds": [{ "key": "song", "label": "Song" }],
        "release_kinds": [{ "key": "clip", "label": "Clip" }],
        "collection_kinds": [{ "key": "album", "label": "Album" }],
        "version_roles": [{ "key": "lyrics", "label": "Lyrics" }],
        "statuses": [{ "key": "draft", "label": "Draft" }],
        "axes": [
            { "key": "hook", "label": "Hook", "weight": 2.0, "scale": 10.0 },
            { "key": "text", "label": "Text", "weight": 1.0, "scale": 10.0 }
        ],
        "tiers": [
            { "key": "hold", "label": "Hold", "min": 0.0 },
            { "key": "clip", "label": "Clip", "min": 75.0 }
        ],
        "work_meta_fields": []
    }))
    .unwrap()
}

// A profile from before rubrics existed must load, with the axis simply
// saying nothing about what its marks mean.
#[test]
fn an_axis_without_a_rubric_parses_as_naming_no_marks() {
    assert!(config().work_kinds[0].axes[0].rubric.is_empty());
}

#[test]
fn a_rubric_names_marks_on_the_axis_scale() {
    let axis: Axis = serde_json::from_value(json!({
        "key": "hook", "label": "Hook", "weight": 2.0, "scale": 10.0,
        "rubric": [
            { "at": 3.0, "label": "audible, but it does not catch" },
            { "at": 7.0, "label": "the chorus sticks on the first listen" }
        ]
    }))
    .unwrap();

    assert_eq!(axis.rubric.len(), 2);
    assert_eq!(axis.rubric[1].at, 7.0);
}

#[test]
fn a_rubric_mark_off_the_scale_is_refused() {
    let mut config = config();
    config.work_kinds[0].axes[0].rubric = vec![AxisMark {
        at: 12.0,
        label: "beyond the scale".into(),
    }];

    let problems = config.validate();
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("outside 0-10")),
        "got {problems:?}"
    );
}

#[test]
fn a_rubric_naming_the_same_mark_twice_is_refused() {
    let mut config = config();
    config.work_kinds[0].axes[0].rubric = vec![
        AxisMark {
            at: 7.0,
            label: "one sentence".into(),
        },
        AxisMark {
            at: 7.0,
            label: "and another".into(),
        },
    ];

    let problems = config.validate();
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("names 7 twice")),
        "got {problems:?}"
    );
}

#[test]
fn a_sound_rubric_is_not_refused() {
    let mut config = config();
    config.work_kinds[0].axes[0].rubric = vec![
        AxisMark {
            at: 0.0,
            label: "the bottom of the scale is a verdict too".into(),
        },
        AxisMark {
            at: 10.0,
            label: "and so is the top".into(),
        },
    ];

    assert!(config.validate().is_empty(), "got {:?}", config.validate());
}

// The fixture's release kind carries no `requires`, as every profile
// written before the field existed does.
#[test]
fn a_release_kind_without_requirements_parses_as_requiring_nothing() {
    assert!(config().work_kinds[0].release_kinds[0].requires.is_empty());
}

// The fixture states no rhythm either — a profile from before the field
// must load, and the absence must read as "no rhythm" rather than a guess.
#[test]
fn a_profile_without_a_rhythm_parses_as_having_none() {
    assert!(config().rhythm.is_none());
}

// Nor an overview - and a document that never chose one must not be
// given one on the way out, or the shipped board could never change
// under a profile that is following it.
#[test]
fn a_profile_without_an_overview_reads_and_writes_as_having_none() {
    let config = config();
    assert!(config.overview.is_none());
    let written = serde_json::to_value(&config).unwrap();
    assert!(
        written.get("overview").is_none(),
        "an absent overview is left out, not written as null: {written}"
    );
}

#[test]
fn an_overview_round_trips_with_a_widget_this_build_does_not_know() {
    let mut document = serde_json::to_value(config()).unwrap();
    document["overview"] = json!({
        "layout": "mosaic",
        "widgets": [
            { "id": "score", "size": "l", "position": 0 },
            { "id": "from-a-later-build", "size": "s", "position": 1 },
        ],
    });
    let read: ProfileConfig = serde_json::from_value(document).unwrap();
    let overview = read.overview.as_ref().expect("the overview is read");
    assert_eq!(overview.layout, "mosaic");
    assert_eq!(overview.widgets.len(), 2, "the unknown widget is kept");
    assert!(read.validate().is_empty(), "{:?}", read.validate());

    let again: ProfileConfig =
        serde_json::from_value(serde_json::to_value(&read).unwrap()).unwrap();
    assert_eq!(
        again.overview, read.overview,
        "it is written back as it was read"
    );
}

#[test]
fn an_overview_that_only_names_a_layout_has_the_shipped_placement() {
    let overview: OverviewConfig = serde_json::from_value(json!({ "layout": "lead" })).unwrap();
    assert!(overview.widgets.is_empty());
}

/// A layout nobody draws is refused on save, by name - but read, so a
/// profile a later build wrote with a sixth layout still loads.
#[test]
fn an_unknown_layout_or_size_reads_but_is_refused_on_save() {
    let mut document = serde_json::to_value(config()).unwrap();
    document["overview"] = json!({
        "layout": "carousel",
        "widgets": [
            { "id": "score", "size": "xl", "position": 0 },
            { "id": "score", "size": "s", "position": 1 },
        ],
    });
    let read: ProfileConfig =
        serde_json::from_value(document).expect("an unknown layout still reads");
    let problems = read.validate();
    assert!(
        problems.iter().any(|p| p
            == "the overview's layout is `grid`, `lead`, `bands`, `mosaic` or `sheet`, not `carousel`"),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p == "overview widget 1 (`score`): the size is `s`, `m` or `l`, not `xl`"),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("overview widget 2 repeats the key `score`")),
        "{problems:?}"
    );
}

#[test]
fn every_layout_the_window_draws_is_accepted() {
    let mut config = config();
    for layout in OVERVIEW_LAYOUTS {
        config.overview = Some(OverviewConfig {
            layout: layout.into(),
            widgets: WIDGET_SIZES
                .iter()
                .enumerate()
                .map(|(position, size)| WidgetPlacement {
                    id: format!("w{position}"),
                    size: (*size).into(),
                    position: position as u32,
                })
                .collect(),
        });
        assert!(
            config.validate().is_empty(),
            "{layout}: {:?}",
            config.validate()
        );
    }
}

#[test]
fn a_monthly_goal_counts_a_tier_or_a_kind_the_profile_has() {
    // As a profile writes it: what is counted, and how many.
    let rhythm: Rhythm = serde_json::from_value(json!({
        "every_days": 2,
        "monthly": [{ "tier": "clip", "count": 4 }, { "kind": "short", "count": 8 }],
    }))
    .unwrap();
    assert_eq!(
        rhythm.monthly,
        vec![
            MonthlyGoal {
                of: GoalOf::Tier("clip".into()),
                count: 4
            },
            MonthlyGoal {
                of: GoalOf::Kind("short".into()),
                count: 8
            },
        ]
    );
    // And reads back as it was written.
    assert_eq!(
        serde_json::to_value(&rhythm.monthly).unwrap(),
        json!([{ "tier": "clip", "count": 4 }, { "kind": "short", "count": 8 }])
    );
    // Neither, and both, are not goals.
    assert!(serde_json::from_value::<MonthlyGoal>(json!({ "count": 4 })).is_err());

    let mut config = config();
    let tier = config.work_kinds[0].tiers[0].key.clone();
    let kind = config.work_kinds[0].key.clone();
    let goal = |of: GoalOf, count: u32| MonthlyGoal { of, count };
    config.rhythm = Some(Rhythm {
        every_days: 2,
        default_time: None,
        monthly: vec![
            goal(GoalOf::Tier(tier.clone()), 4),
            goal(GoalOf::Kind(kind.clone()), 8),
        ],
    });
    assert_eq!(config.validate(), Vec::<String>::new());

    config.rhythm = Some(Rhythm {
        every_days: 2,
        default_time: None,
        monthly: vec![
            goal(GoalOf::Tier("no-such-tier".into()), 4),
            goal(GoalOf::Kind("no-such-kind".into()), 8),
            goal(GoalOf::Kind(kind.clone()), 0),
            goal(GoalOf::Kind(kind), 3),
        ],
    });
    let problems = config.validate();
    for needle in [
        "counts the tier `no-such-tier`",
        "works of the kind `no-such-kind`",
        "at least one release",
        "count the same thing",
    ] {
        assert!(
            problems.iter().any(|problem| problem.contains(needle)),
            "expected `{needle}` in {problems:?}"
        );
    }
}

#[test]
fn a_rhythm_needs_no_default_time() {
    let rhythm: Rhythm = serde_json::from_value(json!({ "every_days": 3 })).unwrap();
    assert_eq!(rhythm.every_days, 3);
    assert!(rhythm.default_time.is_none());
}

#[test]
fn total_weighs_the_axes() {
    let values = json!({ "hook": 10.0, "text": 4.0 });
    // (1.0 * 2 + 0.4 * 1) / 3 = 0.8
    let total = config().total("song", values.as_object().unwrap());
    assert!((total - 80.0).abs() < 1e-9, "got {total}");
}

#[test]
fn a_missing_axis_does_not_count_as_zero() {
    let values = json!({ "hook": 8.0 });
    let total = config().total("song", values.as_object().unwrap());
    assert!((total - 80.0).abs() < 1e-9, "got {total}");
}

#[test]
fn an_empty_score_card_totals_zero_rather_than_dividing_by_zero() {
    let values = json!({});
    assert_eq!(config().total("song", values.as_object().unwrap()), 0.0);
}

#[test]
fn tier_picks_the_highest_threshold_reached() {
    let config = config();
    assert_eq!(config.tier_for("song", 80.0).unwrap().key, "clip");
    assert_eq!(config.tier_for("song", 74.9).unwrap().key, "hold");
}

fn typed() -> ProfileConfig {
    serde_json::from_value(json!({
        "work_kinds": [{ "key": "song", "label": "Song" }],
        "release_kinds": [
            { "key": "clip", "label": "Clip", "axis_weights": { "hook": 4.0, "chorus": 0.0 } },
            { "key": "audio", "label": "Audio" }
        ],
        "collection_kinds": [],
        "version_roles": [{ "key": "lyrics", "label": "Lyrics" }],
        "statuses": [{ "key": "draft", "label": "Draft" }],
        "axes": [
            { "key": "hook", "label": "Hook", "weight": 1.0, "scale": 10.0 },
            { "key": "chorus", "label": "Has a chorus", "weight": 1.0, "scale": 10.0, "kind": "flag" },
            { "key": "length", "label": "Length", "weight": 1.0, "scale": 10.0, "kind": "choice",
              "options": [
                  { "key": "short", "label": "Short", "value": 4.0 },
                  { "key": "right", "label": "Right", "value": 10.0 }
              ] }
        ],
        "tiers": [],
        "work_meta_fields": []
    }))
    .unwrap()
}

#[test]
fn an_axis_written_before_kinds_existed_is_a_scale() {
    assert_eq!(config().work_kinds[0].axes[0].kind, AxisKind::Scale);
    assert!(config().work_kinds[0].axes[0].options.is_empty());
}

#[test]
fn a_flag_is_worth_the_scale_or_nothing_and_a_choice_its_option() {
    let config = typed();
    let values = json!({ "hook": 5.0, "chorus": true, "length": "short" });
    // (0.5 + 1.0 + 0.4) / 3
    let total = config.total("song", values.as_object().unwrap());
    assert!((total - 63.333_333).abs() < 1e-3, "got {total}");

    let values = json!({ "hook": 5.0, "chorus": false, "length": "short" });
    let total = config.total("song", values.as_object().unwrap());
    assert!((total - 30.0).abs() < 1e-9, "got {total}");
}

#[test]
fn an_answer_the_axis_cannot_read_is_skipped_like_a_missing_one() {
    let config = typed();
    let values = json!({ "hook": 5.0, "length": "epic" });
    let total = config.total("song", values.as_object().unwrap());
    assert!(
        (total - 50.0).abs() < 1e-9,
        "an unknown option must not count: {total}"
    );
}

#[test]
fn a_number_still_reads_on_a_flag_or_a_choice() {
    // A snapshot taken while the axis was a scale keeps its value.
    let config = typed();
    let values = json!({ "chorus": 10.0, "length": 10.0 });
    let total = config.total("song", values.as_object().unwrap());
    assert!((total - 100.0).abs() < 1e-9, "got {total}");
}

#[test]
fn a_release_kind_reweighs_the_axes_it_names_and_keeps_the_rest() {
    let config = typed();
    let values = json!({ "hook": 10.0, "chorus": false, "length": "short" });
    let values = values.as_object().unwrap();
    // Plain: (1.0 + 0 + 0.4) / 3
    assert!((config.total("song", values) - 46.666_666).abs() < 1e-3);
    // As a clip: hook ×4, chorus ×0, length keeps 1: (4.0 + 0 + 0.4) / 5
    let clip = config.vocabulary("song").total_for(values, "clip");
    assert!((clip - 88.0).abs() < 1e-9, "got {clip}");
    // A kind that reweights nothing, or none at all, is the plain total.
    assert_eq!(
        config.vocabulary("song").total_for(values, "audio"),
        config.total("song", values)
    );
    assert_eq!(
        config.vocabulary("song").total_for(values, "no-such-kind"),
        config.total("song", values)
    );
}

#[test]
fn every_release_kind_gets_a_verdict_even_when_it_reweighs_nothing() {
    let config = typed();
    let values = json!({ "hook": 10.0, "chorus": false, "length": "short" });
    let verdicts = config.verdicts("song", values.as_object().unwrap());

    // One row per kind: a kind missing from the comparison would read as
    // "not applicable" rather than "weighs them the same way".
    assert_eq!(verdicts.len(), config.work_kinds[0].release_kinds.len());

    let clip = verdicts.iter().find(|v| v.kind == "clip").unwrap();
    let audio = verdicts.iter().find(|v| v.kind == "audio").unwrap();

    assert!(clip.reweighed, "clip names its own weights");
    assert!(!audio.reweighed, "audio names none");

    // The verdicts are the per-kind totals, not the flat one repeated.
    assert!((clip.total - 88.0).abs() < 1e-9, "got {}", clip.total);
    assert_eq!(
        audio.total,
        config.total("song", values.as_object().unwrap())
    );
    assert!(
        clip.total > audio.total,
        "the same answers read better as a clip: {} vs {}",
        clip.total,
        audio.total
    );
}

#[test]
fn a_verdict_carries_the_tier_its_own_total_reaches() {
    let config = typed();
    let values = json!({ "hook": 10.0, "chorus": false, "length": "short" });
    let verdicts = config.verdicts("song", values.as_object().unwrap());

    for verdict in &verdicts {
        // The claim is checkable: each tier is the one the profile itself
        // returns for that verdict's own total, not for the flat total.
        let expected = config
            .tier_for("song", verdict.total)
            .map(|t| t.key.clone());
        assert_eq!(verdict.tier, expected, "kind {}", verdict.kind);
    }
}

#[test]
fn a_sound_profile_has_no_problems() {
    assert_eq!(typed().validate(), Vec::<String>::new());
    assert_eq!(config().validate(), Vec::<String>::new());
}

#[test]
fn every_problem_names_its_place() {
    let mut config = typed();
    let first_axis = config.work_kinds[0].axes[0].clone();
    config.work_kinds[0].axes.push(first_axis);
    config.work_kinds[0].axes[1].scale = 0.0;
    config.work_kinds[0].axes[2].options.clear();
    config.work_kinds[0].tiers.push(Tier {
        key: "top".into(),
        label: "Top".into(),
        min: 140.0,
    });
    config.work_kinds[0].release_kinds[0]
        .requires
        .push("melody".into());
    config.work_kinds[0].release_kinds[0]
        .axis_weights
        .insert("ghost".into(), 1.0);
    config.work_kinds[0].version_roles.push(VersionRole {
        key: "review".into(),
        label: "Review".into(),
        comments_on: Some("prose".into()),
        body: None,
        counts_as_version: None,
        sung: false,
        skeleton: None,
    });
    config.rhythm = Some(Rhythm {
        every_days: 0,
        default_time: Some("noon".into()),
        monthly: Vec::new(),
    });

    let problems = config.validate();
    let expect = |needle: &str| {
        assert!(
            problems.iter().any(|p| p.contains(needle)),
            "no problem mentions `{needle}`:
{}",
            problems.join(
                "
"
            )
        );
    };
    expect("axis 4 repeats the key `hook`");
    expect("axis 2 (`chorus`) has the scale 0");
    expect("axis 3 (`length`) is a choice with nothing to choose from");
    expect("tier 1 (`top`) starts at 140");
    expect("release kind 1 (`clip`) requires `melody`");
    expect("release kind 1 (`clip`) weights `ghost`");
    expect("version role 2 (`review`) comments on `prose`");
    expect("the rhythm must be at least one day");
    expect("default time `noon`");
}

#[test]
fn a_problem_is_a_code_with_a_place_that_can_be_walked() {
    let mut config = typed();
    let key = config.work_kinds[0].key.clone();
    let axis = config.work_kinds[0].axes[1].key.clone();
    config.work_kinds[0].axes[1].scale = 0.0;

    let problems = config.problems();
    let found = problems
        .iter()
        .find(|problem| problem.key == "problem.axisScale")
        .expect("the broken axis is a problem of its own code");

    let at = found.params["place"].clone();
    assert_eq!(at["key"], "place.axis");
    assert_eq!(at["params"]["n"], 2);
    assert_eq!(at["params"]["key"], axis.as_str());
    let within = &at["params"]["in"];
    assert_eq!(within["key"], "place.workKind");
    assert_eq!(within["params"]["n"], 1);
    assert_eq!(within["params"]["key"], key.as_str());

    assert_eq!(
        found.english(),
        format!("work kind 1 (`{key}`): axis 2 (`{axis}`) has the scale 0; it must be above zero")
    );
}

#[test]
fn options_on_a_scale_are_refused_rather_than_ignored() {
    let mut config = typed();
    config.work_kinds[0].axes[0].options.push(AxisOption {
        key: "x".into(),
        label: "X".into(),
        value: 1.0,
    });
    let problems = config.validate();
    assert!(
        problems
            .iter()
            .any(|p| p.contains("lists options but is not a choice")),
        "{problems:?}"
    );
}

// ---- format 2: the vocabulary belongs to the kind -----------------------

fn flat_document() -> serde_json::Value {
    json!({
        "work_kinds": [{ "key": "song", "label": "Song" }, { "key": "instrumental", "label": "Instrumental" }],
        "release_kinds": [{ "key": "clip", "label": "Clip" }],
        "collection_kinds": [],
        "version_roles": [{ "key": "lyrics", "label": "Lyrics" }],
        "statuses": [{ "key": "draft", "label": "Draft", "derive": "draft" }],
        "axes": [{ "key": "hook", "label": "Hook", "weight": 1.0, "scale": 10.0 }],
        "tiers": [{ "key": "hold", "label": "Hold", "min": 0.0 }],
        "work_meta_fields": []
    })
}

#[test]
fn a_flat_document_hands_its_vocabulary_to_every_kind_that_declares_none() {
    let config: ProfileConfig = serde_json::from_value(flat_document()).unwrap();

    assert_eq!(config.format, FORMAT);
    for kind in &config.work_kinds {
        assert_eq!(kind.axes.len(), 1, "{}", kind.key);
        assert_eq!(kind.tiers.len(), 1, "{}", kind.key);
        assert_eq!(kind.version_roles.len(), 1, "{}", kind.key);
        assert_eq!(kind.release_kinds.len(), 1, "{}", kind.key);
        assert_eq!(kind.statuses.len(), 1, "{}", kind.key);
    }
}

#[test]
fn a_kind_that_declares_anything_keeps_only_what_it_declared() {
    let mut document = flat_document();
    document["work_kinds"].as_array_mut().unwrap().push(json!({
        "key": "video", "label": "Video",
        "statuses": [{ "key": "cut", "label": "Cut" }]
    }));
    let config: ProfileConfig = serde_json::from_value(document).unwrap();

    let video = config.kind("video").unwrap();
    assert!(
        video.axes.is_empty(),
        "a video must not inherit the song's axes"
    );
    assert!(video.tiers.is_empty());
    assert!(video.version_roles.is_empty());
    assert_eq!(video.statuses[0].key, "cut");
    assert_eq!(config.kind("song").unwrap().axes.len(), 1);
}

#[test]
fn the_written_document_has_no_flat_vocabulary_left() {
    let config: ProfileConfig = serde_json::from_value(flat_document()).unwrap();
    let written = serde_json::to_value(&config).unwrap();

    assert_eq!(written["format"], FORMAT);
    for flat in [
        "axes",
        "tiers",
        "version_roles",
        "release_kinds",
        "statuses",
    ] {
        assert!(
            written.get(flat).is_none(),
            "`{flat}` must live under the kinds now"
        );
    }
    assert_eq!(written["work_kinds"][0]["axes"][0]["key"], "hook");

    // And it reads back as the same thing: format 2 in, format 2 out.
    let again: ProfileConfig = serde_json::from_value(written).unwrap();
    assert_eq!(again.kind("song").unwrap().axes.len(), 1);
    assert_eq!(again.format, FORMAT);
}

#[test]
fn an_unknown_kind_reads_as_an_empty_vocabulary_rather_than_a_panic() {
    let config: ProfileConfig = serde_json::from_value(flat_document()).unwrap();
    let values = json!({ "hook": 9.0 });
    let values = values.as_object().unwrap();

    assert!(config.kind("poem").is_none());
    assert!(config.vocabulary("poem").axes.is_empty());
    assert_eq!(config.total("poem", values), 0.0);
    assert!(config.tier_for("poem", 50.0).is_none());
    assert!(config.verdicts("poem", values).is_empty());
    assert!(config.vocabulary("poem").starting_status().is_none());
}

#[test]
fn the_unions_name_each_key_once_with_the_first_label() {
    let mut document = flat_document();
    document["work_kinds"].as_array_mut().unwrap().push(json!({
        "key": "video", "label": "Video",
        "statuses": [{ "key": "draft", "label": "Rough cut" }, { "key": "posted", "label": "Posted" }],
        "tiers": [{ "key": "post", "label": "Post", "min": 70.0 }]
    }));
    let config: ProfileConfig = serde_json::from_value(document).unwrap();

    let statuses = config.all_statuses();
    assert_eq!(
        statuses.iter().map(|s| s.key.as_str()).collect::<Vec<_>>(),
        vec!["draft", "posted"]
    );
    assert_eq!(statuses[0].label, "Draft", "the first kind's label wins");
    assert_eq!(
        config
            .all_tiers()
            .iter()
            .map(|t| t.key.as_str())
            .collect::<Vec<_>>(),
        vec!["hold", "post"]
    );
}

#[test]
fn a_kind_without_statuses_is_a_problem_named_by_its_place() {
    let mut document = flat_document();
    document["work_kinds"].as_array_mut().unwrap().push(json!({
        "key": "video", "label": "Video",
        "axes": [{ "key": "cut", "label": "Cut", "weight": 1.0, "scale": 10.0 }]
    }));
    let config: ProfileConfig = serde_json::from_value(document).unwrap();
    let problems = config.validate();
    assert!(
        problems
            .iter()
            .any(|p| p.contains("work kind 3 (`video`) names no statuses")),
        "{problems:?}"
    );
}

#[test]
fn without_judgement_keeps_the_doors_and_drops_the_axes() {
    let config: ProfileConfig = serde_json::from_value(flat_document()).unwrap();
    let bare = config.kind("song").unwrap().without_judgement();
    assert!(bare.axes.is_empty() && bare.tiers.is_empty());
    assert_eq!(bare.statuses.len(), 1);
    assert_eq!(bare.version_roles.len(), 1);
    assert_eq!(bare.release_kinds.len(), 1);
}

fn studio() -> ProfileConfig {
    let conn = crate::db::open_in_memory().unwrap();
    crate::profile::seed(&conn, crate::profile::DEFAULT_KIND).unwrap();
    crate::profile::active(&conn).unwrap().unwrap().config
}

fn action(template: &str) -> crate::assistant::prompt::PromptTemplate {
    crate::assistant::prompt::PromptTemplate {
        show_in_ui: true,
        when: None,
        category: None,
        key: "x".into(),
        label: "X".into(),
        template: template.into(),
        description: None,
        icon: None,
        produces: None,
        method: None,
        kinds: Vec::new(),
        scope: None,
        web: false,
    }
}

/// The shipped Studio profile, with the audio release's YouTube door -
/// the audio kind's own since v0.86 - given the fields named here.
fn audio_fields(fields: Vec<ReleaseField>) -> ProfileConfig {
    let mut config = studio();
    for kind in &mut config.work_kinds {
        if kind.key != "audio" {
            continue;
        }
        for release_kind in &mut kind.release_kinds {
            if release_kind.key == "youtube" {
                release_kind.fields = fields.clone();
            }
        }
    }
    config
}

#[test]
fn the_shipped_profiles_say_what_a_release_goes_out_as() {
    let config = studio();
    let door_of = |kind: &str, door: &str| {
        config
            .vocabulary(kind)
            .release_kinds
            .iter()
            .find(|release_kind| release_kind.key == door)
            .unwrap_or_else(|| panic!("the studio profile ships `{door}` for a {kind}"))
            .clone()
    };
    let keys_of = |release_kind: &ReleaseKind| -> Vec<String> {
        release_kind
            .fields
            .iter()
            .map(|field| field.key.clone())
            .collect()
    };

    // A song goes out as what is made from it, never itself (v0.86).
    assert!(
        config.vocabulary("song").release_kinds.is_empty(),
        "a song has no door of its own"
    );
    // The audio release and the clip both go out on YouTube, each with a
    // comment pinned under it, and each in the shape of a video's preview.
    // The audio keeps two titles for later beside the one it premieres
    // under (v0.100.2); a clip's title is the song's name for good.
    let audio = door_of("audio", "youtube");
    let clip = door_of("video", "youtube");
    assert_eq!(
        keys_of(&audio),
        vec![
            "title",
            "alt_title_1",
            "alt_title_2",
            "description",
            "tags",
            "pinned"
        ]
    );
    assert_eq!(
        keys_of(&clip),
        vec!["title", "description", "tags", "pinned"]
    );
    for door in [&audio, &clip] {
        assert_eq!(door.cover_format.as_deref(), Some("16:9"));
        assert_eq!(
            door.fields[0].limit,
            Some(100),
            "a YouTube title is counted to the platform's own limit"
        );
    }
    for later in &audio.fields[1..3] {
        // The box names no tail of its own (v0.128): a title's suffix is the
        // owner's convention, written in the profile when they want one.
        assert_eq!(later.suffix, None);
        assert!(
            later.premiere.is_none(),
            "a title for later never premieres"
        );
        assert!(!later.is_required(), "a release goes out without one");
    }
    assert_eq!(
        door_of("short", "short").cover_format.as_deref(),
        Some("9:16")
    );
    assert_eq!(
        door_of("audio", "streaming").cover_format.as_deref(),
        Some("1:1")
    );
    assert!(
        clip.fields[0].template().is_some(),
        "a clip's title is filled from the work rather than typed every time"
    );
    assert_eq!(
        audio.fields[3].template(),
        Some("{donor:lyrics}"),
        "an audio release is described by the song's words, as the song's own door was"
    );
}

/// An action about a release is offered where there is one, reads
/// `{release}` only there, and answers with fields.
#[test]
fn an_action_about_a_release_is_held_to_releases() {
    let mut config = studio();
    let mut about = action("Write {release} for {title}, after {releases}");
    about.scope = Some("release".into());
    about.produces = Some("release".into());
    about.kinds = vec!["video".into(), "audio".into()];
    config.prompts = vec![about.clone()];
    assert!(config.validate().is_empty(), "{:?}", config.validate());

    let mut on_a_song = about.clone();
    on_a_song.kinds = vec!["song".into()];
    config.prompts = vec![on_a_song];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("is about a release, but `song` has no kind of release")),
        "{:?}",
        config.validate()
    );

    let mut loose = action("Write {release}");
    loose.produces = Some("release".into());
    config.prompts = vec![loose];
    let problems = config.validate().join("\n");
    assert!(
        problems.contains("which only an action about a release can"),
        "{problems}"
    );
    assert!(
        problems.contains("reads `{release}` but is not about a release"),
        "{problems}"
    );

    let mut prose = about;
    prose.produces = None;
    config.prompts = vec![prose];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("is about a release and must produce `release`"))
    );
}

/// An action about a cover is offered where there is a board - on a kind
/// with a cover - reads `{ideas}` and `{choices}` only there, and answers
/// with ideas. The shipped one is valid as it ships.
#[test]
fn an_action_about_a_cover_is_held_to_boards() {
    let mut config = studio();
    let shipped = config
        .prompts
        .iter()
        .find(|prompt| prompt.key == "cover-ideas")
        .expect("Studio ships the cover's ideas")
        .clone();
    assert_eq!(shipped.scope(), crate::assistant::prompt::Scope::Cover);
    assert!(config.validate().is_empty(), "{:?}", config.validate());

    let mut about = action("Ideas for {title}: {ideas} {choices}");
    about.scope = Some("cover".into());
    about.produces = Some("cover-ideas".into());
    about.kinds = vec!["song".into()];
    config.prompts = vec![about.clone()];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("is about a cover, but `song` has no cover")),
        "{:?}",
        config.validate()
    );

    let mut loose = action("Ideas for {title}: {ideas}");
    loose.produces = Some("cover-ideas".into());
    config.prompts = vec![loose];
    let problems = config.validate().join("\n");
    assert!(
        problems.contains("which only an action about a cover can"),
        "{problems}"
    );
    assert!(
        problems.contains("reads `{ideas}` but is not about a cover"),
        "{problems}"
    );

    let mut prose = about;
    prose.kinds = vec!["audio".into()];
    prose.produces = None;
    config.prompts = vec![prose];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("is about a cover and must produce `cover-ideas`"))
    );
}

/// A conception holds only where it can (ADR 0065): its brief a role of
/// the kind that is no version of the work, its idea a text field the
/// kind carries, its boards ones the profile names - and a board with
/// nothing to put on it is refused.
#[test]
fn a_conception_is_held_to_its_kind() {
    let config = studio();
    assert!(config.validate().is_empty(), "{:?}", config.validate());
    let song = config.conception("song").expect("Studio prepares a song");
    assert_eq!(song.brief, "brief");
    assert_eq!(song.idea.as_deref(), Some("premise"));

    let mut counting = config.clone();
    for kind in &mut counting.work_kinds {
        for role in &mut kind.version_roles {
            if role.key == "brief" {
                role.counts_as_version = Some(true);
            }
        }
    }
    let problems = counting.validate().join("\n");
    assert!(
        problems.contains("must not count as a version"),
        "{problems}"
    );

    let mut wrong = config.clone();
    let kind = wrong
        .work_kinds
        .iter_mut()
        .find(|kind| kind.key == "song")
        .unwrap();
    let conception = kind.conception.as_mut().unwrap();
    conception.idea = Some("bpm".into());
    conception.with = vec!["brief".into()];
    conception.boards.push("sources".into());
    let problems = wrong.validate().join("\n");
    assert!(
        problems.contains("refines its brief into `bpm`, which is not a text"),
        "{problems}"
    );
    assert!(
        problems.contains("sends `brief` with its order"),
        "{problems}"
    );
    assert!(
        problems.contains("the board `sources`, which the profile does not name"),
        "{problems}"
    );

    let mut empty = config;
    empty.boards[0].cards[0].fields.clear();
    let problems = empty.validate().join("\n");
    assert!(problems.contains("has no field"), "{problems}");
}

/// An action about a board lands its cards on a board the kinds it is
/// for lay out, reads the board, and `{cards}` is read there alone; the
/// conception's placeholders only where a kind is prepared on one.
#[test]
fn an_action_about_a_board_is_held_to_the_board() {
    let mut config = studio();
    let shipped = config
        .prompts
        .iter()
        .find(|prompt| prompt.key == "scenarios")
        .expect("Studio ships scenarios")
        .clone();
    assert_eq!(shipped.scope(), crate::assistant::prompt::Scope::Board);
    assert_eq!(
        shipped.produces(),
        crate::assistant::prompt::Produces::Cards("scenarios".into())
    );

    let mut about = action("Scenarios for {title}: {conception} {cards}");
    about.scope = Some("board".into());
    about.produces = Some("cards:scenarios".into());
    about.kinds = vec!["audio".into()];
    config.prompts = vec![about.clone()];
    let problems = config.validate().join("\n");
    assert!(
        problems.contains("`audio` lays out no such board"),
        "{problems}"
    );
    assert!(
        problems.contains("reads `{conception}`, but `audio` is not prepared"),
        "{problems}"
    );

    let mut loose = action("Ideas for {title}: {cards}");
    loose.produces = Some("cards:scenarios".into());
    loose.kinds = vec!["song".into()];
    config.prompts = vec![loose];
    let problems = config.validate().join("\n");
    assert!(
        problems.contains("which only an action about a board can"),
        "{problems}"
    );
    assert!(
        problems.contains("reads `{cards}` but is not about a board"),
        "{problems}"
    );

    let mut blind = about;
    blind.template = "Scenarios for {title}".into();
    blind.kinds = vec!["song".into()];
    blind.produces = Some("cards:nowhere".into());
    config.prompts = vec![blind];
    let problems = config.validate().join("\n");
    assert!(problems.contains("never reads `{cards}`"), "{problems}");
    assert!(problems.contains("names no board `nowhere`"), "{problems}");
}

/// How many ideas a "Make…" asks for: three when the profile says
/// nothing, never more than a run gives.
#[test]
fn the_ideas_on_make_default_to_three_and_stop_at_five() {
    let mut config = studio();
    assert_eq!(config.ideas_on_make(), 3);
    config.cover_ideas = Some(0);
    assert_eq!(config.ideas_on_make(), 0);
    assert!(config.validate().is_empty());
    config.cover_ideas = Some(9);
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("at most 5 ideas")),
        "{:?}",
        config.validate()
    );
}

/// A choice lists what can be chosen and starts at one of those; a field
/// of kinds names kinds the profile has.
#[test]
fn a_choice_field_offers_its_options_and_starts_at_one_of_them() {
    let mut config = studio();
    let variant = config
        .work_meta_fields
        .iter()
        .find(|field| field.key == "variant")
        .expect("the studio profile ships the audio's variant")
        .clone();
    assert_eq!(variant.field_type, MetaFieldType::Choice);
    assert!(variant.applies_to("audio") && !variant.applies_to("song"));
    assert_eq!(
        config.defaults_of("audio").get("variant"),
        Some(&serde_json::json!("original"))
    );
    assert!(config.defaults_of("song").is_empty());

    let mut broken = variant.clone();
    broken.default = Some(serde_json::json!("slowed-to-a-halt"));
    broken.kinds.push("opera".into());
    let mut empty = MetaField::new("mood-of-the-day", "Mood", MetaFieldType::Choice);
    empty.key = "empty".into();
    let mut stray = MetaField::new("tempo-word", "Tempo", MetaFieldType::Text);
    stray.options = variant.options.clone();
    config.work_meta_fields = vec![broken, empty, stray];
    let problems = config.validate().join("\n");
    assert!(
        problems.contains("which is not one of its options"),
        "{problems}"
    );
    assert!(problems.contains("names a work kind `opera`"), "{problems}");
    assert!(
        problems.contains("is a choice with nothing to choose from"),
        "{problems}"
    );
    assert!(
        problems.contains("lists options but is not a choice"),
        "{problems}"
    );
}

/// A text field may be numbered, by an example that ends in digits; a
/// field of another type, an example with nothing to count, and a default
/// beside the numbering are each named.
#[test]
fn a_numbered_field_is_text_counted_from_an_example() {
    let mut config = studio();
    let mut code = MetaField::new("code", "Code", MetaFieldType::Text);
    code.kinds = vec!["song".into()];
    code.numbered_from = Some("CAT-001".into());
    config.work_meta_fields.push(code.clone());
    assert!(config.validate().is_empty(), "{:?}", config.validate());
    assert_eq!(
        config
            .numbered_of("song")
            .iter()
            .map(|field| field.key.as_str())
            .collect::<Vec<_>>(),
        ["code"]
    );
    assert!(config.numbered_of("video").is_empty());

    let mut counted = MetaField::new("counted", "Counted", MetaFieldType::Number);
    counted.numbered_from = Some("1".into());
    let mut nothing = code.clone();
    nothing.key = "nothing".into();
    nothing.numbered_from = Some("CAT".into());
    let mut both = code.clone();
    both.key = "both".into();
    both.default = Some(serde_json::json!("CAT-000"));
    config.work_meta_fields = vec![counted, nothing, both];
    let problems = config.validate().join("\n");
    assert!(
        problems.contains("only a `text` field can be"),
        "{problems}"
    );
    assert!(problems.contains("does not end in digits"), "{problems}");
    assert!(problems.contains("keep one"), "{problems}");
}

/// The title of a work made from another reads the source's title and a
/// number, and a cover is a shape.
#[test]
fn a_made_title_and_a_cover_shape_are_checked() {
    let short = studio().kind("short").unwrap().clone();
    assert_eq!(short.title_made_from("Tide", "ru", 1), "Tide (short)");
    assert_eq!(
        short.title_made_from("Tide", "de", 3),
        "Tide (short 3)",
        "one name in every language, the number inside its bracket"
    );
    let clip = studio().kind("video").unwrap().clone();
    assert_eq!(clip.title_made_from("Tide", "en", 1), "Tide (video)");
    assert_eq!(
        clip.title_made_from("Tide", "en", 2),
        "Tide (video 2)",
        "a template without a number numbers the second one on"
    );
    let mut numbered = clip.clone();
    numbered.made_title = Some("{title} · take {n}".into());
    assert_eq!(numbered.title_made_from("Tide", "en", 1), "Tide · take 1");
    numbered.made_title = Some("{title} cut".into());
    assert_eq!(
        numbered.title_made_from("Tide", "en", 2),
        "Tide cut 2",
        "with no bracket to go in, the number follows"
    );

    let mut config = studio();
    for kind in &mut config.work_kinds {
        if kind.key == "video" {
            kind.made_title = Some("{name} ({n})".into());
            kind.release_kinds[0].cover_format = Some("wide".into());
        }
    }
    let problems = config.validate().join("\n");
    assert!(problems.contains("never reads `{title}`"), "{problems}");
    assert!(
        problems.contains("reads `{name}`; it reads `{title}` and `{n}`"),
        "{problems}"
    );
    assert!(
        problems.contains("gives its cover the shape `wide`"),
        "{problems}"
    );
    assert!(is_cover_format("16:9") && is_cover_format("1:1"));
    assert!(!is_cover_format("16:0") && !is_cover_format("16x9") && !is_cover_format(":9"));
}

/// A link pattern is a host with a dot, and the start of a path after it
/// when the host is shared: what a pasted address can be held against.
/// A scheme, a query or a host of one word is refused at save, by name.
#[test]
fn a_door_knows_its_links_by_a_host_and_a_path() {
    for pattern in [
        "youtu.be",
        "youtube.com/shorts/",
        "open.spotify.com/episode/",
    ] {
        assert!(is_link_pattern(pattern), "{pattern}");
    }
    for pattern in [
        "https://youtu.be",
        "youtube",
        ".youtube.com",
        "youtube.com/watch?v=",
        "you tube.com",
        "",
    ] {
        assert!(!is_link_pattern(pattern), "{pattern}");
    }

    let mut config = studio();
    for kind in &mut config.work_kinds {
        if kind.key == "video" {
            kind.release_kinds[0].links = vec!["https://youtu.be".into()];
        }
    }
    let problems = config.validate().join("\n");
    assert!(
        problems.contains("knows its links by `https://youtu.be`"),
        "{problems}"
    );
}

/// Every shipped door that says what a release of it carries says what
/// it cannot go without (v0.97): a door of fields none of which is needed
/// would be ready with every box empty.
#[test]
fn every_shipped_door_with_fields_needs_one_of_them() {
    for profile in crate::profile::builtin().unwrap() {
        for kind in &profile.config.work_kinds {
            for door in &kind.release_kinds {
                assert!(
                    door.fields.iter().any(ReleaseField::is_required) || door.fields.is_empty(),
                    "{}: door `{}` of `{}` says nothing it needs written",
                    profile.key,
                    door.key,
                    kind.key
                );
            }
        }
    }
}

/// Where a work of a kind opens is a word the document keeps as written:
/// absent from a document that never chose, and back as it went out.
#[test]
fn a_kind_keeps_the_tab_its_works_open_on() {
    let song = studio().kind("song").unwrap().clone();
    assert_eq!(
        song.open_on, None,
        "a kind that never chose opens the overview"
    );
    let written = serde_json::to_value(&song).unwrap();
    assert!(
        written.get("open_on").is_none(),
        "absent stays absent: {written}"
    );

    let mut chosen = song.clone();
    chosen.open_on = Some("versions".into());
    let written = serde_json::to_value(&chosen).unwrap();
    assert_eq!(written["open_on"], "versions");
    let read: WorkKind = serde_json::from_value(written).unwrap();
    assert_eq!(read.open_on.as_deref(), Some("versions"));

    // A tab a later build draws is kept, not refused: the window falls
    // back from a word it does not know.
    let mut config = studio();
    config.work_kinds[0].open_on = Some("a-tab-from-tomorrow".into());
    assert!(config.validate().is_empty(), "{:?}", config.validate());
}

#[test]
fn a_release_field_with_a_hole_in_it_is_refused_at_save() {
    let config = audio_fields(vec![
        ReleaseField::new("title", "Title", ReleaseFieldType::Line).from_template("{typo}"),
    ]);

    let problems = config.validate();

    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("reads `{typo}`, which nothing fills")),
        "{problems:?}"
    );
}

#[test]
fn a_release_field_reads_the_canon_only_as_a_public_text_may() {
    let config = audio_fields(vec![
        ReleaseField::new("title", "Title", ReleaseFieldType::Line).from_template("{title}"),
        ReleaseField::new("description", "Description", ReleaseFieldType::Text)
            .from_template("{canon}"),
        ReleaseField::new("tags", "Tags", ReleaseFieldType::Tags)
            .from_template("{canon:cover} {selection}"),
        ReleaseField::new("pinned", "Pinned", ReleaseFieldType::Text)
            .from_template("{canon:public}"),
    ]);

    let problems = config.validate().join("\n");

    assert!(
        problems.contains("reads `{canon}`, but a release goes out in public"),
        "{problems}"
    );
    assert!(
        problems.contains("reads `{canon:cover}`, but a release goes out in public"),
        "{problems}"
    );
    assert!(
        problems.contains("reads `{selection}`, which only an action"),
        "{problems}"
    );
    assert!(!problems.contains("`{canon:public}`, but"), "{problems}");
}

#[test]
fn a_release_field_is_held_to_the_roles_its_own_kind_has() {
    // `lyrics` is a role of the song, not of the audio release made from
    // it - which reads the song's words as `{donor:lyrics}`. An action
    // naming no kinds would be judged against every kind and could pass
    // on the strength of the song's roles; a release field cannot,
    // because it is read against exactly one kind.
    let config = audio_fields(vec![
        ReleaseField::new("description", "Description", ReleaseFieldType::Text)
            .from_template("{role:lyrics}"),
    ]);

    let problems = config.validate();

    assert!(
        problems
            .iter()
            .any(|problem| problem
                .contains("reads `{role:lyrics}`, but this kind has no `lyrics` role")),
        "{problems:?}"
    );
}

#[test]
fn a_release_field_reads_the_board_not_a_row_of_it() {
    // `{scene}` is one row, filled from the scene an action was started
    // on. A release is about the whole work and is started from no row at
    // all, so the placeholder would render empty forever.
    let config = audio_fields(vec![
        ReleaseField::new("description", "Description", ReleaseFieldType::Text)
            .from_template("{scene}"),
    ]);

    let problems = config.validate();

    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("it reads `{scenes}`")),
        "{problems:?}"
    );
}

#[test]
fn two_release_fields_cannot_share_a_key() {
    let config = audio_fields(vec![
        ReleaseField::new("title", "Title", ReleaseFieldType::Line),
        ReleaseField::new("title", "Headline", ReleaseFieldType::Line),
    ]);

    let problems = config.validate();

    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("repeats the key `title`")),
        "one key is one box: the second would overwrite the first in `release.meta`. {problems:?}"
    );
}

#[test]
fn a_release_field_needs_a_label_and_a_limit_worth_having() {
    let config = audio_fields(vec![ReleaseField {
        key: "title".into(),
        label: "  ".into(),
        field_type: ReleaseFieldType::Line,
        template: None,
        hint: None,
        limit: Some(0),
        suffix: None,
        premiere: None,
        required: None,
    }]);

    let problems = config.validate();

    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("has no label")),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("limited to no characters at all")),
        "{problems:?}"
    );
}

/// A premiere's tail comes off in any language it names and any year, so
/// a release moving into another year or out of its premiere loses the
/// old tail before it takes the new one; the plain tail comes off too,
/// and a title of nothing but a tail is not written.
#[test]
fn a_premiere_tail_is_known_in_every_year_and_every_language() {
    let mut field = ReleaseField::new("title", "Title", ReleaseFieldType::Line);
    field.suffix = Some(" (audio)".into());
    field.premiere = Some(Label::PerLocale(BTreeMap::from([
        ("en".to_owned(), " (Song premiere {year})".to_owned()),
        ("ru".to_owned(), " (Премьера песни {year})".to_owned()),
    ])));

    assert_eq!(
        field.premiere_tail(2026, "Tide").as_deref(),
        Some(" (Song premiere 2026)")
    );
    assert_eq!(
        field.premiere_tail(2027, "Прилив").as_deref(),
        Some(" (Премьера песни 2027)"),
        "the tail speaks the language of the title it ends"
    );
    assert_eq!(field.without_tail("Tide (Song premiere 2026)"), "Tide");
    assert_eq!(field.without_tail("Прилив (Премьера песни 2031)"), "Прилив");
    assert_eq!(field.without_tail("Tide (audio)"), "Tide");
    assert_eq!(
        field.without_tail("Tide (Song premiere)"),
        "Tide (Song premiere)",
        "no year is not the tail"
    );
    assert_eq!(field.without_tail("Tide 2026)"), "Tide 2026)");
    assert!(!field.is_filled(Some(&serde_json::json!(" (Премьера песни 2026)"))));
    assert!(field.is_filled(Some(&serde_json::json!("Tide (Song premiere 2026)"))));
}

#[test]
fn a_release_field_reading_a_role_its_kind_has_is_accepted() {
    let mut config = audio_fields(vec![
        ReleaseField::new("description", "Description", ReleaseFieldType::Text)
            .from_template("{title}\n\n{role:plot}"),
    ]);
    // An audio release names no role of its own since v0.117; the test
    // gives it the one its field reads.
    for kind in &mut config.work_kinds {
        if kind.key == "audio" {
            kind.version_roles.push(VersionRole::new("plot", "Concept"));
        }
    }

    assert!(config.validate().is_empty(), "{:?}", config.validate());
}

#[test]
fn an_action_with_a_hole_in_it_is_refused_at_save() {
    let mut config = studio();
    config.prompts = vec![action("Check {typo} of {title}")];
    let problems = config.validate();
    assert!(
        problems
            .iter()
            .any(|p| p.contains("reads `{typo}`, which nothing fills")),
        "{problems:?}"
    );

    // A role not every kind of the action has: the shipped Studio
    // kinds without lyrics are named, and naming the kinds fixes it.
    let mut config = studio();
    config.prompts = vec![action("{role:lyrics}")];
    let problems = config.validate();
    assert!(
        problems.iter().any(|p| p.contains(
            "reads `{role:lyrics}`, but `video`, `audio`, `short`, `experiment` have no `lyrics` role"
        )),
        "{problems:?}"
    );
    config.prompts[0].kinds = vec!["song".into()];
    assert!(config.validate().is_empty(), "{:?}", config.validate());

    let mut config = studio();
    let mut a = action("{role:plot}");
    a.kinds = vec!["video".into()];
    a.produces = Some("version:critique".into());
    config.prompts = vec![a];
    let problems = config.validate();
    assert!(
        problems
            .iter()
            .any(|p| p.contains("produces `version:critique`, but `video` has no `critique` role")),
        "{problems:?}"
    );

    let mut config = studio();
    let mut a = action("{scenes}");
    a.kinds = vec!["song".into()];
    config.prompts = vec![a];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("reads `{scenes}`, but `song` has no storyboard")),
        "{:?}",
        config.validate()
    );
}

#[test]
fn a_style_action_describes_its_brick_and_nothing_else_produces_a_description() {
    let mut config = studio();
    let mut stray = action("Describe it.");
    stray.produces = Some("description".into());
    config.prompts = vec![stray];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("only an action about a style can")),
        "{:?}",
        config.validate()
    );

    let mut chatty = action("Say something about it.");
    chatty.scope = Some("style".into());
    config.prompts = vec![chatty];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("is about a style and must produce `description`")),
        "{:?}",
        config.validate()
    );

    let mut kept = action("Describe it.");
    kept.scope = Some("style".into());
    kept.produces = Some("description".into());
    config.prompts = vec![kept];
    assert!(
        !config.validate().iter().any(|p| p.contains("style")),
        "{:?}",
        config.validate()
    );
}

#[test]
fn a_comment_action_is_about_a_comment_and_nothing_else_produces_one() {
    let mut config = studio();
    let mut stray = action("Write the reply.");
    stray.produces = Some("reply".into());
    config.prompts = vec![stray];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("only an action about a comment can")),
        "{:?}",
        config.validate()
    );

    let mut chatty = action("Say something about it.");
    chatty.scope = Some("comment".into());
    config.prompts = vec![chatty];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("must produce `comment` or `reply`")),
        "{:?}",
        config.validate()
    );

    let mut reader = action("Read it.");
    reader.scope = Some("comment".into());
    reader.produces = Some("comment".into());
    config.prompts = vec![reader];
    assert!(config.validate().is_empty(), "{:?}", config.validate());
}

#[test]
fn a_scene_action_is_held_to_its_shape() {
    let mut config = studio();
    let mut a = action("{scene}");
    a.kinds = vec!["video".into()];
    config.prompts = vec![a.clone()];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("give it `\"scope\": \"scene\"`")),
        "{:?}",
        config.validate()
    );

    a.scope = Some("scene".into());
    a.produces = Some("scenes".into());
    config.prompts = vec![a.clone()];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("must produce `scenes:revise`")),
        "{:?}",
        config.validate()
    );

    a.produces = Some("scenes:revise".into());
    config.prompts = vec![a];
    assert!(config.validate().is_empty(), "{:?}", config.validate());

    let mut config = studio();
    let mut a = action("{title}");
    a.scope = Some("scene".into());
    a.produces = Some("something-later".into());
    a.kinds = vec!["poem".into()];
    config.prompts = vec![a];
    let problems = config.validate();
    assert!(
        problems.iter().any(|p| p.contains("never reads `{scene}`")),
        "{problems:?}"
    );
    assert!(
        problems.iter().any(|p| p.contains("`produces` is `score`")),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("work kind `poem` the profile does not have")),
        "{problems:?}"
    );
}

#[test]
fn the_canon_of_a_profile_is_checked_at_save() {
    let mut config = studio();
    assert!(config.validate().is_empty(), "{:?}", config.validate());

    let mut second_root = NoteKind::new("brand", "Brand");
    second_root.root = true;
    second_root.sections = vec![CanonSection::new("identity", "General")];
    config.note_kinds.push(second_root);
    let mut crooked = NoteKind::new("prop", "Prop");
    crooked.sections = vec![{
        let mut relations = CanonSection::new("owners", "Owners");
        relations.shape = SectionShape::Relations;
        relations.kinds = vec!["note".into()];
        relations
    }];
    crooked.describe_from = vec!["looks".into()];
    config.note_kinds.push(crooked);

    let problems = config.validate().join("\n");
    assert!(problems.contains("one root"), "{problems}");
    assert!(
        problems.contains("`note`, which is not a kind of card"),
        "{problems}"
    );
    assert!(
        problems.contains("described from a section `looks`"),
        "{problems}"
    );
}

#[test]
fn an_action_about_a_card_or_a_selection_is_held_to_its_shape() {
    let mut config = studio();
    let action = |key: &str, scope: Option<&str>, produces: Option<&str>, template: &str| {
        let mut prompt = config.prompts[0].clone();
        prompt.key = key.into();
        prompt.scope = scope.map(str::to_owned);
        prompt.produces = produces.map(str::to_owned);
        prompt.template = template.into();
        prompt.kinds = Vec::new();
        prompt
    };
    let added = vec![
        action("card-scores", Some("canon"), Some("score"), "Judge it."),
        action(
            "loose-selection",
            Some("selection"),
            Some("canon"),
            "{title}",
        ),
        action("stray-selection", None, None, "{selection}"),
        action("stray-describe", None, Some("card-prompt"), "Describe."),
        action(
            "card-by-title",
            Some("canon"),
            Some("canon"),
            "About {title}.",
        ),
    ];
    config.prompts.extend(added);

    let problems = config.validate().join("\n");
    assert!(
        problems.contains("`card-scores`) is about a card"),
        "{problems}"
    );
    assert!(
        problems.contains("`loose-selection`) is about a selection but never reads"),
        "{problems}"
    );
    assert!(
        problems.contains("`stray-selection`) reads `{selection}` but is not about one"),
        "{problems}"
    );
    assert!(
        problems.contains("`stray-describe`) produces `card-prompt`"),
        "{problems}"
    );
    assert!(
        problems.contains("`card-by-title`) is about a card, which it is given whole"),
        "{problems}"
    );
}

/// The week is counted from the person's day, which only an action about
/// a work is given: an action about anything else reading `{week}` is
/// refused at save, and an action about a work is not (v0.106).
#[test]
fn only_an_action_about_a_work_reads_the_week() {
    let mut config = studio();
    let mut about_work = config.prompts[0].clone();
    about_work.key = "weekly".into();
    about_work.scope = None;
    about_work.produces = None;
    about_work.kinds = Vec::new();
    about_work.template =
        "{title} this week: {week} {score} {previous_version} {collection}".into();
    let mut about_cover = about_work.clone();
    about_cover.key = "cover-week".into();
    about_cover.scope = Some("cover".into());
    about_cover.template = "{ideas} {choices} {week}".into();
    config.prompts.extend([about_work, about_cover]);

    let problems = config.validate().join("\n");
    assert!(!problems.contains("`weekly`"), "{problems}");
    assert!(
        problems.contains("`cover-week`) reads `{week}`, which only an action about a work"),
        "{problems}"
    );
}

/// The shipped sound is one composition the document can hold: Studio's
/// style prompt picked from the ten types of sound, closing with the BPM
/// and the key.
#[test]
fn the_studio_writes_a_songs_style_out_of_its_sound() {
    let config = studio();
    assert!(config.validate().is_empty(), "{:?}", config.validate());
    let sound = config
        .composition_for("song", "style")
        .expect("a song's style is composed");
    assert_eq!(sound.key, "sound");
    assert!(
        sound.types().all(|key| config
            .style_type(key)
            .is_some_and(|t| t.form == StyleForm::Phrase)),
        "every block is a type of phrase"
    );
    assert_eq!(
        sound
            .fields
            .iter()
            .map(|f| f.field.as_str())
            .collect::<Vec<_>>(),
        ["bpm", "key"]
    );
    assert!(
        config.composition_for("video", "style").is_none(),
        "a clip has no style prompt"
    );
}

#[test]
fn a_composition_the_vocabulary_cannot_hold_is_refused_with_its_place() {
    let mut config = studio();
    let mut broken = config.compose[0].clone();
    broken.key = "broken".into();
    broken.kinds = vec!["video".into(), "nowhere".into()];
    broken.parts.push(CompositionPart {
        type_key: "image-style".into(),
        min: 3,
        max: Some(1),
        rule: None,
    });
    broken.fields.push(CompositionField {
        field: "bpm".into(),
        template: "fast".into(),
    });
    config.compose.push(broken);
    let mut orphan = StyleType::new("orphan", "Orphan");
    orphan.form = StyleForm::Phrase;
    config.style_types.push(orphan);

    let problems = config.validate().join("\n");
    for expected in [
        "composition 2 (`broken`) names a work kind `nowhere`",
        "writes the `style` role, which `video` does not have",
        "is picked from `image-style`, which is not a type of phrase",
        "`image-style` takes at least 3 and at most 1",
        "the template of `bpm` does not say where `{value}` goes",
        "style type `orphan` is a type of phrase no composition is picked from",
    ] {
        assert!(
            problems.contains(expected),
            "missing {expected:?} in:\n{problems}"
        );
    }
}

#[test]
fn two_compositions_of_one_role_and_kind_are_two_answers_and_refused() {
    let mut config = studio();
    let mut twin = config.compose[0].clone();
    twin.key = "twin".into();
    config.compose.push(twin);
    let problems = config.validate().join("\n");
    assert!(
        problems.contains("compositions `sound` and `twin` both write the `style` role of `song`"),
        "{problems}"
    );
}

/// A skill is a button with a message, or a playbook read by its method
/// (FR-17): a button needs its message, a playbook its method, and a hidden
/// button keeps its message.
#[test]
fn a_playbook_needs_its_method_and_a_button_its_message() {
    let mut config = studio();
    let mut playbook = action("");
    playbook.show_in_ui = false;
    playbook.method = Some("Work through the song in this order.".into());
    config.prompts = vec![playbook.clone()];
    assert!(config.validate().is_empty(), "{:?}", config.validate());

    // A hidden button keeps its message and needs nothing more.
    let mut hidden = action("Critique {title}");
    hidden.show_in_ui = false;
    hidden.method = Some("Read it slowly.".into());
    config.prompts = vec![hidden];
    assert!(config.validate().is_empty(), "{:?}", config.validate());

    let mut bare = playbook.clone();
    bare.method = None;
    config.prompts = vec![bare];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("is not a button and has no method")),
        "{:?}",
        config.validate()
    );

    config.prompts = vec![action("  ")];
    assert!(
        config
            .validate()
            .iter()
            .any(|p| p.contains("has no message")),
        "{:?}",
        config.validate()
    );

    let mut grouped = action("Critique {title}");
    grouped.category = Some("critique".into());
    config.prompts = vec![grouped.clone()];
    assert!(config.validate().is_empty(), "{:?}", config.validate());
    grouped.category = Some("misc".into());
    config.prompts = vec![grouped];
    assert!(
        config.validate().iter().any(|p| p.contains("“misc”")),
        "{:?}",
        config.validate()
    );
}

/// A button is written as it always was: an action stored before skills
/// reads the same, and keeps its fingerprint (ADR 0087).
#[test]
fn a_shown_action_is_written_as_before_and_a_hidden_one_says_so() {
    let shown = serde_json::to_value(action("Critique {title}")).unwrap();
    assert!(shown.get("show_in_ui").is_none(), "{shown}");
    let read: crate::assistant::prompt::PromptTemplate =
        serde_json::from_value(serde_json::json!({ "key": "x", "label": "X", "template": "t" }))
            .unwrap();
    assert!(read.show_in_ui);
    let mut hidden = action("");
    hidden.show_in_ui = false;
    let written = serde_json::to_value(&hidden).unwrap();
    assert_eq!(written["show_in_ui"], false);
    let read: crate::assistant::prompt::PromptTemplate = serde_json::from_value(
        serde_json::json!({ "key": "x", "label": "X", "show_in_ui": false, "method": "m" }),
    )
    .unwrap();
    assert_eq!(
        read.template, "",
        "a playbook may be stored without a message"
    );
}
