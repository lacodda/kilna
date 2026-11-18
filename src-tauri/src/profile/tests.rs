use super::upgrade::*;
use super::*;
use crate::db;
use crate::profile::config::{Derive, Label};

/// The whole upgrade of a copy met for the first time, as `carry_forward`
/// runs it: the entries kilna goes on shipping one by one, then the steps.
/// Names what was carried.
fn upgrade(config: &mut ProfileConfig, ships: &ProfileConfig) -> Vec<&'static str> {
    let mut ledger = super::shipped::Ledger::default();
    let seen = super::shipped::carry(config, ships, &mut ledger).unwrap();
    let mut carried: Vec<&'static str> = Vec::new();
    if seen.iter().any(|one| one.verdict.writes()) {
        carried.push("the entries kilna goes on shipping");
    }
    for step in UPGRADE {
        if (step.carry)(config, ships) {
            carried.push(step.name);
        }
    }
    carried
}

/// What a stored document says its format is: the top-level `format`
/// when it is a whole non-negative number, and 1 - the format that never
/// wrote one - for anything else, including a document that is not JSON.
#[test]
fn a_documents_format_is_its_top_level_number_or_one() {
    for (raw, format) in [
        (r#"{"format": 3, "work_kinds": []}"#, 3),
        (r#"{"work_kinds": [{"format": 3}]}"#, 1),
        (r#"{"work_kinds": [], "format": 2}"#, 2),
        (r#"{"format": 2, "format": 3}"#, 3),
        (r#"{"format": "3"}"#, 1),
        (r#"{"format": -3}"#, 1),
        (r#"{"format": 3.5}"#, 1),
        (r#"{"format": null}"#, 1),
        (r#"{}"#, 1),
        (r#"[3]"#, 1),
        (r#"3"#, 1),
        (r#"{"format": 3"#, 1),
        (r#"{"format": 3} trailing"#, 1),
        ("", 1),
    ] {
        assert_eq!(raw_format(raw), format, "{raw}");
    }
}

#[test]
fn every_builtin_profile_parses() {
    let profiles = builtin().unwrap();
    assert!(!profiles.is_empty());
    for profile in profiles {
        // The empty profile is judged on nothing until its owner says on
        // what: axes are the craft's own words (v0.112).
        if profile.key == BLANK_KIND {
            assert!(profile.config.work_kinds[0].axes.is_empty());
            continue;
        }
        assert!(
            !profile.config.work_kinds[0].axes.is_empty(),
            "{} has no axes",
            profile.key
        );
        assert!(
            !profile.config.work_kinds[0].tiers.is_empty(),
            "{} has no tiers",
            profile.key
        );
    }
}

/// A stored copy from before every part the upgrade carries: each list the
/// steps fill emptied, each field they set cleared, the kinds past the
/// first dropped.
fn stripped(shipped: &ProfileConfig) -> ProfileConfig {
    let mut old = shipped.clone();
    old.prompts.clear();
    old.rhythm = None;
    old.work_meta_fields.clear();
    old.note_kinds.clear();
    old.stages.clear();
    old.style_types.clear();
    old.boards.clear();
    for mark in &mut old.marks {
        mark.icon = None;
    }
    old.work_kinds.truncate(1);
    for kind in &mut old.work_kinds {
        kind.version_roles.truncate(1);
        for role in &mut kind.version_roles {
            role.body = None;
            role.counts_as_version = None;
        }
        kind.conception = None;
        kind.shot_types.clear();
        kind.scene_blocks.clear();
        kind.cover = false;
        for status in &mut kind.statuses {
            status.derive = Derive::Manual;
            status.colour = None;
        }
        for release_kind in &mut kind.release_kinds {
            release_kind.requires.clear();
            release_kind.icon = None;
            release_kind.fields.clear();
            release_kind.requires_cover = None;
            release_kind.links.clear();
        }
    }
    old
}

/// A Studio copy as the owner's stood before v0.99: the style prompt and
/// the experiment's brief never said whether they count as versions, the
/// premise is the one English word it shipped as, a kind opens on the
/// Notes tab, and nothing is prepared on a conception. The upgrade gives
/// the roles their word, the song and the experiment their conceptions
/// with the board of scenarios, the premise the word it ships with now,
/// and the tab its new name - and the copy still validates.
#[test]
fn an_older_copy_is_prepared_on_a_conception() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == "music")
        .unwrap();
    let mut config = shipped.config.clone();
    config.boards.clear();
    for kind in &mut config.work_kinds {
        kind.conception = None;
        kind.version_roles
            .retain(|role| kind.key == "experiment" || role.key != "brief");
        for role in &mut kind.version_roles {
            if role.key == "style" || role.key == "brief" {
                role.counts_as_version = None;
            }
        }
        if kind.key == "song" {
            kind.open_on = Some("notes".into());
        }
    }
    for field in &mut config.work_meta_fields {
        if field.key == "premise" {
            field.label = config::Label::One("Premise".into());
        }
    }

    upgrade(&mut config, &shipped.config);

    assert!(config.validate().is_empty(), "{:?}", config.validate());
    let song = config.vocabulary("song");
    let role = |key: &str| {
        song.version_roles
            .iter()
            .find(|role| role.key == key)
            .unwrap()
    };
    assert!(
        !role("style").counts_as_a_version(),
        "a style prompt is no draft"
    );
    assert!(!role("brief").counts_as_a_version());
    assert_eq!(
        song.conception.as_ref().unwrap().boards,
        ["research", "scenarios"]
    );
    assert_eq!(song.conception.as_ref().unwrap().with, ["style"]);
    assert_eq!(song.open_on.as_deref(), Some("conception"));
    let lab = config.vocabulary("experiment");
    assert_eq!(lab.conception.as_ref().unwrap().brief, "brief");
    assert!(config.board("scenarios").is_some());
    let premise = config
        .work_meta_fields
        .iter()
        .find(|field| field.key == "premise")
        .unwrap();
    assert_eq!(premise.label.in_locale("ru"), "Идея");
}

/// A copy as the owner's stood after v0.99: the song prepared on the
/// scenarios alone, no library, no research, the scenarios' template as it
/// shipped. The upgrade lays the research out before the scenarios,
/// brings the library with its media and the action that reads the web,
/// and the scenarios read what is starred in the research - and the copy
/// validates. A copy of Blog, whose `source` was a plain kind, finds it
/// the library.
#[test]
fn a_copy_of_v099_is_given_the_research_and_the_library() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == "music")
        .unwrap();
    let mut config = shipped.config.clone();
    config.boards.retain(|board| board.key != "research");
    config.note_kinds.retain(|kind| !kind.library);
    config.prompts.retain(|prompt| prompt.key != "research");
    for prompt in &mut config.prompts {
        if prompt.key == "scenarios" {
            prompt.template = TEMPLATES_BEFORE_THE_RESEARCH[0].1.to_owned();
        }
    }
    for kind in &mut config.work_kinds {
        if let Some(conception) = &mut kind.conception {
            conception.boards.retain(|board| board != "research");
        }
    }

    upgrade(&mut config, &shipped.config);

    assert!(config.validate().is_empty(), "{:?}", config.validate());
    assert_eq!(
        config.conception("song").unwrap().boards,
        ["research", "scenarios"]
    );
    let library = config.library().expect("the library arrived");
    assert_eq!(library.key, "source");
    assert!(library.medium("music").is_some_and(|music| !music.quoted));
    let research = config
        .prompts
        .iter()
        .find(|prompt| prompt.key == "research")
        .unwrap();
    assert!(research.web);
    let scenarios = config
        .prompts
        .iter()
        .find(|prompt| prompt.key == "scenarios")
        .unwrap();
    assert!(
        scenarios.template.contains("{starred:research}"),
        "{}",
        scenarios.template
    );

    let blog = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == "blog")
        .unwrap();
    let mut config = blog.config.clone();
    for kind in &mut config.note_kinds {
        if kind.key == "source" {
            *kind = config::NoteKind::new("source", "Source");
        }
    }
    config.boards.retain(|board| board.key != "research");
    upgrade(&mut config, &blog.config);
    assert!(config.validate().is_empty(), "{:?}", config.validate());
    let source = config.library().expect("Blog's sources are the library");
    assert_eq!(source.label.as_str(), "Source", "the owner's word stays");
    assert_eq!(
        source.media.len(),
        blog.config.library().unwrap().media.len()
    );
    assert!(config.board("research").is_some());
}

/// A brief role the owner said counts as a version is theirs to say: the
/// kind is left without a conception rather than given one its role
/// breaks, and the copy still validates.
#[test]
fn a_brief_the_owner_counts_keeps_its_kind_off_the_conception() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == "music")
        .unwrap();
    let mut config = shipped.config.clone();
    for kind in &mut config.work_kinds {
        kind.conception = None;
        for role in &mut kind.version_roles {
            if role.key == "brief" {
                role.counts_as_version = Some(true);
            }
        }
    }
    upgrade(&mut config, &shipped.config);
    assert!(config.conception("song").is_none());
    assert!(config.conception("experiment").is_none());
    assert!(
        config
            .validate()
            .iter()
            .all(|problem| !problem.contains("brief")),
        "{:?}",
        config.validate()
    );
}

/// A Studio copy from before v0.97: its doors ask for nothing beyond the
/// roles and know no links. The upgrade brings the shipped checklist in;
/// a requirement the owner took off - written `false` - stays off, and a
/// field the owner keeps optional is not made required behind their back.
#[test]
fn a_door_learns_what_it_needs_and_keeps_what_the_owner_took_off() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == "music")
        .unwrap();
    let mut config = shipped.config.clone();
    for kind in &mut config.work_kinds {
        for door in &mut kind.release_kinds {
            door.requires_cover = None;
            door.links.clear();
            for field in &mut door.fields {
                field.required = None;
            }
        }
    }
    // The owner's own word on two things the shipped door requires.
    {
        let youtube = config
            .work_kinds
            .iter_mut()
            .find(|kind| kind.key == "video")
            .and_then(|kind| kind.release_kinds.iter_mut().find(|d| d.key == "youtube"))
            .unwrap();
        youtube.requires_cover = Some(false);
        youtube.fields[1].required = Some(false);
    }

    assert!(going_out_needs(&mut config, &shipped.config));

    let video = config.vocabulary("video");
    let youtube = video
        .release_kinds
        .iter()
        .find(|d| d.key == "youtube")
        .unwrap();
    assert_eq!(youtube.requires_cover, Some(false), "taken off, stays off");
    assert!(youtube.fields[0].is_required(), "the title is required");
    assert_eq!(
        youtube.fields[1].required,
        Some(false),
        "the description the owner made optional stays optional"
    );
    assert!(youtube.links.iter().any(|link| link == "youtu.be"));
    let premiere = video
        .release_kinds
        .iter()
        .find(|d| d.key == "premiere")
        .unwrap();
    assert!(
        premiere.needs_cover(),
        "a door the owner said nothing about learns it"
    );
}

/// Every upgrade step is idempotent: run on its own result, it changes
/// nothing and says so. A step that reported a change every time would
/// rewrite every profile at every start; one that changed a second time
/// would be appending twice.
#[test]
fn every_upgrade_step_is_idempotent() {
    for profile in builtin().unwrap() {
        let mut config = stripped(&profile.config);
        for step in UPGRADE {
            (step.carry)(&mut config, &profile.config);
            let once = serde_json::to_value(&config).unwrap();
            assert!(
                !(step.carry)(&mut config, &profile.config),
                "{}: `{}` says it changed something the second time",
                profile.key,
                step.name
            );
            assert_eq!(
                serde_json::to_value(&config).unwrap(),
                once,
                "{}: `{}` changed the copy the second time",
                profile.key,
                step.name
            );
        }
    }
}

/// The steps together bring an old copy up to what ships, except for the
/// judgement of kinds it did not have - the axes and tiers the upgrade
/// deliberately leaves for the owner to write.
#[test]
fn the_upgrade_brings_an_old_copy_up_to_what_ships() {
    let profile = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.config.work_kinds.len() > 1)
        .expect("a shipped profile with more than one kind");
    let mut config = stripped(&profile.config);
    let carried = upgrade(&mut config, &profile.config);

    let mut expected = profile.config.clone();
    for kind in expected.work_kinds.iter_mut().skip(1) {
        *kind = kind.without_judgement();
    }
    assert_eq!(
        serde_json::to_value(&config).unwrap(),
        serde_json::to_value(&expected).unwrap(),
        "carried {carried:?}, and the copy still differs from what ships"
    );
}

/// A Studio copy as v0.89 stored it: titles reading the publication's
/// own name, no tail on the audio's title, a length carried into what is
/// made. A title the owner reworded stays theirs.
#[test]
fn a_title_comes_to_read_the_origin_and_keep_its_tail() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == "music")
        .unwrap()
        .config;
    let mut old = shipped.clone();
    for kind in &mut old.work_kinds {
        for door in &mut kind.release_kinds {
            for field in &mut door.fields {
                if field.template.as_deref() == Some("{origin}") {
                    field.template = Some("{title}".into());
                }
                field.suffix = None;
            }
        }
    }
    for field in &mut old.work_meta_fields {
        field.own = false;
    }
    let reworded = "{title} #shorts";
    old.work_kinds
        .iter_mut()
        .find(|kind| kind.key == "short")
        .unwrap()
        .release_kinds[0]
        .fields[0]
        .template = Some(reworded.into());

    assert!(going_out_words(&mut old, &shipped));

    let title = |kind: &str, door: &str| {
        old.kind(kind)
            .unwrap()
            .release_kinds
            .iter()
            .find(|candidate| candidate.key == door)
            .unwrap()
            .fields
            .iter()
            .find(|field| field.key == "title")
            .unwrap()
            .clone()
    };
    assert_eq!(
        title("video", "youtube").template.as_deref(),
        Some("{origin}")
    );
    assert_eq!(
        title("audio", "youtube").suffix.as_deref(),
        Some(" (audio)")
    );
    assert_eq!(title("audio", "streaming").suffix, None);
    assert_eq!(title("short", "short").template.as_deref(), Some(reworded));
    assert!(
        old.work_meta_fields
            .iter()
            .any(|field| field.key == "duration" && field.own)
    );
}

/// A Studio copy as v0.87 stored it: the image style filed under its
/// first five families, one the owner relabelled, and one of their own
/// put first. The families shipped since arrive, each after the one it
/// follows in the shipped list; the owner's word and family stay.
#[test]
fn a_family_shipped_later_reaches_a_type_that_already_files() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == "music")
        .unwrap()
        .config;
    let shipped_families = shipped.style_type("image-style").unwrap().families.clone();
    assert!(
        shipped_families.len() > 5,
        "the set files image styles under more than the first five families"
    );

    let mut old = shipped.clone();
    let image = old
        .style_types
        .iter_mut()
        .find(|style| style.key == "image-style")
        .unwrap();
    image.families.truncate(5);
    image.families[1].label = Label::from("Old masters");
    image.families.insert(
        0,
        config::StyleFamily {
            key: "mine".into(),
            label: Label::from("Mine"),
        },
    );

    assert!(style_types(&mut old, &shipped));

    let families = &old.style_type("image-style").unwrap().families;
    let keys: Vec<&str> = families.iter().map(|f| f.key.as_str()).collect();
    let mut expected = vec!["mine"];
    expected.extend(shipped_families.iter().map(|f| f.key.as_str()));
    assert_eq!(keys, expected);
    assert_eq!(families[2].label, Label::from("Old masters"));
    assert!(!style_types(&mut old, &shipped), "once is enough");
}

