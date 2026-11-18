//! What is wrong with a profile document, every problem named.

use std::collections::BTreeSet;

use crate::error::Reason;

use super::*;

impl ProfileConfig {
    /// Everything wrong with the document, in the words a person can act on.
    ///
    /// Empty means the profile is sound. Each problem names the place — "work
    /// kind `song`: axis 3 (`hook`)" — and the rule it breaks, because a
    /// profile is edited by hand and "invalid config" sends the person back
    /// to guess. Only what would make the app misbehave is refused: an empty
    /// label is a taste, a duplicate key is a corruption waiting for the next
    /// score.
    ///
    /// A problem is a [`Reason`] - a code under `problem.` and the values its
    /// sentence needs, the place among them as a reason under `place.` - so
    /// that a window reading Russian says it in Russian. [`Self::validate`] is
    /// the same list in English.
    pub fn problems(&self) -> Vec<Reason> {
        let mut problems = Vec::new();

        unique(
            &mut problems,
            Reason::of("place.workKinds"),
            self.work_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            &mut problems,
            Reason::of("place.collectionKinds"),
            self.collection_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            &mut problems,
            Reason::of("place.metaFields"),
            self.work_meta_fields.iter().map(|f| f.key.clone()),
        );
        self.validate_stages(&mut problems);
        self.validate_vibe(&mut problems);
        unique(
            &mut problems,
            Reason::of("place.styleTypes"),
            self.style_types.iter().map(|t| t.key.clone()),
        );
        for (index, style) in self.style_types.iter().enumerate() {
            let place = style_type_at(index, &style.key);
            unique(
                &mut problems,
                Reason::of("place.families").param("in", place.clone()),
                style.families.iter().map(|f| f.key.clone()),
            );
            if let Some(kind) = &style.canon_kind
                && !self
                    .note_kinds
                    .iter()
                    .any(|n| n.key == *kind && n.is_card())
            {
                problems.push(
                    Reason::of("problem.styleStandIn")
                        .param("place", place.clone())
                        .param("kind", kind.as_str()),
                );
            }
        }
        if self.work_kinds.is_empty() {
            problems.push(Reason::of("problem.noWorkKinds"));
        }
        self.validate_compose(&mut problems);

        for (index, field) in self.work_meta_fields.iter().enumerate() {
            let place = meta_field_at(index, &field.key);
            for kind in &field.kinds {
                if self.kind(kind).is_none() {
                    problems.push(
                        Reason::of("problem.namesUnknownKind")
                            .param("place", place.clone())
                            .param("kind", kind.as_str()),
                    );
                }
            }
            if field.field_type == MetaFieldType::Choice {
                if field.options.is_empty() {
                    problems.push(Reason::of("problem.choiceEmpty").param("place", place.clone()));
                }
                unique(
                    &mut problems,
                    Reason::of("place.options").param("in", place.clone()),
                    field.options.iter().map(|option| option.key.clone()),
                );
                if let Some(default) = &field.default
                    && !default
                        .as_str()
                        .is_some_and(|key| field.options.iter().any(|option| option.key == key))
                {
                    problems.push(
                        Reason::of("problem.metaDefaultNotOption")
                            .param("place", place.clone())
                            .param("default", default.to_string()),
                    );
                }
            } else if !field.options.is_empty() {
                problems
                    .push(Reason::of("problem.metaOptionsNotChoice").param("place", place.clone()));
            }
            if let Some(first) = &field.numbered_from {
                if field.field_type != MetaFieldType::Text {
                    problems.push(
                        Reason::of("problem.metaNumberedNotText").param("place", place.clone()),
                    );
                }
                if crate::work::numbering::Pattern::of(first).is_none() {
                    problems.push(
                        Reason::of("problem.metaNumberedNoDigits")
                            .param("place", place.clone())
                            .param("first", first.as_str()),
                    );
                }
                // Two answers to "what does a new work start with" - one of
                // them would be quietly ignored.
                if field.default.is_some() {
                    problems.push(
                        Reason::of("problem.metaNumberedWithDefault").param("place", place.clone()),
                    );
                }
            }
        }

        for (index, kind) in self.work_kinds.iter().enumerate() {
            let place = work_kind_at(index, &kind.key);
            kind.validate_into(&mut problems, &place);
        }

        // A kept trial goes into a role of another kind's works: one no other
        // kind has would make "Into a work…" a menu of nothing. The anchors
        // are a field of the experiment, read line by line.
        for (index, kind) in self.work_kinds.iter().enumerate() {
            let Some(lab) = &kind.lab else {
                continue;
            };
            let place = work_kind_at(index, &kind.key);
            if let Some(role) = lab.harvest.as_deref()
                && self.harvest_kinds(role, &kind.key).next().is_none()
            {
                problems.push(
                    Reason::of("problem.labHarvest")
                        .param("place", place.clone())
                        .param("role", role),
                );
            }
            if let Some(field) = lab.anchors.as_deref() {
                match self.work_meta_fields.iter().find(|f| f.key == field) {
                    None => problems.push(
                        Reason::of("problem.labAnchorsUnknown")
                            .param("place", place.clone())
                            .param("field", field),
                    ),
                    Some(found) if !found.applies_to(&kind.key) => problems.push(
                        Reason::of("problem.labAnchorsNotCarried")
                            .param("place", place.clone())
                            .param("field", field),
                    ),
                    Some(found)
                        if !matches!(
                            found.field_type,
                            MetaFieldType::Text | MetaFieldType::Multiline | MetaFieldType::List
                        ) =>
                    {
                        problems.push(
                            Reason::of("problem.labAnchorsNotText")
                                .param("place", place.clone())
                                .param("field", field),
                        );
                    }
                    Some(_) => {}
                }
            }
        }

        self.validate_boards(&mut problems);

        unique(
            &mut problems,
            Reason::of("place.noteKinds"),
            self.note_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            &mut problems,
            Reason::of("place.relationKinds"),
            self.relation_kinds.iter().map(|k| k.key.clone()),
        );
        let roots: Vec<&str> = self
            .note_kinds
            .iter()
            .filter(|kind| kind.root)
            .map(|kind| kind.key.as_str())
            .collect();
        if roots.len() > 1 {
            problems.push(
                Reason::of("problem.manyRoots")
                    .param("count", roots.len())
                    .param("kinds", roots.join(", ")),
            );
        }
        for (index, kind) in self.note_kinds.iter().enumerate() {
            let place = note_kind_at(index, &kind.key);
            kind.validate_into(&mut problems, &place, &self.note_kinds);
        }

        if let Some(count) = self.cover_ideas
            && count > crate::cover::idea::MOST
        {
            problems.push(
                Reason::of("problem.coverIdeasMost")
                    .param("most", crate::cover::idea::MOST)
                    .param("count", count),
            );
        }

        if let Some(rhythm) = &self.rhythm {
            if rhythm.every_days == 0 && !rhythm.by_weekdays() {
                problems.push(Reason::of("problem.rhythmDays"));
            }
            // A day named twice is a typo; a target of none is no target; a
            // zone the database does not know cannot give an hour (v0.150).
            let mut days = rhythm.weekdays.clone();
            days.sort();
            days.dedup();
            if days.len() != rhythm.weekdays.len() {
                problems.push(Reason::of("problem.rhythmWeekdayTwice"));
            }
            if rhythm.monthly_target == Some(0) {
                problems.push(Reason::of("problem.rhythmTarget"));
            }
            if let Some(zone) = &rhythm.timezone
                && !super::rhythm::is_time_zone(zone)
            {
                problems.push(Reason::of("problem.rhythmZone").param("zone", zone.as_str()));
            }
            if let Some(time) = &rhythm.default_time
                && !crate::time::is_clock_time(time)
            {
                problems.push(Reason::of("problem.rhythmTime").param("time", time.as_str()));
            }
            // A goal names a tier or a kind the profile has: one that names
            // neither would stand unmet every month and say so every month.
            let mut counted: Vec<&GoalOf> = Vec::new();
            for goal in &rhythm.monthly {
                match &goal.of {
                    GoalOf::Tier(key) => {
                        let known = self
                            .work_kinds
                            .iter()
                            .any(|kind| kind.tiers.iter().any(|tier| tier.key == *key));
                        if !known {
                            problems
                                .push(Reason::of("problem.goalTier").param("key", key.as_str()));
                        }
                    }
                    GoalOf::Kind(key) => {
                        if !self.work_kinds.iter().any(|kind| kind.key == *key) {
                            problems
                                .push(Reason::of("problem.goalKind").param("key", key.as_str()));
                        }
                    }
                }
                if goal.count == 0 {
                    problems.push(Reason::of("problem.goalCount"));
                }
                if counted.contains(&&goal.of) {
                    problems.push(Reason::of("problem.goalTwice"));
                }
                counted.push(&goal.of);
            }
        }

        if let Some(overview) = &self.overview {
            overview.validate_into(&mut problems);
        }

        self.validate_prompts(&mut problems);

        problems
    }