#[test]
fn upgrade_steps_have_names_of_their_own() {
    let names: std::collections::BTreeSet<&str> = UPGRADE.iter().map(|step| step.name).collect();
    assert_eq!(names.len(), UPGRADE.len(), "two steps share a name");
}

#[test]
fn builtin_keys_are_unique() {
    let profiles = builtin().unwrap();
    let mut keys: Vec<_> = profiles.iter().map(|p| p.key.as_str()).collect();
    keys.sort_unstable();
    let count = keys.len();
    keys.dedup();
    assert_eq!(keys.len(), count, "two built-in profiles share a key");
}

/// A space is of one kind: its database holds that profile, active, and
/// none of the crafts it is not (ADR 0077).
#[test]
fn a_space_is_seeded_with_its_own_kind_alone() {
    for shipped in builtin().unwrap() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn, &shipped.key).unwrap();

        let profiles = list(&conn).unwrap();
        assert_eq!(
            profiles.iter().map(|p| p.key.as_str()).collect::<Vec<_>>(),
            [shipped.key.as_str()]
        );
        assert!(profiles[0].is_active && profiles[0].is_builtin);
    }
}

#[test]
fn a_kind_kilna_does_not_ship_is_refused_and_nothing_seeded() {
    let conn = db::open_in_memory().unwrap();

    let refused = seed(&conn, "poetry").unwrap_err();

    assert_eq!(refused.refusal().unwrap().code, "space.unknownKind");
    assert!(list(&conn).unwrap().is_empty());
}

#[test]
fn studio_is_what_a_space_is_made_of_by_default() {
    assert_eq!(builtin().unwrap()[0].key, DEFAULT_KIND);
}

#[test]
fn seed_does_not_duplicate_on_a_second_run() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    assert_eq!(list(&conn).unwrap().len(), 1);
}

/// A workspace from before spaces holds every craft; seeding it adds none
/// and carries forward each it has - the move into spaces is what takes
/// the empty ones out (`space::legacy`).
#[test]
fn a_database_that_holds_profiles_gains_none() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, "blog").unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let keys: Vec<String> = list(&conn).unwrap().into_iter().map(|p| p.key).collect();
    assert_eq!(keys, ["blog"]);
    assert_eq!(active(&conn).unwrap().unwrap().key, "blog");
}

/// A workspace created before a field existed keeps the profile it copied
/// on first run, and shipping the field changes nothing there on its own.
/// This is not hypothetical: the status automation was found inert on a
/// real database for exactly this reason, deriving `Manual` for every
/// status while the shipped profile named meanings for four of them.
#[test]
fn an_older_workspace_gains_the_meanings_its_statuses_were_missing() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    // Wind the stored copy back to what a pre-`derive` workspace holds: the
    // same words, with no meaning attached to any of them.
    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    for status in &mut config.work_kinds[0].statuses {
        status.derive = Derive::Manual;
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let derive_of = |key: &str| {
        config.work_kinds[0]
            .statuses
            .iter()
            .find(|status| status.key == key)
            .unwrap()
            .derive
    };
    assert_eq!(derive_of("draft"), Derive::Draft);
    assert_eq!(derive_of("scored"), Derive::Scored);
    assert_eq!(derive_of("scheduled"), Derive::Scheduled);
    assert_eq!(derive_of("released"), Derive::Released);
    // Nothing is invented for a status the shipped profile leaves manual.
    assert_eq!(derive_of("shelved"), Derive::Manual);
}

/// Same delivery path as `derive`: a workspace copied before release kinds
/// stated their requirements gains them on the next start, or readiness
/// marks stay blank there forever.
#[test]
fn an_older_workspace_gains_the_roles_its_release_kinds_require() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    // Wind the stored copy back: the same doors on the video, requiring
    // nothing — and one the user narrowed by hand, which must stay
    // narrowed.
    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    let video = config
        .work_kinds
        .iter_mut()
        .find(|kind| kind.key == "video")
        .unwrap();
    for kind in &mut video.release_kinds {
        kind.requires = if kind.key == "premiere" {
            vec!["context".into()]
        } else {
            Vec::new()
        };
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let requires_of = |key: &str| {
        config
            .vocabulary("video")
            .release_kinds
            .iter()
            .find(|kind| kind.key == key)
            .unwrap()
            .requires
            .clone()
    };
    assert_eq!(requires_of("youtube"), vec!["plot"]);
    // A list the user set themselves is not overwritten by the upgrade.
    assert_eq!(requires_of("premiere"), vec!["context"]);
}

/// Same delivery path again, for the glyph the calendar draws a kind with.
/// A workspace made before the field existed shows every kind under the
/// same fallback mark until this backfill runs.
#[test]
fn an_older_workspace_gains_the_icons_its_release_kinds_are_drawn_with() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    // Wind the stored copy back to before the field: no icons at all,
    // except one the user chose by hand, which must survive the upgrade.
    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    for kind in &mut config.work_kinds {
        for door in &mut kind.release_kinds {
            door.icon = if door.key == "premiere" {
                Some("radio".into())
            } else {
                None
            };
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let icon_of = |kind: &str, key: &str| {
        config
            .vocabulary(kind)
            .release_kinds
            .iter()
            .find(|door| door.key == key)
            .unwrap()
            .icon
            .clone()
    };
    assert_eq!(icon_of("audio", "youtube").as_deref(), Some("disc"));
    assert_eq!(icon_of("video", "youtube").as_deref(), Some("film"));
    assert_eq!(icon_of("short", "short").as_deref(), Some("smartphone"));
    // A glyph the user picked themselves is not taken back by the upgrade.
    assert_eq!(icon_of("video", "premiere").as_deref(), Some("radio"));
}

/// A workspace from before v0.86 gains the audio kind where the shipped
/// list puts it, the shape of every door's cover, the titles of what is
/// made from a song, and the variant as a choice for audio alone - and a
/// word the owner wrote is not taken back.
#[test]
fn an_older_workspace_gains_what_a_publication_is_made_of() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    config.work_kinds.retain(|kind| kind.key != "audio");
    for kind in &mut config.work_kinds {
        kind.made_title = None;
        for door in &mut kind.release_kinds {
            door.cover_format = None;
        }
        if kind.key == "short" {
            kind.made_title = Some("{title} (cut {n})".into());
        }
    }
    for field in &mut config.work_meta_fields {
        if field.key == "variant" {
            field.kinds.clear();
            field.options.clear();
            field.default = None;
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let order: Vec<&str> = config
        .work_kinds
        .iter()
        .map(|kind| kind.key.as_str())
        .collect();
    assert_eq!(order, vec!["song", "video", "audio", "short", "experiment"]);
    assert!(config.vocabulary("audio").frame);
    assert_eq!(
        config.vocabulary("video").release_kinds[0]
            .cover_format
            .as_deref(),
        Some("16:9")
    );
    assert_eq!(
        config.vocabulary("video").title_made_from("Tide", "en", 1),
        "Tide (video)"
    );
    assert_eq!(
        config.vocabulary("short").title_made_from("Tide", "en", 2),
        "Tide (cut 2)",
        "a title the owner wrote stays theirs"
    );
    let variant = config
        .work_meta_fields
        .iter()
        .find(|field| field.key == "variant")
        .unwrap();
    assert_eq!(variant.kinds, vec!["audio".to_owned()]);
    assert_eq!(variant.options.len(), 5);
    assert_eq!(variant.default, Some(serde_json::json!("original")));
}

#[test]
fn an_older_workspace_gains_the_way_its_roles_read() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    // Before the field: no role says how it reads, except one the owner
    // set by hand, which the upgrade must leave alone.
    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    for role in &mut config.work_kinds[0].version_roles {
        role.body = if role.key == "critique" {
            Some("plain".into())
        } else {
            None
        };
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let body_of = |key: &str| {
        config.work_kinds[0]
            .version_roles
            .iter()
            .find(|role| role.key == key)
            .unwrap()
            .body
            .clone()
    };
    assert_eq!(body_of("lyrics").as_deref(), Some("plain"));
    assert_eq!(body_of("review").as_deref(), Some("markdown"));
    // The owner's own choice is not taken back by the upgrade.
    assert_eq!(body_of("critique").as_deref(), Some("plain"));
}

/// A kind the owner invented is theirs alone: the shipped profile has
/// nothing to say about it, and the backfill must not reach for a glyph
/// belonging to some other kind that happens to sit at the same index.
#[test]
fn a_kind_of_the_owners_own_gains_no_icon() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    let video = config
        .work_kinds
        .iter()
        .position(|kind| kind.key == "video")
        .unwrap();
    config.work_kinds[video].release_kinds.insert(
        0,
        crate::profile::config::ReleaseKind::new("vinyl", "Vinyl pressing", &[]),
    );
    // The shipped door beside it loses its glyph, so the backfill has
    // something to do that the assertion below can tell from nothing.
    for door in &mut config.work_kinds[video].release_kinds {
        if door.key == "youtube" {
            door.icon = None;
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let vinyl = config
        .vocabulary("video")
        .release_kinds
        .iter()
        .find(|kind| kind.key == "vinyl")
        .unwrap();
    assert_eq!(vinyl.icon, None);
    // And the kinds that do ship still got theirs, so the assertion above
    // is not passing because the backfill did nothing at all.
    assert!(
        config
            .vocabulary("video")
            .release_kinds
            .iter()
            .any(|kind| kind.key == "youtube" && kind.icon.as_deref() == Some("film"))
    );
}

/// The labels are the user's, and an upgrade must not take them back.
#[test]
fn carrying_meanings_forward_leaves_renamed_statuses_renamed() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    for status in &mut config.work_kinds[0].statuses {
        status.derive = Derive::Manual;
        if status.key == "released" {
            status.label = "Out in the world".into();
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let released = config.work_kinds[0]
        .statuses
        .iter()
        .find(|status| status.key == "released")
        .unwrap();
    assert_eq!(released.label, "Out in the world");
    assert_eq!(released.derive, Derive::Released);
}

/// Every profile must be self-consistent, because a broken one is only
/// discovered when a user switches to it.
#[test]
fn every_builtin_profile_is_internally_consistent() {
    for profile in builtin().unwrap() {
        let config = &profile.config;
        let key = &profile.key;

        assert!(!config.work_kinds.is_empty(), "{key} has no work kinds");
        // Every kind carries its own vocabulary (format 2), and each has
        // to be sound on its own: a video is judged and shipped without
        // ever consulting the song's lists.
        for vocab in &config.work_kinds {
            assert!(
                !vocab.statuses.is_empty(),
                "{key}/{}: no statuses",
                vocab.key
            );

            // A profile whose statuses carry no meaning derives nothing, and
            // the automation would sit there doing nothing with no error to
            // show for it. Every derivable meaning needs a word, and no two
            // words may claim the same one — the automation would pick between
            // them by list order, which is not a decision anybody made. An
            // experiment is the exception that proves it: it is never
            // judged on axes, booked or released (ADR 0061), so of the four
            // only a draft can happen to it, and a word for the others
            // would be a word nothing ever says.
            let meanings: &[Derive] = if vocab.lab.is_some() {
                &[Derive::Draft]
            } else {
                &[
                    Derive::Draft,
                    Derive::Scored,
                    Derive::Scheduled,
                    Derive::Released,
                ]
            };
            for meaning in [
                Derive::Draft,
                Derive::Scored,
                Derive::Scheduled,
                Derive::Released,
            ] {
                let named: Vec<&str> = vocab
                    .statuses
                    .iter()
                    .filter(|status| status.derive == meaning)
                    .map(|status| status.key.as_str())
                    .collect();
                let wanted = usize::from(meanings.contains(&meaning));
                assert_eq!(
                    named.len(),
                    wanted,
                    "{key}/{} names {} status(es) for {meaning:?}: {named:?}",
                    vocab.key,
                    named.len()
                );
            }
            // A kind with a text of its own names the roles it is written
            // in. One that plays under a single picture - an audio release -
            // is its song, whole, and has none (v0.117).
            assert!(
                !vocab.version_roles.is_empty() || vocab.frame,
                "{key}/{}: no version roles",
                vocab.key
            );

            // A requirement naming a role the profile does not have can never
            // be satisfied, so every release of that kind reads as unready
            // forever — with nothing on any screen to explain why.
            for kind in &vocab.release_kinds {
                for role in &kind.requires {
                    assert!(
                        vocab.version_roles.iter().any(|known| &known.key == role),
                        "{key}: release kind `{}` requires role `{role}`, which the profile does not define",
                        kind.key
                    );
                }
            }

            // A kind nobody judges - an audio release is judged as the
            // song it plays - has no axes and so no tiers; the rest of
            // the checks below are about judgement.
            if vocab.axes.is_empty() {
                assert!(
                    vocab.tiers.is_empty(),
                    "{key}/{}: tiers no score can land in",
                    vocab.key
                );
                continue;
            }

            // A tier reachable by nothing is a tier that never appears.
            assert!(
                vocab.tiers.iter().any(|tier| tier.min == 0.0),
                "{key} has no tier a zero score falls into"
            );
            assert!(
                vocab.tiers.iter().all(|tier| tier.min <= 100.0),
                "{key} has a tier no score can reach"
            );

            // Full marks on every axis must land in the top tier, or the
            // scoring scale and the tiers disagree with each other.
            let full: serde_json::Map<String, serde_json::Value> = vocab
                .axes
                .iter()
                .map(|axis| (axis.key.clone(), serde_json::json!(axis.scale)))
                .collect();
            let total = vocab.total(&full);
            assert!(
                (total - 100.0).abs() < 1e-9,
                "{key}: full marks scored {total}, not 100"
            );
        }

        // A rhythm every built-in states, and states sanely: zero days
        // between releases is not a pace, and the usual time is HH:MM or
        // absent. User-made profiles are free to omit the rhythm — the
        // layout button then explains itself — but a built-in shipping
        // without one would make the feature invisible by default.
        let rhythm = config
            .rhythm
            .as_ref()
            .unwrap_or_else(|| panic!("{key} ships without a rhythm"));
        assert!(rhythm.every_days >= 1, "{key} has a rhythm of zero days");
        if let Some(time) = &rhythm.default_time {
            let split = time.split_once(':');
            let sane = split.is_some_and(|(hours, minutes)| {
                hours.parse::<u8>().is_ok_and(|h| h < 24)
                    && minutes.parse::<u8>().is_ok_and(|m| m < 60)
                    && hours.len() == 2
                    && minutes.len() == 2
            });
            assert!(sane, "{key} has a default time of `{time}`, not HH:MM");
        }

        // Prompts may only ask for roles every kind they are for has;
        // `validate` refuses otherwise, and a shipped file must pass it.
        for prompt in &config.prompts {
            let kinds: Vec<_> = config
                .work_kinds
                .iter()
                .filter(|kind| prompt.applies_to(&kind.key))
                .collect();
            assert!(
                !kinds.is_empty(),
                "{key}: prompt `{}` is for no kind",
                prompt.key
            );
            for fragment in prompt.template.split("{role:").skip(1) {
                let role = fragment.split('}').next().unwrap_or_default();
                for kind in &kinds {
                    assert!(
                        kind.version_roles.iter().any(|r| r.key == role),
                        "{key}: prompt `{}` asks `{}` for unknown role `{role}`",
                        prompt.key,
                        kind.key
                    );
                }
            }
        }
    }
}

/// Same delivery path as `derive` and `requires`: a workspace copied
/// before profiles knew their pace gains the shipped one on the next
/// start — and one the user set themselves is left alone.
#[test]
fn an_older_workspace_gains_the_rhythm_its_profile_was_missing() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    config.rhythm = None;
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let shipped = builtin().unwrap()[0].config.rhythm.clone().unwrap();
    let gained = config_for(&conn, &id).unwrap().rhythm.unwrap();
    assert_eq!(gained.every_days, shipped.every_days);

    // A pace the user chose is not overwritten by the upgrade.
    let mut config = config_for(&conn, &id).unwrap();
    config.rhythm = Some(config::Rhythm {
        every_days: 30,
        default_time: None,
        monthly: Vec::new(),
    });
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    assert_eq!(
        config_for(&conn, &id).unwrap().rhythm.unwrap().every_days,
        30
    );
}

#[test]
fn seed_carries_newly_shipped_version_roles_into_an_existing_profile() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let (id, _): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    // A workspace from before the review roles shipped, with the user's own
    // rename on the one role it did have.
    let mut config = config_for(&conn, &id).unwrap();
    config.work_kinds[0]
        .version_roles
        .retain(|role| role.key == "lyrics");
    config.work_kinds[0].version_roles[0].label = "Words".into();
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let roles = config_for(&conn, &id).unwrap().work_kinds[0]
        .version_roles
        .clone();
    let keys: Vec<&str> = roles.iter().map(|role| role.key.as_str()).collect();
    assert!(
        keys.contains(&"review"),
        "a newly shipped role arrives: {keys:?}"
    );
    assert_eq!(keys.first(), Some(&"lyrics"), "the kept role stays first");
    assert_eq!(
        roles[0].label, "Words",
        "and keeps the user's own name for it"
    );

    // The pairing arrives with the role: without it the review would be one
    // more list to page through rather than something read beside the text.
    assert_eq!(
        roles
            .iter()
            .find(|role| role.key == "review")
            .and_then(|role| role.comments_on.as_deref()),
        Some("lyrics"),
    );

    let mut sorted = keys.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), keys.len(), "no role appears twice: {keys:?}");
}