    /// [`Self::problems`] said in English, the one wording the locale holds:
    /// for the log, for tests, and for whatever reads the document without a
    /// window.
    pub fn validate(&self) -> Vec<String> {
        self.problems().iter().map(Reason::english).collect()
    }

    /// The lines of a work's feel (FR-02): each with words on it, no two
    /// under one key, a mood's colour written as `#RRGGBB`, a character's
    /// shape one the window can draw, a complexity of one, two or three
    /// strokes - a mark is built of exactly these.
    pub(super) fn validate_vibe(&self, problems: &mut Vec<Reason>) {
        let Some(axes) = &self.vibe else {
            return;
        };
        let mut line = |place: Reason, keys: Vec<String>| {
            if keys.is_empty() {
                problems.push(Reason::of("problem.vibeEmpty").param("what", place.clone()));
            }
            unique(problems, place, keys.into_iter());
        };
        line(
            Reason::of("place.moods"),
            axes.moods.iter().map(|m| m.key.clone()).collect(),
        );
        line(
            Reason::of("place.characters"),
            axes.characters.iter().map(|c| c.key.clone()).collect(),
        );
        line(
            Reason::of("place.complexities"),
            axes.complexities.iter().map(|c| c.key.clone()).collect(),
        );
        for mood in &axes.moods {
            let hex = &mood.color;
            if !(hex.len() == 7
                && hex.starts_with('#')
                && hex[1..].chars().all(|c| c.is_ascii_hexdigit()))
            {
                problems.push(
                    Reason::of("problem.vibeColour")
                        .param("key", mood.key.clone())
                        .param("colour", hex.clone()),
                );
            }
        }
        for complexity in &axes.complexities {
            if !(1..=3).contains(&complexity.strokes) {
                problems.push(
                    Reason::of("problem.vibeStrokes")
                        .param("key", complexity.key.clone())
                        .param("strokes", i64::from(complexity.strokes)),
                );
            }
        }
    }