/// A skeleton (v0.91) reaches a workspace made before it shipped, and a
/// skeleton the owner wrote for a role is theirs.
#[test]
fn seed_carries_a_skeleton_where_the_role_has_none() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let id: String = conn
        .query_row("SELECT id FROM profile WHERE key = 'music'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut config = config_for(&conn, &id).unwrap();
    let shipped = config.vocabulary("song").version_roles[0].skeleton.clone();
    assert!(shipped.is_some(), "the lyric ships with its parts");
    for role in &mut config.work_kinds[0].version_roles {
        role.skeleton = (role.key == "style").then(|| "[Mine]".into());
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let roles = config_for(&conn, &id).unwrap().work_kinds[0]
        .version_roles
        .clone();
    let skeleton_of = |key: &str| {
        roles
            .iter()
            .find(|role| role.key == key)
            .and_then(|role| role.skeleton.clone())
    };
    assert_eq!(skeleton_of("lyrics"), shipped);
    assert_eq!(skeleton_of("style"), Some("[Mine]".into()));
}

#[test]
fn seed_carries_newly_shipped_meta_fields_into_an_existing_profile() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    // A workspace created before the descriptive fields shipped: it holds
    // the reference numbers and nothing else.
    let (id, _): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config = config_for(&conn, &id).unwrap();
    config.work_meta_fields.retain(|field| field.key == "bpm");
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let gained = config_for(&conn, &id).unwrap().work_meta_fields;
    let keys: Vec<&str> = gained.iter().map(|field| field.key.as_str()).collect();
    assert!(
        keys.contains(&"premise"),
        "a newly shipped field arrives: {keys:?}"
    );
    assert_eq!(
        keys.first(),
        Some(&"bpm"),
        "the kept field stays where it was"
    );
    assert_eq!(
        gained
            .iter()
            .find(|field| field.key == "premise")
            .map(|field| field.field_type),
        Some(config::MetaFieldType::Multiline),
        "and arrives with its own type, not as plain text"
    );
}

/// A workspace made before v0.98 holds the idea kind; the two shelves
/// retired it (ADR 0064), so the upgrade takes it out of the stored copy
/// and puts the rest in the shipped order - phrase, then note. Caught on
/// the owner's workspace in the live run of v0.98: a copy grown kind by
/// kind held note, idea, phrase, and its Notes screen opened on the
/// notes while a new one opened on the phrases.
#[test]
fn seed_takes_a_retired_note_kind_out_of_an_existing_profile() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let (id, _): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config = config_for(&conn, &id).unwrap();
    let mut idea = config.note_kind("note").unwrap().clone();
    idea.key = "idea".into();
    idea.material = true;
    // The order a copy grown kind by kind holds: the note first, a card
    // between, the phrase last.
    let phrase = config.note_kind("phrase").unwrap().clone();
    let note = config.note_kind("note").unwrap().clone();
    config.note_kinds.retain(|kind| kind.is_card());
    let card = config.note_kinds.remove(0);
    config
        .note_kinds
        .splice(0..0, [note, card.clone(), idea, phrase]);
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let kinds: Vec<String> = config_for(&conn, &id)
        .unwrap()
        .note_kinds
        .into_iter()
        .map(|kind| kind.key)
        .collect();
    // The plain kinds take the shipped order in the places they held -
    // the phrase first, where the Notes screen opens - and the cards
    // stand where they stood. The library, which this copy never had,
    // arrived beside the note it follows in the shipped list (v0.112:
    // a newcomer stands after its neighbour, not at the end).
    let plain: Vec<&str> = kinds
        .iter()
        .map(String::as_str)
        .filter(|key| ["phrase", "note", "source"].contains(key))
        .collect();
    assert_eq!(plain, ["phrase", "note", "source"]);
    assert_eq!(kinds[0], "phrase");
    assert_eq!(
        kinds[..3],
        ["phrase".to_string(), "note".to_string(), card.key]
    );
    assert!(!kinds.contains(&"idea".to_string()));
}

// Caught on the owner's own workspace during the live run of v0.72: every
// real workspace predates the stage dial, so without this the craft's stops
// never arrive and the dial silently falls back to the line's generic ones.
#[test]
fn seed_carries_newly_shipped_stages_into_an_existing_profile() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    // A workspace created before stages existed: the list is empty, which
    // is exactly what `#[serde(default)]` leaves behind.
    let id: String = conn
        .query_row("SELECT id FROM profile WHERE key = 'music'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut config = config_for(&conn, &id).unwrap();
    assert!(
        !config.stages.is_empty(),
        "the shipped profile has stages to lose"
    );
    config.stages.clear();
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let gained = config_for(&conn, &id).unwrap().stages;
    assert!(
        !gained.is_empty(),
        "the craft's own stops arrive rather than the line's fallback"
    );
    assert_eq!(
        gained.last().map(|stage| stage.percent),
        Some(100),
        "and the scale still ends where a finished work stands"
    );
}

#[test]
fn a_tier_still_worded_as_it_shipped_takes_the_new_wording() {
    // Studio's song tiers named what would be MADE from the song -
    // "Picture track", "Clip" - and once clips became works of their own
    // a song was being judged "Clip". The keys do not move, so a score
    // already recorded keeps its tier; only the word does.
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let id = active(&conn).unwrap().unwrap().id;

    let mut config = config_for(&conn, &id).unwrap();
    for kind in &mut config.work_kinds {
        for tier in &mut kind.tiers {
            if tier.key == "clip" {
                tier.label = Label::from("Clip");
            }
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![&id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let after = config_for(&conn, &id).unwrap();
    let clip = after
        .work_kinds
        .iter()
        .flat_map(|kind| kind.tiers.iter())
        .find(|tier| tier.key == "clip")
        .expect("the tier keeps its key");
    assert_eq!(
        clip.label, "Strong",
        "the word we shipped is ours to correct"
    );
}

#[test]
fn a_tier_the_owner_reworded_keeps_their_word() {
    // The other half, and the one that matters more: correcting our own
    // wording must never reach across and rewrite theirs.
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let id = active(&conn).unwrap().unwrap().id;

    let mut config = config_for(&conn, &id).unwrap();
    for kind in &mut config.work_kinds {
        for tier in &mut kind.tiers {
            if tier.key == "clip" {
                tier.label = Label::from("Worth a video");
            }
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![&id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let after = config_for(&conn, &id).unwrap();
    let clip = after
        .work_kinds
        .iter()
        .flat_map(|kind| kind.tiers.iter())
        .find(|tier| tier.key == "clip")
        .expect("the tier keeps its key");
    assert_eq!(clip.label, "Worth a video", "their word, untouched");
}

#[test]
fn carrying_stages_forward_puts_an_older_scale_on_the_five_stops() {
    // The seven stops every workspace held until v0.119. A stage is kept
    // on the five places now, so five of the seven are stops no work
    // stands at: the stored list takes the shipped five whole rather
    // than growing into twelve.
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let id = active(&conn).unwrap().unwrap().id;

    let mut config = config_for(&conn, &id).unwrap();
    let shipped = config.stages.clone();
    config.stages = [
        ("idea", 0),
        ("sketch", 17),
        ("half", 33),
        ("nearly", 50),
        ("polish", 67),
        ("done", 83),
        ("final", 100),
    ]
    .into_iter()
    .map(|(key, percent)| config::Stage {
        key: key.to_owned(),
        label: Label::from(format!("{key} (mine)")),
        percent,
        colour: config::MarkColour::Plain,
        hex: None,
    })
    .collect();
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![&id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let stages = config_for(&conn, &id).unwrap().stages;
    assert_eq!(stages, shipped, "the five as they ship");
    assert_eq!(
        stages.iter().map(|stage| stage.percent).collect::<Vec<_>>(),
        config::STOPS
    );
    assert!(config_for(&conn, &id).unwrap().validate().is_empty());
}

#[test]
fn carrying_stages_forward_keeps_the_word_the_owner_gave_a_stop() {
    // On the five already, the words are the owner's: a stop they
    // renamed keeps its name, and gains only the exact colour it lacks.
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let id = active(&conn).unwrap().unwrap().id;

    let mut config = config_for(&conn, &id).unwrap();
    // One of the five dropped, one renamed and left without a colour.
    config.stages.retain(|stage| stage.percent != 75);
    let half = config
        .stages
        .iter_mut()
        .find(|stage| stage.percent == 50)
        .unwrap();
    half.label = Label::from("Mixed, not mastered");
    half.hex = None;
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![&id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let stages = config_for(&conn, &id).unwrap().stages;
    assert_eq!(
        stages.iter().map(|stage| stage.percent).collect::<Vec<_>>(),
        config::STOPS,
        "the missing stop arrives, in its place"
    );
    let half = stages.iter().find(|stage| stage.percent == 50).unwrap();
    assert_eq!(half.label, "Mixed, not mastered", "their word, kept");
    assert_eq!(half.hex.as_deref(), Some("#E8B341"), "its colour, gained");
}

#[test]
fn carrying_stages_forward_leaves_the_users_own_stops_alone() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let id: String = conn
        .query_row("SELECT id FROM profile WHERE key = 'music'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut config = config_for(&conn, &id).unwrap();
    for stage in &mut config.stages {
        stage.label = Label::from(format!("{} (mine)", stage.label));
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let kept = config_for(&conn, &id).unwrap().stages;
    assert!(
        kept.iter()
            .all(|stage| stage.label.as_str().ends_with("(mine)")),
        "renamed stops stay renamed: {:?}",
        kept.iter().map(|s| &s.label).collect::<Vec<_>>()
    );
}

#[test]
fn carrying_meta_fields_forward_leaves_the_users_own_wording_alone() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let (id, _): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    // The user renames a shipped field and retypes another.
    let mut config = config_for(&conn, &id).unwrap();
    for field in &mut config.work_meta_fields {
        if field.key == "premise" {
            field.label = "What it is about".into();
            field.field_type = config::MetaFieldType::Text;
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let fields = config_for(&conn, &id).unwrap().work_meta_fields;

    // Every key at most once. `find` would happily return the user's own
    // field while a shipped duplicate sat behind it, which is exactly what
    // a carry-forward without the key filter produces.
    let premises: Vec<_> = fields
        .iter()
        .filter(|field| field.key == "premise")
        .collect();
    assert_eq!(
        premises.len(),
        1,
        "the field is carried once, not duplicated"
    );
    assert_eq!(premises[0].label, "What it is about");
    assert_eq!(premises[0].field_type, config::MetaFieldType::Text);

    let mut keys: Vec<&str> = fields.iter().map(|field| field.key.as_str()).collect();
    let total = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), total, "no key appears twice: {keys:?}");
}

#[test]
fn seed_carries_newly_shipped_prompts_into_an_existing_profile() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    // A workspace created before the prompts shipped.
    let mut config = active(&conn).unwrap().unwrap().config;
    config.prompts.clear();
    conn.execute(
        "UPDATE profile SET config = ?1 WHERE key = 'music'",
        params![serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let shipped = builtin().unwrap()[0].config.prompts.len();
    assert_eq!(
        active(&conn).unwrap().unwrap().config.prompts.len(),
        shipped
    );
}

#[test]
fn seed_does_not_overwrite_a_reworded_prompt() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let mut config = active(&conn).unwrap().unwrap().config;
    let key = config.prompts[0].key.clone();
    config.prompts[0].label = "My own wording".into();
    conn.execute(
        "UPDATE profile SET config = ?1 WHERE key = 'music'",
        params![serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let reloaded = active(&conn).unwrap().unwrap().config;
    let kept = reloaded.prompts.iter().find(|p| p.key == key).unwrap();
    assert_eq!(kept.label, "My own wording");
    assert_eq!(
        reloaded.prompts.len(),
        builtin().unwrap()[0].config.prompts.len(),
        "no duplicate was added alongside it"
    );
}

#[test]
fn seed_keeps_user_edits_to_a_builtin_profile() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    conn.execute(
        "UPDATE profile SET name = 'My music' WHERE key = 'music'",
        [],
    )
    .unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let profile = list(&conn)
        .unwrap()
        .into_iter()
        .find(|p| p.key == "music")
        .unwrap();
    assert_eq!(profile.name, "My music");
}

#[test]
fn workspace_reports_the_schema_and_counts() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let workspace = workspace(&conn).unwrap();

    assert_eq!(workspace.schema_version, migrations::latest_version());
    assert_eq!(workspace.works, 0);
    assert!(workspace.profile.is_some());
}

#[test]
fn every_builtin_profile_is_sound() {
    for profile in builtin().unwrap() {
        let problems = profile.config.validate();
        assert!(problems.is_empty(), "{}: {problems:?}", profile.key);
    }
}

#[test]
fn a_profile_that_would_misbehave_is_refused_with_every_problem_named() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let profile = active(&conn).unwrap().unwrap();
    let mut broken = profile.config.clone();
    broken.work_kinds[0].axes[0].scale = 0.0;
    let first_tier = broken.work_kinds[0].tiers[0].key.clone();
    broken.work_kinds[0].tiers.push(config::Tier {
        key: first_tier,
        label: "Twice".into(),
        min: 0.0,
    });

    let refused = update_config(&conn, &profile.id, &broken).unwrap_err();
    let refusal = refused.refusal().expect("a refusal");
    assert_eq!(refusal.code, "profile.invalid");
    let problems = refusal.params["problems"].to_string();
    assert!(problems.contains("problem.axisScale"), "{problems}");
    assert!(problems.contains("problem.repeatedKey"), "{problems}");

    let stored = config_for(&conn, &profile.id).unwrap();
    assert!(
        stored.work_kinds[0].axes[0].scale > 0.0,
        "the stored copy must be untouched"
    );
}

/// The layout chosen in Settings is kept on the profile; one no window
/// draws is refused by name and leaves the stored choice as it was.
#[test]
fn the_overview_layout_is_saved_and_an_unknown_one_refused() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let profile = active(&conn).unwrap().unwrap();
    assert!(
        profile.config.overview.is_none(),
        "a shipped profile follows the window's own board"
    );

    let mut chosen = profile.config.clone();
    chosen.overview = Some(config::OverviewConfig {
        layout: "bands".into(),
        widgets: Vec::new(),
    });
    let saved = update_config(&conn, &profile.id, &chosen).unwrap();
    assert_eq!(saved.config.overview.as_ref().unwrap().layout, "bands");

    let mut unknown = chosen.clone();
    unknown.overview.as_mut().unwrap().layout = "carousel".into();
    let message = update_config(&conn, &profile.id, &unknown)
        .unwrap_err()
        .to_string();
    assert!(
        message.contains(
            "the overview's layout is `grid`, `lead`, `bands`, `mosaic` or `sheet`, not `carousel`"
        ),
        "{message}"
    );
    let stored = config_for(&conn, &profile.id).unwrap();
    assert_eq!(stored.overview.unwrap().layout, "bands");
}

/// A workspace written in format 1 comes up in format 2 — and the
/// document in the row is rewritten, so the migration happens once rather
/// than on every read.
#[test]
fn an_older_workspace_is_rewritten_into_format_2_once() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    // Wind the owner's own profile back to format 1: flat vocabulary, no
    // `format`, a kind of their own beside the shipped ones.
    let flat = serde_json::json!({
        "work_kinds": [{ "key": "song", "label": "Song" }, { "key": "poem", "label": "Poem" }],
        "release_kinds": [{ "key": "clip", "label": "Clip" }],
        "collection_kinds": [],
        "version_roles": [{ "key": "lyrics", "label": "Lyrics" }],
        "statuses": [{ "key": "draft", "label": "Draft", "derive": "draft" }],
        "axes": [{ "key": "imagery", "label": "Imagery", "weight": 1.0, "scale": 10.0 }],
        "tiers": [{ "key": "hold", "label": "Hold", "min": 0.0 }],
        "work_meta_fields": []
    });
    conn.execute(
        "INSERT INTO profile (id, key, name, description, config, is_active, is_builtin, created_at, updated_at)
         VALUES ('mine', 'mine', 'Mine', NULL, ?1, 0, 0, 'x', 'x')",
        params![flat.to_string()],
    )
    .unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let raw: String = conn
        .query_row("SELECT config FROM profile WHERE id = 'mine'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let written: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        written["format"],
        config::FORMAT,
        "the row was not rewritten"
    );
    assert!(written.get("axes").is_none());
    let config = config_for(&conn, "mine").unwrap();
    assert_eq!(config.kind("poem").unwrap().axes[0].key, "imagery");
    assert_eq!(config.kind("song").unwrap().tiers.len(), 1);

    // A second start leaves the row alone: same bytes, no churn.
    let before: String = conn
        .query_row(
            "SELECT config || updated_at FROM profile WHERE id = 'mine'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let after: String = conn
        .query_row(
            "SELECT config || updated_at FROM profile WHERE id = 'mine'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(before, after);
}

/// The owner's music workspace, made before videos existed, gains the
/// video and short kinds — with their statuses, roles and kinds of
/// release, and without their axes and tiers. The judgement is theirs to
/// write; the owner's own axes stay exactly as they were.
#[test]
fn an_older_music_workspace_gains_the_video_kinds_without_their_judgement() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    // Before videos: only the song kinds, and the owner's own axes.
    config
        .work_kinds
        .retain(|kind| kind.key == "song" || kind.key == "instrumental");
    for kind in &mut config.work_kinds {
        kind.axes = vec![config::Axis {
            key: "imagery".into(),
            label: "Imagery".into(),
            weight: 1.0,
            scale: 10.0,
            description: None,
            kind: config::AxisKind::Scale,
            options: Vec::new(),
            rubric: Vec::new(),
        }];
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let video = config.kind("video").expect("the video kind arrived");
    assert!(
        video.axes.is_empty(),
        "a stranger's axes must not appear beside the owner's"
    );
    assert!(video.tiers.is_empty());
    assert!(!video.statuses.is_empty(), "but a video can hold a work");
    assert!(video.version_roles.iter().any(|role| role.key == "plot"));
    assert!(!video.release_kinds.is_empty());
    assert!(config.kind("short").is_some());
    // The owner's judgement of a song is untouched.
    let song = config.kind("song").unwrap();
    assert_eq!(song.axes.len(), 1);
    assert_eq!(song.axes[0].key, "imagery");
}

/// Marks had no glyph and statuses no colour before v0.63: a stored copy
/// naming none gains what the shipped profile states, by key, and a
/// glyph or a colour the owner chose stays theirs.
#[test]
fn a_workspace_from_before_glyphs_gains_mark_icons_and_status_colours() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    for mark in &mut config.marks {
        mark.icon = if mark.key == "unsure" {
            Some("flame".into())
        } else {
            None
        };
    }
    for kind in &mut config.work_kinds {
        for status in &mut kind.statuses {
            status.colour = if status.key == "released" {
                Some(config::MarkColour::Bad)
            } else {
                None
            };
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let icon = |key: &str| {
        config
            .marks
            .iter()
            .find(|m| m.key == key)
            .and_then(|m| m.icon.clone())
    };
    assert_eq!(icon("working").as_deref(), Some("wrench"), "arrived");
    assert_eq!(
        icon("unsure").as_deref(),
        Some("flame"),
        "the owner's stays"
    );
    let song = config.kind("song").unwrap();
    let colour = |key: &str| {
        song.statuses
            .iter()
            .find(|s| s.key == key)
            .and_then(|s| s.colour)
    };
    assert_eq!(colour("scored"), Some(config::MarkColour::Accent));
    assert_eq!(
        colour("released"),
        Some(config::MarkColour::Bad),
        "the owner's stays"
    );
    assert_eq!(
        colour("draft"),
        None,
        "a status shipped without a colour gains none"
    );
}

/// The owner's workspace gained the video kinds in v0.57, before scenes
/// existed: its stored video kind names no kinds of shot, no blocks and
/// no `context` role. At the next start it gains all three, on the same
/// terms a role's body arrived — where the stored copy states nothing.
/// The kinds a note can take arrive in a workspace made before them, and
/// a kind the owner wrote themselves is left alone: the same rule every
/// vocabulary follows — matched by key, theirs stays theirs.
#[test]
fn a_workspace_from_before_the_canon_gains_its_cards_and_keeps_its_own_sections() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    // As a workspace written before 0.84 looks: characters and places as
    // plain kinds, no kinds of relation - and one kind the owner already
    // gave sections of their own.
    config.note_kinds = vec![
        config::NoteKind::new("character", "Character"),
        config::NoteKind::new("location", "Place I know"),
        {
            let mut own = config::NoteKind::new("lore", "Lore");
            own.sections = vec![config::CanonSection::new("myth", "Myth")];
            own
        },
    ];
    config.relation_kinds = Vec::new();
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let carried = config_for(&conn, &id).unwrap();
    let character = carried
        .card_kind("character")
        .expect("a character is a card now");
    assert!(character.section("looks").is_some());
    assert_eq!(character.describe_from, ["looks"]);
    assert!(
        carried.card_kind("channel").is_some_and(|k| k.root),
        "the root arrives"
    );
    let lore = carried.card_kind("lore").unwrap();
    assert_eq!(
        lore.sections
            .iter()
            .map(|s| s.key.as_str())
            .collect::<Vec<_>>(),
        ["myth"],
        "sections the owner wrote were overwritten"
    );
    assert!(carried.relation_kinds.iter().any(|k| k.key == "neighbour"));
    assert!(
        matches!(&character.label, Label::PerLocale(words) if words.contains_key("ru")),
        "a kind still named as it shipped did not follow into Russian: {:?}",
        character.label
    );
    assert_eq!(
        carried.card_kind("location").unwrap().label,
        Label::from("Place I know"),
        "a word the owner typed was overwritten"
    );
    assert!(carried.validate().is_empty(), "{:?}", carried.validate());
}

#[test]
fn a_workspace_from_before_note_kinds_gains_them_and_keeps_its_own() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    // As a workspace written before 0.65 looks: no kinds of note at all,
    // plus one the owner added for themselves.
    config.note_kinds = vec![config::NoteKind::new("prop", "Prop")];
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let gained = config_for(&conn, &id).unwrap().note_kinds;
    let keys: Vec<&str> = gained.iter().map(|kind| kind.key.as_str()).collect();
    assert!(
        keys.contains(&"character"),
        "the craft's words arrive: {keys:?}"
    );
    assert!(keys.contains(&"location"));
    assert!(
        keys.contains(&"prop"),
        "and the owner's own stays: {keys:?}"
    );
    assert_eq!(keys[0], "prop", "theirs first, the shipped ones after");
}

/// A list the owner narrowed is left alone.
#[test]
fn a_workspace_with_video_kinds_from_before_scenes_gains_the_storyboard_words() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();

    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    for kind in &mut config.work_kinds {
        if kind.key == "video" {
            kind.shot_types.clear();
            kind.scene_blocks.clear();
            kind.version_roles.retain(|role| role.key != "context");
        }
        if kind.key == "short" {
            // The owner kept one kind of shot on purpose.
            kind.shot_types.retain(|shot| shot.key == "close");
            // And their shorts predate covers at all — the state of
            // every workspace alive when v0.73 landed.
            kind.cover = false;
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let video = config.kind("video").unwrap();
    assert!(!video.shot_types.is_empty(), "the kinds of shot arrived");
    assert!(!video.scene_blocks.is_empty(), "the blocks arrived");
    assert!(
        video.version_roles.iter().any(|role| role.key == "context"),
        "the context role arrived"
    );
    let short = config.kind("short").unwrap();
    assert_eq!(
        short.shot_types.len(),
        1,
        "a list the owner narrowed is theirs"
    );
    assert!(
        short.cover,
        "the cover arrived in a workspace that had the kind already"
    );
}

/// A method still reading as an earlier release shipped it follows the
/// shipped wording; one the owner rewrote stays theirs. And the premiere's
/// tail and the excerpt arrive where the stored copy has neither.
#[test]
fn an_untouched_method_follows_and_a_rewritten_one_stays() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    for prompt in &mut config.prompts {
        if prompt.key == "release-meta" {
            prompt.method = Some(METHODS_BEFORE[0].1.to_owned());
        }
    }
    for kind in &mut config.work_kinds {
        kind.excerpt = false;
        for door in &mut kind.release_kinds {
            for field in &mut door.fields {
                field.premiere = None;
            }
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == "music")
        .unwrap()
        .config;
    let method = |config: &ProfileConfig| {
        config
            .prompts
            .iter()
            .find(|prompt| prompt.key == "release-meta")
            .and_then(|prompt| prompt.method().map(str::to_owned))
    };
    assert_eq!(
        method(&config),
        method(&shipped),
        "the untouched method follows"
    );
    assert_ne!(method(&config).as_deref(), Some(METHODS_BEFORE[0].1.trim()));
    assert!(config.vocabulary("short").excerpt, "the excerpt arrived");
    let title = config.vocabulary("audio").release_kinds[0]
        .fields
        .iter()
        .find(|field| field.key == "title")
        .unwrap();
    assert!(title.premiere.is_some(), "the premiere's tail arrived");

    let mut rewritten = config.clone();
    for prompt in &mut rewritten.prompts {
        if prompt.key == "release-meta" {
            prompt.method = Some("My own way of writing a release".into());
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&rewritten).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    assert_eq!(
        method(&config_for(&conn, &id).unwrap()).as_deref(),
        Some("My own way of writing a release"),
        "a rewritten method stays the owner's"
    );
}

/// A workspace stored at v0.100.1: the audio track's titles for later
/// arrive after its title, the title's limit as it shipped wrong follows,
/// a limit the owner typed stays; the channel's pools arrive after its
/// template, drawn as they shipped; and the method as v0.100.1 shipped it
/// follows the new one.
#[test]
fn a_workspace_of_v0_100_1_takes_its_titles_for_later_and_its_pools() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    let before = METHODS_BEFORE
        .iter()
        .rfind(|(key, _)| *key == "release-meta")
        .unwrap()
        .1;
    for prompt in &mut config.prompts {
        if prompt.key == "release-meta" {
            prompt.method = Some(before.to_owned());
        }
    }
    for kind in &mut config.work_kinds {
        for door in &mut kind.release_kinds {
            door.fields
                .retain(|field| !field.key.starts_with("alt_title"));
            for field in &mut door.fields {
                if field.key == "title" && field.limit.is_some() {
                    field.limit = Some(if kind.key == "video" && door.key == "youtube" {
                        150
                    } else {
                        205
                    });
                }
            }
        }
    }
    for kind in &mut config.note_kinds {
        kind.sections.retain(|section| {
            !["moves", "assemblies", "pinned_forms"].contains(&section.key.as_str())
        });
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == "music")
        .unwrap()
        .config;
    let door = |config: &ProfileConfig, kind: &str, key: &str| {
        config
            .vocabulary(kind)
            .release_kinds
            .iter()
            .find(|door| door.key == key)
            .unwrap()
            .clone()
    };
    let audio = door(&config, "audio", "youtube");
    let keys: Vec<&str> = audio
        .fields
        .iter()
        .map(|field| field.key.as_str())
        .collect();
    assert_eq!(
        keys[..3],
        ["title", "alt_title_1", "alt_title_2"],
        "the titles for later arrived after the title"
    );
    assert_eq!(audio.fields[1].suffix.as_deref(), Some(" (audio)"));
    assert!(
        audio.fields[1].premiere.is_none(),
        "a later title never premieres"
    );
    assert_eq!(
        audio.fields[0].limit,
        Some(100),
        "the limit as it shipped followed"
    );
    assert_eq!(
        door(&config, "video", "premiere").fields[0].limit,
        Some(100)
    );
    assert_eq!(
        door(&config, "video", "youtube").fields[0].limit,
        Some(150),
        "a limit the owner typed stays"
    );

    let channel = config.card_kind("channel").unwrap();
    let keys: Vec<&str> = channel.sections.iter().map(|s| s.key.as_str()).collect();
    let at = keys.iter().position(|key| *key == "template").unwrap();
    assert_eq!(
        keys[at + 1..at + 4],
        ["moves", "assemblies", "pinned_forms"]
    );
    assert_eq!(channel.section("moves").unwrap().draw, Some(8));

    let method = config
        .prompts
        .iter()
        .find(|prompt| prompt.key == "release-meta")
        .and_then(|prompt| prompt.method());
    let shipped_method = shipped
        .prompts
        .iter()
        .find(|prompt| prompt.key == "release-meta")
        .and_then(|prompt| prompt.method());
    assert_eq!(method, shipped_method, "the method of v0.100.1 followed");
    assert_ne!(method, Some(before.trim()));
}

/// A workspace whose actions predate methods: the shipped method
/// arrives where the stored action has none, the shipped `produces`
/// where the stored one says nothing, and a `score` template still
/// reading exactly as it shipped follows the new wording. A template
/// the owner reworded stays, and so does a method they wrote.
#[test]
fn methods_arrive_and_an_unchanged_score_template_follows() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
    for prompt in &mut config.prompts {
        prompt.method = None;
        if prompt.key == "score" {
            prompt.template = SCORE_TEMPLATES_BEFORE_METHODS[0].to_owned();
        }
        if prompt.key == "critique" {
            prompt.produces = None;
        }
        if prompt.key == "polish" {
            prompt.template = "My own polish".into();
            prompt.method = Some("My own way".into());
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let config = config_for(&conn, &id).unwrap();
    let by_key = |key: &str| config.prompts.iter().find(|p| p.key == key).unwrap();
    assert!(by_key("score").method().is_some(), "the method arrived");
    assert!(
        !by_key("score").template.contains("Hook ("),
        "the template no longer names the axes by hand"
    );
    assert_eq!(
        by_key("critique").produces.as_deref(),
        Some("version:critique")
    );
    assert_eq!(by_key("polish").template, "My own polish", "reworded stays");
    assert_eq!(by_key("polish").method.as_deref(), Some("My own way"));
}

/// A workspace made when the profile was still called Music wakes up
/// with it called Studio; one whose owner named it themselves keeps that.
#[test]
fn the_old_shipped_name_follows_the_new_one_and_an_owners_name_stays() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    conn.execute("UPDATE profile SET name = 'Music' WHERE key = 'music'", [])
        .unwrap();
    let novel = db::open_in_memory().unwrap();
    seed(&novel, "novel").unwrap();
    novel
        .execute(
            "UPDATE profile SET name = 'Chapters' WHERE key = 'novel'",
            [],
        )
        .unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();
    seed(&novel, "novel").unwrap();

    let name = |conn: &Connection, key: &str| -> String {
        conn.query_row("SELECT name FROM profile WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .unwrap()
    };
    assert_eq!(name(&conn, "music"), "Studio");
    assert_eq!(
        name(&novel, "novel"),
        "Chapters",
        "a name the owner chose is not taken back"
    );
    let description: String = conn
        .query_row(
            "SELECT description FROM profile WHERE key = 'music'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        description.starts_with("Songs and what goes out for them"),
        "{description}"
    );
}

/// A fresh workspace gets the whole shipped kind, judgement included: there
/// is nobody's vocabulary to keep out of the way.
#[test]
fn a_fresh_workspace_gets_the_video_kinds_whole() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let profile = active(&conn).unwrap().unwrap();
    let video = profile.config.kind("video").unwrap();
    assert!(!video.axes.is_empty());
    assert!(!video.tiers.is_empty());
    assert_eq!(profile.name, "Studio");
}

#[test]
fn seed_carries_kinds_and_scope_onto_actions_stored_without_them() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let mut config = active(&conn).unwrap().unwrap().config;
    for prompt in &mut config.prompts {
        prompt.kinds.clear();
        prompt.scope = None;
    }
    conn.execute(
        "UPDATE profile SET config = ?1 WHERE key = 'music'",
        params![serde_json::to_string(&config).unwrap()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let reloaded = active(&conn).unwrap().unwrap().config;
    let by_key = |key: &str| reloaded.prompts.iter().find(|p| p.key == key).unwrap();
    assert_eq!(by_key("critique").kinds, vec!["song"]);
    assert_eq!(by_key("prompts").scope.as_deref(), Some("scene"));
    assert!(
        by_key("score").kinds.is_empty(),
        "an action for every kind stays so"
    );
}

/// A workspace from before v0.88: its kinds name the parts of a cover
/// (format 2), its scene blocks mark no picture, and the channel's and a
/// character's bans are plain statements read by covers and public texts
/// alike, as they shipped. After the upgrade a kind with parts has a
/// cover, the still is the picture, the bans are bans read by public
/// texts, and the bans of a picture have a section of their own beside
/// them. A section the owner re-read keeps its lenses.
/// v0.94: a workspace whose channel card kind was stored before the
/// house sound gains its section after the house styles - the owner's
/// own sections and their order untouched - and the composition with it.
#[test]
fn a_stored_channel_card_gains_the_house_sound_after_its_styles() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut stored: serde_json::Value = serde_json::from_str(&raw).unwrap();
    for kind in stored["note_kinds"].as_array_mut().unwrap() {
        if let Some(sections) = kind.get_mut("sections").and_then(|v| v.as_array_mut()) {
            sections.retain(|section| section["key"] != "sound");
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, stored.to_string()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let carried = config_for(&conn, &id).unwrap();
    let channel = carried.card_kind("channel").unwrap();
    let keys: Vec<&str> = channel.sections.iter().map(|s| s.key.as_str()).collect();
    let styles = keys.iter().position(|k| *k == "styles").unwrap();
    assert_eq!(keys.get(styles + 1), Some(&"sound"), "{keys:?}");
    let sound = &channel.sections[styles + 1];
    assert_eq!(sound.shape, config::SectionShape::Styles);
    assert_eq!(sound.lenses, vec![config::Lens::Work]);
    assert!(carried.validate().is_empty(), "{:?}", carried.validate());
}

#[test]
fn a_workspace_from_before_the_constructor_gains_covers_and_picture_bans() {
    let conn = db::open_in_memory().unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let (id, raw): (String, String) = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = 'music'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut stored: serde_json::Value = serde_json::from_str(&raw).unwrap();
    stored["format"] = serde_json::json!(2);
    for kind in stored["work_kinds"].as_array_mut().unwrap() {
        let has_cover = kind.get("cover").is_some();
        kind.as_object_mut().unwrap().remove("cover");
        if has_cover {
            kind["cover_blocks"] = serde_json::json!([
                { "key": "picture", "label": "Picture" },
                { "key": "negative", "label": "Negative" }
            ]);
        }
        // `get_mut`, not an index: indexing a missing key writes a null.
        let blocks = kind.get_mut("scene_blocks").and_then(|v| v.as_array_mut());
        for block in blocks.into_iter().flatten() {
            block.as_object_mut().unwrap().remove("picture");
        }
    }
    for kind in stored["note_kinds"].as_array_mut().unwrap() {
        let key = kind["key"].as_str().unwrap_or_default().to_owned();
        let Some(sections) = kind.get_mut("sections").and_then(|v| v.as_array_mut()) else {
            continue;
        };
        sections.retain(|section| section["key"] != "picture_bans");
        for section in sections.iter_mut() {
            if section["key"] == "bans" {
                let object = section.as_object_mut().unwrap();
                object.remove("shape");
                object.insert("lenses".into(), serde_json::json!(["cover", "public"]));
                if key == "channel" {
                    object.insert(
                        "hint".into(),
                        serde_json::json!({
                            "en": "→ the negative prompt and the idea generator",
                            "ru": "→ негатив и генератор идей"
                        }),
                    );
                } else {
                    // The owner read this one again and kept covers on it.
                    object.remove("hint");
                    object.insert("lenses".into(), serde_json::json!(["cover"]));
                }
            }
        }
    }
    conn.execute(
        "UPDATE profile SET config = ?2 WHERE id = ?1",
        params![id, stored.to_string()],
    )
    .unwrap();
    // A copy stored before v0.125 was given nothing yet.
    conn.execute("DELETE FROM shipped_entry", []).unwrap();

    seed(&conn, DEFAULT_KIND).unwrap();

    let carried = config_for(&conn, &id).unwrap();
    assert_eq!(carried.format, config::FORMAT);
    for key in ["video", "audio", "short"] {
        assert!(carried.kind(key).unwrap().cover, "{key} has a cover");
    }
    assert!(!carried.kind("song").unwrap().cover, "a song has none");
    let video = carried.kind("video").unwrap();
    assert_eq!(
        video
            .scene_blocks
            .iter()
            .filter(|block| block.picture)
            .map(|block| block.key.as_str())
            .collect::<Vec<_>>(),
        ["still"]
    );
    let written = config_for(&conn, &id).unwrap();
    let channel = written.card_kind("channel").unwrap();
    let keys: Vec<&str> = channel.sections.iter().map(|s| s.key.as_str()).collect();
    let bans = keys.iter().position(|key| *key == "bans").unwrap();
    assert_eq!(keys[bans + 1], "picture_bans", "{keys:?}");
    let channel_bans = channel.section("bans").unwrap();
    assert_eq!(channel_bans.shape, config::SectionShape::Bans);
    assert_eq!(channel_bans.lenses, [config::Lens::Public]);
    assert_eq!(
        channel.section("picture_bans").unwrap().lenses,
        [config::Lens::Cover]
    );
    let character = written.card_kind("character").unwrap();
    let character_bans = character.section("bans").unwrap();
    assert_eq!(character_bans.shape, config::SectionShape::Bans);
    assert_eq!(
        character_bans.lenses,
        [config::Lens::Cover],
        "lenses the owner set are theirs"
    );
    assert!(character.section("picture_bans").is_some());

    // Idempotent: a second start changes nothing.
    let before: String = conn
        .query_row("SELECT config FROM profile WHERE id = ?1", [&id], |row| {
            row.get(0)
        })
        .unwrap();
    seed(&conn, DEFAULT_KIND).unwrap();
    let after: String = conn
        .query_row("SELECT config FROM profile WHERE id = ?1", [&id], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(before, after);
}

/// A workspace made before compositions existed takes the shipped sound
/// with the types it is picked from - and keeps a composition the owner
/// reshaped as they left it.
#[test]
fn a_stored_profile_gains_the_compositions_it_can_hold() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|p| p.key == "music")
        .unwrap()
        .config;
    let mut old = shipped.clone();
    old.compose.clear();
    old.style_types
        .retain(|t| t.form != config::StyleForm::Phrase);

    let mut stored = old.clone();
    assert!(
        style_types(&mut stored, &shipped),
        "the types of sound arrive"
    );
    assert!(
        compositions(&mut stored, &shipped),
        "and the sound with them"
    );
    assert_eq!(stored.compose, shipped.compose);
    assert!(stored.validate().is_empty(), "{:?}", stored.validate());
    assert!(
        !compositions(&mut stored, &shipped),
        "a second run changes nothing"
    );

    // Without its types the sound would not validate: it waits for them.
    let mut bare = old.clone();
    assert!(!compositions(&mut bare, &shipped));
    assert!(bare.compose.is_empty());

    // The owner's own reading of the sound stays theirs.
    let mut reshaped = stored.clone();
    reshaped.compose[0].parts.truncate(2);
    let mine = reshaped.compose[0].clone();
    assert!(!compositions(&mut reshaped, &shipped));
    assert_eq!(reshaped.compose[0], mine);
}

/// A copy stored before its words spoke two languages (v0.112): the
/// shipped English as plain strings. Each takes the shipped pair; a word
/// the owner wrote, in any language, stays as written - and so does a
/// word of an entry kilna does not ship.
/// A stored Studio still as it shipped before v0.117 loses the leftovers:
/// a song's own fields stand on a song's card alone, and an audio release
/// names no roles. What the owner changed is left as they have it.
#[test]
fn a_stored_profile_loses_the_leftovers_it_did_not_touch() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == "music")
        .unwrap()
        .config;
    // As v0.116 stored it: every song field for every kind or for the
    // four, the audio release with a clip's two roles.
    let mut stored = shipped.clone();
    for field in &mut stored.work_meta_fields {
        if let Some((_, before)) = FIELD_KINDS_BEFORE.iter().find(|(key, _)| *key == field.key) {
            field.kinds = before.iter().map(|kind| (*kind).to_owned()).collect();
        }
    }
    let clip_roles = shipped.vocabulary("video").version_roles.clone();
    for kind in &mut stored.work_kinds {
        if kind.key == "audio" {
            kind.version_roles.clone_from(&clip_roles);
        }
    }
    // And two things the owner made their own.
    let own = |config: &mut ProfileConfig| {
        for field in &mut config.work_meta_fields {
            if field.key == "mood" {
                field.kinds = vec!["song".into(), "video".into()];
            }
        }
    };
    own(&mut stored);
    assert!(
        stored
            .fields_of("audio")
            .iter()
            .any(|field| field.key == "bpm")
    );

    assert!(card_leftovers(&mut stored, &shipped));

    let on_audio: Vec<&str> = stored
        .fields_of("audio")
        .iter()
        .map(|field| field.key.as_str())
        .collect();
    for gone in [
        "bpm", "key", "tempo", "vocal", "language", "tagline", "premise",
    ] {
        assert!(
            !on_audio.contains(&gone),
            "`{gone}` is still on an audio release"
        );
    }
    assert!(on_audio.contains(&"variant") && on_audio.contains(&"duration"));
    assert!(
        stored
            .fields_of("song")
            .iter()
            .any(|field| field.key == "bpm")
    );
    assert!(
        stored
            .fields_of("experiment")
            .iter()
            .any(|field| field.key == "direction")
    );
    assert!(stored.vocabulary("audio").version_roles.is_empty());
    assert!(!stored.vocabulary("video").version_roles.is_empty());
    // The owner's own choice stands.
    assert!(
        stored
            .fields_of("video")
            .iter()
            .any(|field| field.key == "mood")
    );
    // And a second pass finds nothing to do.
    assert!(!card_leftovers(&mut stored, &shipped));
    assert!(stored.validate().is_empty(), "{:?}", stored.validate());
}

#[test]
fn a_word_still_as_it_shipped_in_english_gains_its_second_language() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == DEFAULT_KIND)
        .unwrap()
        .config;
    let english = |label: &config::Label| config::Label::from(label.in_locale("en"));
    let mut stored = shipped.clone();
    for kind in &mut stored.work_kinds {
        kind.label = english(&kind.label);
        for status in &mut kind.statuses {
            status.label = english(&status.label);
        }
        for axis in &mut kind.axes {
            axis.label = english(&axis.label);
            axis.description = axis.description.as_ref().map(english);
            for mark in &mut axis.rubric {
                mark.label = english(&mark.label);
            }
        }
    }
    for prompt in &mut stored.prompts {
        prompt.label = english(&prompt.label);
    }
    // The owner's own words: a kind renamed in Russian, a status reworded
    // in English, a kind of their own.
    stored.work_kinds[0].label = "Трек".into();
    stored.work_kinds[0].statuses[0].label = "Rough".into();
    let mut own = config::WorkKind::new("demo", "Demo");
    own.statuses = vec![config::Status::new("draft", "Draft", Derive::Draft)];
    stored.work_kinds.push(own);

    assert!(second_language(&mut stored, &shipped));

    assert_eq!(stored.work_kinds[0].label, "Трек");
    assert_eq!(stored.work_kinds[0].statuses[0].label, "Rough");
    assert_eq!(
        stored.work_kinds[0].statuses[1].label,
        shipped.work_kinds[0].statuses[1].label
    );
    assert_eq!(stored.work_kinds[1].label, shipped.work_kinds[1].label);
    assert_eq!(stored.work_kinds[1].label.in_locale("ru"), "Видео");
    let judged = shipped
        .work_kinds
        .iter()
        .position(|kind| kind.axes.iter().any(|axis| !axis.rubric.is_empty()))
        .expect("a shipped kind with a rubric");
    assert_eq!(
        serde_json::to_value(&stored.work_kinds[judged].axes).unwrap(),
        serde_json::to_value(&shipped.work_kinds[judged].axes).unwrap(),
        "an axis's word, what it asks and its rubric's marks, met by key and by mark"
    );
    assert_eq!(stored.prompts[0].label, shipped.prompts[0].label);
    // "Draft" of a kind kilna does not ship is the owner's word, however
    // much it looks like a shipped one.
    let demo = stored.kind("demo").unwrap();
    assert_eq!(demo.label, "Demo");
    assert_eq!(demo.statuses[0].label, config::Label::from("Draft"));
    assert!(
        stored.validate().is_empty(),
        "the copy still validates: {:?}",
        stored.validate()
    );
    assert!(
        !second_language(&mut stored, &shipped),
        "and nothing is left to pair the second time"
    );
}

/// A word stored in English alone, as a map, gains the languages it
/// lacks; one whose English the owner changed is left with what it has.
#[test]
fn a_word_kept_in_one_language_gains_the_rest_only_while_its_english_stands() {
    let shipped = serde_json::json!({ "label": { "en": "Song", "ru": "Песня" } });
    let mut same = serde_json::json!({ "label": { "en": "Song" } });
    let mut other = serde_json::json!({ "label": { "en": "Track" } });
    let mut russian = serde_json::json!({ "label": "Песня" });

    assert!(pair_words(&mut same, &shipped));
    assert!(!pair_words(&mut other, &shipped));
    assert!(!pair_words(&mut russian, &shipped));

    assert_eq!(same, shipped);
    assert_eq!(other["label"], serde_json::json!({ "en": "Track" }));
    assert_eq!(russian["label"], "Песня");
}

/// The critique of each craft reads what came before and how it was
/// judged (v0.112). A stored copy still worded as it shipped follows; a
/// reworded one is the owner's.
#[test]
fn a_critique_still_as_it_shipped_comes_to_read_the_context() {
    for profile in builtin().unwrap() {
        let Some(shipped) = profile
            .config
            .prompts
            .iter()
            .find(|prompt| prompt.key == "critique")
        else {
            continue;
        };
        for name in ["previous_version", "score"] {
            assert!(
                shipped.placeholders().iter().any(|found| found == name),
                "{}: the shipped critique reads {{{name}}}",
                profile.key
            );
        }
        let before = TEMPLATES_BEFORE_THE_CONTEXT
            .iter()
            .map(|(_, before)| *before)
            .find(|before| {
                // The one this craft shipped: it reads the craft's own role.
                crate::assistant::prompt::placeholders(before)
                    .iter()
                    .filter(|name| name.starts_with("role:"))
                    .all(|name| shipped.placeholders().contains(name))
            })
            .unwrap_or_else(|| panic!("{}: no earlier critique is kept", profile.key));

        let mut untouched = profile.config.clone();
        let mut reworded = profile.config.clone();
        for (config, template) in [
            (&mut untouched, before.to_owned()),
            (&mut reworded, format!("{before} Be brief.")),
        ] {
            config
                .prompts
                .iter_mut()
                .find(|prompt| prompt.key == "critique")
                .unwrap()
                .template = template;
        }

        assert!(prompt_details(&mut untouched, &profile.config));
        prompt_details(&mut reworded, &profile.config);

        let template = |config: &ProfileConfig| {
            config
                .prompts
                .iter()
                .find(|prompt| prompt.key == "critique")
                .unwrap()
                .template
                .clone()
        };
        assert_eq!(template(&untouched), shipped.template, "{}", profile.key);
        assert!(
            template(&reworded).ends_with("Be brief."),
            "{}",
            profile.key
        );
        assert!(
            untouched.validate().is_empty(),
            "{:?}",
            untouched.validate()
        );
    }
}

/// Studio's action about a song's place in its album reads the collection
/// (v0.112), and reaches a stored copy as any new action does.
#[test]
fn the_album_action_reads_the_collection_and_arrives_in_a_stored_copy() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == DEFAULT_KIND)
        .unwrap()
        .config;
    let place = shipped
        .prompts
        .iter()
        .find(|prompt| prompt.key == "album-place")
        .expect("the shipped action");
    assert!(place.placeholders().iter().any(|name| name == "collection"));

    let mut stored = shipped.clone();
    stored.prompts.retain(|prompt| prompt.key != "album-place");
    let mut ledger = shipped::Ledger::default();
    shipped::carry(&mut stored, &shipped, &mut ledger).unwrap();
    assert!(
        stored
            .prompts
            .iter()
            .any(|prompt| prompt.key == "album-place")
    );
}

/// The anchors of an experiment are a list (v0.112). A stored field still
/// the text it shipped as follows; one the owner retyped is theirs.
#[test]
fn the_anchors_become_a_list_where_they_are_still_the_text_they_shipped_as() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == DEFAULT_KIND)
        .unwrap()
        .config;
    let field = |config: &ProfileConfig, key: &str| {
        config
            .work_meta_fields
            .iter()
            .find(|field| field.key == key)
            .unwrap()
            .field_type
    };
    assert_eq!(field(&shipped, "anchors"), config::MetaFieldType::List);

    let retype = |config: &mut ProfileConfig, to: config::MetaFieldType| {
        config
            .work_meta_fields
            .iter_mut()
            .find(|field| field.key == "anchors")
            .unwrap()
            .field_type = to;
    };
    let mut stored = shipped.clone();
    retype(&mut stored, config::MetaFieldType::Multiline);
    let mut owners = shipped.clone();
    retype(&mut owners, config::MetaFieldType::Text);

    assert!(retyped_fields(&mut stored, &shipped));
    assert!(!retyped_fields(&mut owners, &shipped));

    assert_eq!(field(&stored, "anchors"), config::MetaFieldType::List);
    assert_eq!(field(&owners, "anchors"), config::MetaFieldType::Text);
    assert!(stored.validate().is_empty(), "{:?}", stored.validate());
    assert!(owners.validate().is_empty(), "{:?}", owners.validate());
}