    /// The stops of the dial (FR-03): each on one of the five places, no two
    /// on the same one, a colour written as `#RRGGBB`. A stop anywhere else
    /// is a stop no work can stand at, since a stage is kept on the scale;
    /// two on one place is a dial that says one of them at random.
    pub(super) fn validate_stages(&self, problems: &mut Vec<Reason>) {
        unique(
            problems,
            Reason::of("place.stages"),
            self.stages.iter().map(|stage| stage.key.clone()),
        );
        let mut taken = BTreeSet::new();
        for stage in &self.stages {
            if !STOPS.contains(&stage.percent) {
                problems.push(
                    Reason::of("problem.stageOffScale")
                        .param("key", stage.key.clone())
                        .param("percent", stage.percent),
                );
            } else if !taken.insert(stage.percent) {
                problems.push(
                    Reason::of("problem.stageSharesStop")
                        .param("key", stage.key.clone())
                        .param("percent", stage.percent),
                );
            }
            if let Some(hex) = &stage.hex
                && !(hex.len() == 7
                    && hex.starts_with('#')
                    && hex[1..].chars().all(|c| c.is_ascii_hexdigit()))
            {
                problems.push(
                    Reason::of("problem.stageColour")
                        .param("key", stage.key.clone())
                        .param("colour", hex.clone()),
                );
            }
        }
    }

    /// The boards, and the conceptions that lay them out (ADR 0065).
    ///
    /// A board with no kind of card, or a kind with no field, is a board
    /// nothing can be put on. A brief that counted as a version would be
    /// taken for a draft of the work - counted in the catalogue, read by
    /// the guard of repeats, made the work's current text - so the role a
    /// conception names must say it is not one. The idea is written by
    /// "Refine" as text into a field of the work, so it is one, and of the
    /// kind.
    pub(super) fn validate_boards(&self, problems: &mut Vec<Reason>) {
        unique(
            problems,
            Reason::of("place.boards"),
            self.boards.iter().map(|b| b.key.clone()),
        );
        for (index, board) in self.boards.iter().enumerate() {
            let place = board_at(index, &board.key);
            if board.cards.is_empty() {
                problems.push(Reason::of("problem.boardHoldsNoCard").param("place", place.clone()));
            }
            unique(
                problems,
                Reason::of("place.cardKinds").param("in", place.clone()),
                board.cards.iter().map(|c| c.key.clone()),
            );
            for (at, card) in board.cards.iter().enumerate() {
                let place = card_kind_at(&place, at, &card.key);
                if card.fields.is_empty() {
                    problems.push(Reason::of("problem.cardNoField").param("place", place.clone()));
                }
                unique(
                    problems,
                    Reason::of("place.cardFields").param("in", place.clone()),
                    card.fields.iter().map(|f| f.key.clone()),
                );
                if card.library && self.library().is_none() {
                    problems.push(
                        Reason::of("problem.cardLinksNoLibrary").param("place", place.clone()),
                    );
                }
                // One answer to "where were the quotes checked", and only on
                // a card that holds a quote to check.
                let shaped = |shape: FieldShape| {
                    card.fields
                        .iter()
                        .filter(|field| field.shape == shape)
                        .count()
                };
                if shaped(FieldShape::Proof) > 1 {
                    problems.push(
                        Reason::of("problem.cardManyProofs")
                            .param("place", place.clone())
                            .param("count", shaped(FieldShape::Proof)),
                    );
                }
                if shaped(FieldShape::Proof) > 0 && shaped(FieldShape::Quote) == 0 {
                    problems.push(
                        Reason::of("problem.cardProofWithoutQuote").param("place", place.clone()),
                    );
                }
            }
        }

        let libraries: Vec<&str> = self
            .note_kinds
            .iter()
            .filter(|kind| kind.library)
            .map(|kind| kind.key.as_str())
            .collect();
        if libraries.len() > 1 {
            problems.push(
                Reason::of("problem.manyLibraries")
                    .param("count", libraries.len())
                    .param("kinds", libraries.join(", ")),
            );
        }

        for (index, kind) in self.work_kinds.iter().enumerate() {
            let Some(conception) = &kind.conception else {
                continue;
            };
            let place = work_kind_at(index, &kind.key);
            match kind
                .version_roles
                .iter()
                .find(|role| role.key == conception.brief)
            {
                None => problems.push(
                    Reason::of("problem.briefNotARole")
                        .param("place", place.clone())
                        .param("role", conception.brief.as_str()),
                ),
                Some(role) if role.comments_on.is_some() || role.counts_as_a_version() => {
                    problems.push(
                        Reason::of("problem.briefCounts")
                            .param("place", place.clone())
                            .param("role", conception.brief.as_str()),
                    );
                }
                Some(_) => {}
            }
            if let Some(field) = conception.idea.as_deref() {
                match self.work_meta_fields.iter().find(|f| f.key == field) {
                    None => problems.push(
                        Reason::of("problem.ideaUnknown")
                            .param("place", place.clone())
                            .param("field", field),
                    ),
                    Some(found) if !found.applies_to(&kind.key) => problems.push(
                        Reason::of("problem.ideaNotCarried")
                            .param("place", place.clone())
                            .param("field", field),
                    ),
                    Some(found)
                        if !matches!(
                            found.field_type,
                            MetaFieldType::Text | MetaFieldType::Multiline
                        ) =>
                    {
                        problems.push(
                            Reason::of("problem.ideaNotText")
                                .param("place", place.clone())
                                .param("field", field),
                        );
                    }
                    Some(_) => {}
                }
            }
            for role in &conception.with {
                if *role == conception.brief || !kind.version_roles.iter().any(|r| r.key == *role) {
                    problems.push(
                        Reason::of("problem.sendsNoRole")
                            .param("place", place.clone())
                            .param("role", role.as_str()),
                    );
                }
            }
            unique(
                problems,
                Reason::of("place.conceptionBoards").param("in", place.clone()),
                conception.boards.iter().cloned(),
            );
            for board in &conception.boards {
                if self.board(board).is_none() {
                    problems.push(
                        Reason::of("problem.conceptionBoardUnknown")
                            .param("place", place.clone())
                            .param("board", board.as_str()),
                    );
                }
            }
        }
    }