/// A list reads what a text field held before it became one.
#[test]
fn a_list_reads_a_text_stored_before_it_as_its_lines() {
    use config::MetaFieldType;
    assert_eq!(
        MetaFieldType::entries(&serde_json::json!("fuzz bass\n\n  amen break \r\n")),
        ["fuzz bass", "amen break"]
    );
    assert_eq!(
        MetaFieldType::entries(&serde_json::json!(["fuzz bass", " ", 3, "amen break"])),
        ["fuzz bass", "amen break"]
    );
    assert!(MetaFieldType::entries(&serde_json::json!(null)).is_empty());
}

/// The layers of a scene (v0.112) reach a stored kind that has a
/// storyboard, and not one whose board the owner took off.
#[test]
fn the_layers_arrive_beside_a_storyboard_and_nowhere_else() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == DEFAULT_KIND)
        .unwrap()
        .config;
    let video = shipped.kind("video").unwrap();
    assert!(!video.layers.is_empty(), "Studio's clip names its layers");

    let mut stored = shipped.clone();
    for kind in &mut stored.work_kinds {
        kind.layers.clear();
    }
    let short = stored
        .work_kinds
        .iter_mut()
        .find(|kind| kind.key == "short")
        .unwrap();
    short.shot_types.clear();
    short.scene_blocks.clear();
    // Without a board the kind names none again either way: the step
    // that fills an empty board runs first, so it is asked alone here.
    let mut boardless = stored.clone();
    boardless.work_kinds.retain(|kind| kind.key == "short");
    let mut only_short = shipped.clone();
    only_short.work_kinds.retain(|kind| kind.key == "short");
    for kind in &mut only_short.work_kinds {
        kind.shot_types.clear();
        kind.scene_blocks.clear();
    }

    assert!(board_vocabulary(&mut stored, &shipped));
    assert_eq!(
        stored.kind("video").unwrap().layers.len(),
        video.layers.len()
    );
    assert!(!board_vocabulary(&mut boardless, &only_short));
    assert!(boardless.work_kinds[0].layers.is_empty());
}

/// A key shipped later takes its place after the entry it follows in the
/// shipped list, in every keyed list, and an order the owner arranged is
/// left as arranged.
#[test]
fn a_newly_shipped_entry_stands_after_its_shipped_neighbour() {
    let words = |keys: &[&str]| -> Vec<config::Kind> {
        keys.iter().map(|key| config::Kind::new(key, key)).collect()
    };
    let keys = |list: &[config::Kind]| -> Vec<String> {
        list.iter().map(|word| word.key.clone()).collect()
    };
    let shipped = words(&["a", "b", "c", "d"]);

    // The owner put `d` first and never had `b`: it arrives after `a`.
    let mut stored = words(&["d", "a", "c"]);
    assert!(insert_new_keys(&mut stored, &shipped, |word| &word.key));
    assert_eq!(keys(&stored), ["d", "a", "b", "c"]);

    // Nothing before it is stored: it goes to the end rather than first.
    let mut tail = words(&["c"]);
    assert!(insert_new_keys(&mut tail, &words(&["a", "c"]), |word| {
        &word.key
    }));
    assert_eq!(keys(&tail), ["c", "a"]);

    assert!(!insert_new_keys(&mut stored, &shipped, |word| &word.key));
}

/// The empty profile (v0.112): the least a craft can start from, sound,
/// and what a space of a craft kilna does not ship is seeded with.
#[test]
fn the_empty_profile_is_one_kind_with_a_text_and_seeds_a_space() {
    let blank = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == BLANK_KIND)
        .expect("the empty profile ships");
    assert_eq!(blank.config.work_kinds.len(), 1);
    assert!(blank.config.prompts.is_empty());
    assert!(
        blank.config.validate().is_empty(),
        "{:?}",
        blank.config.validate()
    );

    let conn = db::open_in_memory().unwrap();
    seed(&conn, BLANK_KIND).unwrap();
    let active = active(&conn).unwrap().unwrap();
    assert_eq!(active.key, BLANK_KIND);
    assert_eq!(active.config.work_kinds[0].key, "work");
}