    /// The compositions, checked against the vocabulary they name: a role a
    /// kind does not have, a type that is not a phrase, a field the kind does
    /// not carry. And every type of phrase stands in one: a phrase is written
    /// into a text, and a type no composition reads would be a shelf of the
    /// dictionary nothing is ever built from.
    pub(super) fn validate_compose(&self, problems: &mut Vec<Reason>) {
        unique(
            problems,
            Reason::of("place.compositions"),
            self.compose.iter().map(|c| c.key.clone()),
        );
        for (index, composition) in self.compose.iter().enumerate() {
            let place = composition_at(index, &composition.key);
            if composition.kinds.is_empty() {
                problems.push(Reason::of("problem.composeNoKinds").param("place", place.clone()));
            }
            for kind in &composition.kinds {
                match self.kind(kind) {
                    None => problems.push(
                        Reason::of("problem.namesUnknownKind")
                            .param("place", place.clone())
                            .param("kind", kind.as_str()),
                    ),
                    Some(found) => {
                        if !found
                            .version_roles
                            .iter()
                            .any(|role| role.key == composition.role)
                        {
                            problems.push(
                                Reason::of("problem.composeNoRole")
                                    .param("place", place.clone())
                                    .param("role", composition.role.as_str())
                                    .param("kind", kind.as_str()),
                            );
                        }
                    }
                }
            }
            if composition.parts.is_empty() {
                problems.push(Reason::of("problem.composeNoParts").param("place", place.clone()));
            }
            unique(
                problems,
                Reason::of("place.compositionTypes").param("in", place.clone()),
                composition.parts.iter().map(|p| p.type_key.clone()),
            );
            for part in &composition.parts {
                match self.style_type(&part.type_key) {
                    None => problems.push(
                        Reason::of("problem.composeNotAStyle")
                            .param("place", place.clone())
                            .param("type", part.type_key.as_str()),
                    ),
                    Some(style) if style.form != StyleForm::Phrase => problems.push(
                        Reason::of("problem.composeNotAPhrase")
                            .param("place", place.clone())
                            .param("type", part.type_key.as_str()),
                    ),
                    Some(_) => {}
                }
                if let Some(max) = part.max
                    && (max == 0 || part.min > max)
                {
                    problems.push(
                        Reason::of("problem.composeRange")
                            .param("place", place.clone())
                            .param("type", part.type_key.as_str())
                            .param("min", part.min)
                            .param("max", max),
                    );
                }
            }
            for field in &composition.fields {
                match self.work_meta_fields.iter().find(|f| f.key == field.field) {
                    None => problems.push(
                        Reason::of("problem.composeFieldUnknown")
                            .param("place", place.clone())
                            .param("field", field.field.as_str()),
                    ),
                    Some(found) => {
                        for kind in &composition.kinds {
                            if !found.applies_to(kind) {
                                problems.push(
                                    Reason::of("problem.composeFieldNotCarried")
                                        .param("place", place.clone())
                                        .param("field", field.field.as_str())
                                        .param("kind", kind.as_str()),
                                );
                            }
                        }
                    }
                }
                if !field.template.contains("{value}") {
                    problems.push(
                        Reason::of("problem.composeTemplate")
                            .param("place", place.clone())
                            .param("field", field.field.as_str()),
                    );
                }
            }
            if composition.limit == Some(0) {
                problems.push(Reason::of("problem.composeLimit").param("place", place.clone()));
            }
        }
        for style in &self.style_types {
            if style.form == StyleForm::Phrase
                && style.retired.is_none()
                && !self
                    .compose
                    .iter()
                    .any(|c| c.parts.iter().any(|p| p.type_key == style.key))
            {
                problems.push(Reason::of("problem.phraseUnused").param("key", style.key.as_str()));
            }
        }
        // One text is written one way: two compositions of one role for one
        // kind would be two answers to which phrases it is read against.
        for (index, composition) in self.compose.iter().enumerate() {
            for later in &self.compose[index + 1..] {
                if later.role == composition.role
                    && let Some(kind) = later.kinds.iter().find(|k| composition.kinds.contains(k))
                {
                    problems.push(
                        Reason::of("problem.composeClash")
                            .param("first", composition.key.as_str())
                            .param("second", later.key.as_str())
                            .param("role", later.role.as_str())
                            .param("kind", kind.as_str()),
                    );
                }
            }
        }
    }