/// A type's document seeds an empty space under the type's name, as the
/// profile of the kind it grew from - and is brought up to what ships at
/// once. A space that already has a profile keeps it.
#[test]
fn a_document_seeds_an_empty_space_and_is_carried_forward_as_its_kind() {
    let shipped = builtin()
        .unwrap()
        .into_iter()
        .find(|profile| profile.key == DEFAULT_KIND)
        .unwrap();
    let mut document = shipped.config.clone();
    document.work_kinds[0].label = "Track".into();
    document.prompts.retain(|prompt| prompt.key != "titles");

    let conn = db::open_in_memory().unwrap();
    seed_from(
        &conn,
        DEFAULT_KIND,
        "Night Shift",
        Some("The second channel."),
        &document,
    )
    .unwrap();

    let profiles = list(&conn).unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].key, DEFAULT_KIND);
    assert_eq!(profiles[0].name, "Night Shift");
    assert_eq!(
        profiles[0].description.as_deref(),
        Some("The second channel.")
    );
    assert!(profiles[0].is_active);
    assert_eq!(profiles[0].config.work_kinds[0].label, "Track");
    assert!(
        profiles[0]
            .config
            .prompts
            .iter()
            .any(|prompt| prompt.key == "titles")
    );

    // Asked again of a space that has its profile: nothing changes.
    seed_from(&conn, DEFAULT_KIND, "Another", None, &shipped.config).unwrap();
    assert_eq!(list(&conn).unwrap()[0].name, "Night Shift");

    // A document with something wrong in it seeds nothing.
    let empty = db::open_in_memory().unwrap();
    let mut broken = shipped.config.clone();
    broken.work_kinds[0].statuses.clear();
    let refused = seed_from(&empty, DEFAULT_KIND, "Broken", None, &broken).unwrap_err();
    assert_eq!(refused.refusal().unwrap().code, "profile.invalid");
    assert!(list(&empty).unwrap().is_empty());
}