    /// The actions, checked against the vocabulary they read: a placeholder
    /// no kind of the action fills, a role the answer cannot be kept in, a
    /// storyboard a kind does not have. Refused at save rather than found
    /// as a hole in a prompt — the predecessor let an edited template lose
    /// its text placeholder and sent critiques of nothing for a month.
    pub(super) fn validate_prompts(&self, problems: &mut Vec<Reason>) {
        use crate::assistant::prompt::{
            BOARD_SCOPE, CANON_SCOPE, COMMENT_SCOPE, COVER_SCOPE, LAB_SCOPE, PHRASES_SCOPE,
            Produces, RELEASE_SCOPE, SCENE_SCOPE, SELECTION_SCOPE, STYLE_SCOPE, Scope,
            is_known_placeholder,
        };

        unique(
            problems,
            Reason::of("place.actions"),
            self.prompts.iter().map(|prompt| prompt.key.clone()),
        );

        for (index, prompt) in self.prompts.iter().enumerate() {
            let place = action_at(index, &prompt.key);
            // A button sends its message; a playbook is read by its method.
            // Either way a skill says something (FR-17).
            if prompt.show_in_ui && !prompt.sends() {
                problems.push(Reason::of("problem.actionNoMessage").param("place", place.clone()));
            }
            if !prompt.show_in_ui && prompt.method().is_none() {
                problems.push(Reason::of("problem.playbookNoMethod").param("place", place.clone()));
            }
            if let Some(category) = prompt.category.as_deref()
                && !crate::assistant::prompt::CATEGORIES.contains(&category)
            {
                problems.push(
                    Reason::of("problem.actionCategory")
                        .param("place", place.clone())
                        .param("category", category)
                        .param("known", crate::assistant::prompt::CATEGORIES.join(", ")),
                );
            }
            for kind in &prompt.kinds {
                if self.kind(kind).is_none() {
                    problems.push(
                        Reason::of("problem.namesUnknownKind")
                            .param("place", place.clone())
                            .param("kind", kind.as_str()),
                    );
                }
            }
            if let Some(scope) = prompt.scope.as_deref().map(str::trim)
                && scope != SCENE_SCOPE
                && scope != STYLE_SCOPE
                && scope != COMMENT_SCOPE
                && scope != CANON_SCOPE
                && scope != SELECTION_SCOPE
                && scope != RELEASE_SCOPE
                && scope != COVER_SCOPE
                && scope != PHRASES_SCOPE
                && scope != LAB_SCOPE
                && scope != BOARD_SCOPE
                && scope != "work"
            {
                problems.push(
                    Reason::of("problem.actionScope")
                        .param("place", place.clone())
                        .param("scope", scope),
                );
            }
            if !prompt.produces_is_known() {
                problems.push(
                    Reason::of("problem.actionProduces")
                        .param("place", place.clone())
                        .param(
                            "produces",
                            prompt.produces.as_deref().unwrap_or_default().trim(),
                        ),
                );
            }

            // The kinds the action is offered on: the ones it names, or all.
            let kinds: Vec<&WorkKind> = if prompt.kinds.is_empty() {
                self.work_kinds.iter().collect()
            } else {
                prompt.kinds.iter().filter_map(|k| self.kind(k)).collect()
            };
            let lacking = |has: &dyn Fn(&WorkKind) -> bool| -> Vec<String> {
                kinds
                    .iter()
                    .filter(|kind| !has(kind))
                    .map(|kind| format!("`{}`", kind.key))
                    .collect()
            };
            fn has_role<'a>(role: &'a str) -> impl Fn(&WorkKind) -> bool + 'a {
                move |kind: &WorkKind| kind.version_roles.iter().any(|r| r.key == role)
            }
            let has_board =
                |kind: &WorkKind| !kind.shot_types.is_empty() || !kind.scene_blocks.is_empty();

            let placeholders = prompt.placeholders();
            for name in &placeholders {
                let placeholder = format!("{{{name}}}");
                if !is_known_placeholder(name) {
                    problems.push(
                        Reason::of("problem.readsNothing")
                            .param("place", place.clone())
                            .param("placeholder", placeholder),
                    );
                    continue;
                }
                if let Some(role) = name.strip_prefix("role:") {
                    let missing = lacking(&has_role(role));
                    if !missing.is_empty() {
                        problems.push(
                            by_count(
                                missing.len(),
                                Reason::of("problem.readsRoleLackedOne"),
                                Reason::of("problem.readsRoleLackedMany"),
                            )
                            .param("place", place.clone())
                            .param("placeholder", placeholder.clone())
                            .param("kinds", missing.join(", "))
                            .param("role", role),
                        );
                    }
                }
                if (name == "styles" || name == "style_library") && self.style_types.is_empty() {
                    problems
                        .push(Reason::of("problem.readsNoStyles").param("place", place.clone()));
                }
                if name == "scenes" || name == "scene" {
                    let missing = lacking(&has_board);
                    if !missing.is_empty() {
                        problems.push(
                            by_count(
                                missing.len(),
                                Reason::of("problem.readsNoStoryboardOne"),
                                Reason::of("problem.readsNoStoryboardMany"),
                            )
                            .param("place", place.clone())
                            .param("placeholder", placeholder.clone())
                            .param("kinds", missing.join(", ")),
                        );
                    }
                }
            }
            let reads_scene = placeholders.iter().any(|name| name == "scene");
            match prompt.scope() {
                Scope::Scene if !reads_scene => {
                    problems
                        .push(Reason::of("problem.sceneNeverRead").param("place", place.clone()));
                }
                Scope::Work if reads_scene => {
                    problems
                        .push(Reason::of("problem.sceneNotAbout").param("place", place.clone()));
                }
                // A card is given whole, ahead of the template, and the
                // template goes as written: a placeholder in it would reach
                // the model as braces.
                Scope::Canon if !placeholders.is_empty() => {
                    problems.push(
                        Reason::of("problem.canonReadsPlaceholders")
                            .param("place", place.clone())
                            .param("placeholder", format!("{{{}}}", placeholders[0])),
                    );
                }
                _ => {}
            }
            // The week is counted from the person's day, which the window
            // hands over only with an action about a work (v0.106).
            if placeholders
                .iter()
                .any(|name| name == crate::assistant::prompt::WEEK)
                && !matches!(
                    prompt.scope(),
                    Scope::Work | Scope::Selection | Scope::Scene
                )
            {
                problems.push(Reason::of("problem.weekScope").param("place", place.clone()));
            }
            match prompt.produces() {
                Produces::Version(role) => {
                    let missing = lacking(&has_role(&role));
                    if !missing.is_empty() {
                        problems.push(
                            by_count(
                                missing.len(),
                                Reason::of("problem.producesVersionLackedOne"),
                                Reason::of("problem.producesVersionLackedMany"),
                            )
                            .param("place", place.clone())
                            .param("kinds", missing.join(", "))
                            .param("role", role.as_str()),
                        );
                    }
                }
                Produces::Scenes(change) => {
                    let missing = lacking(&has_board);
                    if !missing.is_empty() {
                        problems.push(
                            by_count(
                                missing.len(),
                                Reason::of("problem.producesScenesLackedOne"),
                                Reason::of("problem.producesScenesLackedMany"),
                            )
                            .param("place", place.clone())
                            .param("kinds", missing.join(", ")),
                        );
                    }
                    if prompt.scope() == Scope::Scene
                        && change != crate::assistant::proposal::BoardChange::Revise
                    {
                        problems.push(
                            Reason::of("problem.sceneWholeBoard").param("place", place.clone()),
                        );
                    }
                }
                Produces::Comment | Produces::Reply if prompt.scope() != Scope::Comment => {
                    problems.push(
                        Reason::of("problem.commentScope")
                            .param("place", place.clone())
                            .param(
                                "produces",
                                prompt.produces.as_deref().unwrap_or_default().trim(),
                            ),
                    );
                }
                Produces::Description if prompt.scope() != Scope::Style => {
                    problems
                        .push(Reason::of("problem.descriptionScope").param("place", place.clone()));
                }
                Produces::CardPrompt | Produces::CardNote if prompt.scope() != Scope::Canon => {
                    problems
                        .push(Reason::of("problem.cardPromptScope").param("place", place.clone()));
                }
                Produces::Canon
                    if matches!(prompt.scope(), Scope::Scene | Scope::Style | Scope::Comment) =>
                {
                    problems.push(Reason::of("problem.canonScope").param("place", place.clone()));
                }
                Produces::Release if prompt.scope() != Scope::Release => {
                    problems.push(Reason::of("problem.releaseScope").param("place", place.clone()));
                }
                Produces::CoverIdeas if prompt.scope() != Scope::Cover => {
                    problems
                        .push(Reason::of("problem.coverIdeasScope").param("place", place.clone()));
                }
                Produces::Words if prompt.scope() != Scope::Work => {
                    problems.push(Reason::of("problem.wordsScope").param("place", place.clone()));
                }
                Produces::Bricks if prompt.scope() != Scope::Phrases => {
                    problems.push(Reason::of("problem.bricksScope").param("place", place.clone()));
                }
                Produces::Trials if prompt.scope() != Scope::Lab => {
                    problems.push(Reason::of("problem.trialsScope").param("place", place.clone()));
                }
                Produces::Cards(_) if prompt.scope() != Scope::Board => {
                    problems.push(Reason::of("problem.cardsScope").param("place", place.clone()));
                }
                // A board's cards land on the board the action names, on
                // every kind it is offered on.
                Produces::Cards(board) => {
                    if self.board(&board).is_none() {
                        problems.push(
                            Reason::of("problem.cardsUnknownBoard")
                                .param("place", place.clone())
                                .param("board", board.as_str()),
                        );
                    }
                    let missing = lacking(&|kind: &WorkKind| {
                        kind.conception
                            .as_ref()
                            .is_some_and(|conception| conception.boards.contains(&board))
                    });
                    if !missing.is_empty() {
                        problems.push(
                            by_count(
                                missing.len(),
                                Reason::of("problem.cardsNoBoardOne"),
                                Reason::of("problem.cardsNoBoardMany"),
                            )
                            .param("place", place.clone())
                            .param("board", board.as_str())
                            .param("kinds", missing.join(", ")),
                        );
                    }
                }
                // The idea and the brief, refined: a revision of the brief of
                // a work prepared on a conception, offered on such kinds only.
                Produces::Conception => {
                    if prompt.scope() != Scope::Work {
                        problems.push(
                            Reason::of("problem.conceptionScope").param("place", place.clone()),
                        );
                    }
                    let missing = lacking(&|kind: &WorkKind| kind.conception.is_some());
                    if !missing.is_empty() {
                        problems.push(
                            by_count(
                                missing.len(),
                                Reason::of("problem.conceptionLackedOne"),
                                Reason::of("problem.conceptionLackedMany"),
                            )
                            .param("place", place.clone())
                            .param("kinds", missing.join(", ")),
                        );
                    }
                }
                Produces::Score
                | Produces::Prose
                | Produces::Comment
                | Produces::Reply
                | Produces::Description
                | Produces::Canon
                | Produces::CardPrompt
                | Produces::CardNote
                | Produces::Release
                | Produces::CoverIdeas
                | Produces::Words
                | Produces::Bricks
                | Produces::Trials => {}
            }
            // An action about a board proposes its cards and reads the board;
            // `{cards}` is filled for it alone.
            let reads_cards = placeholders.iter().any(|name| name == "cards");
            if prompt.scope() == Scope::Board {
                if !matches!(prompt.produces(), Produces::Cards(_)) {
                    problems
                        .push(Reason::of("problem.boardMustProduce").param("place", place.clone()));
                }
                if !reads_cards {
                    problems
                        .push(Reason::of("problem.boardNeverReads").param("place", place.clone()));
                }
            } else if reads_cards {
                problems
                    .push(Reason::of("problem.cardsNotAboutBoard").param("place", place.clone()));
            }
            // The conception is read only off a work prepared on one: on
            // another kind the action would be refused every time it ran.
            if let Some(name) = placeholders.iter().find(|name| {
                matches!(
                    name.as_str(),
                    "conception" | "prepared" | "starred" | "order"
                ) || name.starts_with("starred:")
            }) {
                let missing = lacking(&|kind: &WorkKind| kind.conception.is_some());
                if !missing.is_empty() {
                    problems.push(
                        by_count(
                            missing.len(),
                            Reason::of("problem.readsConceptionLackedOne"),
                            Reason::of("problem.readsConceptionLackedMany"),
                        )
                        .param("place", place.clone())
                        .param("placeholder", format!("{{{name}}}"))
                        .param("kinds", missing.join(", ")),
                    );
                }
                if let Some(board) = name.strip_prefix("starred:")
                    && self.board(board.trim()).is_none()
                {
                    problems.push(
                        Reason::of("problem.readsStarredNoBoard")
                            .param("place", place.clone())
                            .param("placeholder", format!("{{{name}}}"))
                            .param("board", board.trim()),
                    );
                }
            }
            // An action about a release writes what it goes out under, and is
            // offered only where there is a release: on a kind that goes out.
            if prompt.scope() == Scope::Release {
                if prompt.produces() != Produces::Release {
                    problems.push(
                        Reason::of("problem.releaseMustProduce").param("place", place.clone()),
                    );
                }
                let missing = lacking(&|kind: &WorkKind| kind.has_doors());
                if !missing.is_empty() {
                    problems.push(
                        by_count(
                            missing.len(),
                            Reason::of("problem.releaseLackedOne"),
                            Reason::of("problem.releaseLackedMany"),
                        )
                        .param("place", place.clone())
                        .param("kinds", missing.join(", ")),
                    );
                }
            }
            // `{release}` and `{releases}` are filled only for an action
            // about a release; anywhere else they would be sent as written.
            if prompt.scope() != Scope::Release
                && let Some(name) = placeholders
                    .iter()
                    .find(|name| *name == "release" || *name == "releases")
            {
                problems.push(
                    Reason::of("problem.releaseNotAbout")
                        .param("place", place.clone())
                        .param("placeholder", format!("{{{name}}}")),
                );
            }
            // An action about a cover's board proposes ideas for it, and is
            // offered only where there is a board: on a kind with a cover.
            if prompt.scope() == Scope::Cover {
                if prompt.produces() != Produces::CoverIdeas {
                    problems
                        .push(Reason::of("problem.coverMustProduce").param("place", place.clone()));
                }
                let missing = lacking(&|kind: &WorkKind| kind.cover);
                if !missing.is_empty() {
                    problems.push(
                        by_count(
                            missing.len(),
                            Reason::of("problem.coverLackedOne"),
                            Reason::of("problem.coverLackedMany"),
                        )
                        .param("place", place.clone())
                        .param("kinds", missing.join(", ")),
                    );
                }
            }
            // `{ideas}` is filled only for an action about a cover, and
            // `{choices}` for one about a cover or an experiment's board;
            // anywhere else they would be sent as written.
            if prompt.scope() != Scope::Cover
                && let Some(name) = placeholders.iter().find(|name| {
                    *name == "ideas" || (*name == "choices" && prompt.scope() != Scope::Lab)
                })
            {
                problems.push(
                    Reason::of("problem.coverNotAbout")
                        .param("place", place.clone())
                        .param("placeholder", format!("{{{name}}}")),
                );
            }
            // An action about an experiment's board proposes trials for it,
            // reads the board, and is offered only where there is one: on a
            // kind that is a lab.
            let reads_trials = placeholders.iter().any(|name| name == "trials");
            if prompt.scope() == Scope::Lab {
                if prompt.produces() != Produces::Trials {
                    problems
                        .push(Reason::of("problem.labMustProduce").param("place", place.clone()));
                }
                if !reads_trials {
                    problems
                        .push(Reason::of("problem.labNeverReads").param("place", place.clone()));
                }
                let missing = lacking(&|kind: &WorkKind| kind.lab.is_some());
                if !missing.is_empty() {
                    problems.push(
                        by_count(
                            missing.len(),
                            Reason::of("problem.labLackedOne"),
                            Reason::of("problem.labLackedMany"),
                        )
                        .param("place", place.clone())
                        .param("kinds", missing.join(", ")),
                    );
                }
            } else if reads_trials {
                problems
                    .push(Reason::of("problem.trialsNotAboutLab").param("place", place.clone()));
            }
            // An action about a card gathers facts, describes it or sums it
            // up; anything else it answered would have nowhere to go.
            if prompt.scope() == Scope::Canon
                && !matches!(
                    prompt.produces(),
                    Produces::Canon | Produces::CardPrompt | Produces::CardNote
                )
            {
                problems.push(Reason::of("problem.canonMustProduce").param("place", place.clone()));
            }
            // An action about a selection reads it; one that does not would
            // send the same prompt whatever was selected.
            let reads_selection = placeholders.iter().any(|name| name == "selection");
            if prompt.scope() == Scope::Selection && !reads_selection {
                problems
                    .push(Reason::of("problem.selectionNeverRead").param("place", place.clone()));
            }
            if prompt.scope() != Scope::Selection && reads_selection {
                problems
                    .push(Reason::of("problem.selectionNotAbout").param("place", place.clone()));
            }
            // An action about a style brick describes it; an answer of any
            // other shape has nowhere to go - the rule an action about a
            // comment follows below.
            if prompt.scope() == Scope::Style && prompt.produces() != Produces::Description {
                problems.push(Reason::of("problem.styleMustProduce").param("place", place.clone()));
            }
            // An action about phrases proposes bricks for them, and reads
            // them; `{phrases}` and `{types}` are filled for it alone.
            let reads_phrases = placeholders.iter().any(|name| name == "phrases");
            if prompt.scope() == Scope::Phrases {
                if prompt.produces() != Produces::Bricks {
                    problems.push(
                        Reason::of("problem.phrasesMustProduce").param("place", place.clone()),
                    );
                }
                if !reads_phrases {
                    problems
                        .push(Reason::of("problem.phrasesNeverRead").param("place", place.clone()));
                }
            } else if let Some(name) = placeholders
                .iter()
                .find(|name| *name == "phrases" || *name == "types")
            {
                problems.push(
                    Reason::of("problem.phrasesNotAbout")
                        .param("place", place.clone())
                        .param("placeholder", format!("{{{name}}}")),
                );
            }
            // An action about a comment either reads one off a screenshot or
            // drafts its reply; an answer of any other shape has nowhere to
            // go, and a button whose answer cannot be kept is a dead end.
            if prompt.scope() == Scope::Comment
                && !matches!(prompt.produces(), Produces::Comment | Produces::Reply)
            {
                problems
                    .push(Reason::of("problem.commentMustProduce").param("place", place.clone()));
            }
        }
    }
}
