// The shapes that cross `invoke`, generated from the Rust types in
// src-tauri/src/** by ts-rs (v0.83, ADR 0003). `src-tauri/tests/bindings.rs`
// fails when a generated file no longer matches what the backend actually
// sends and accepts, so this file no longer defines those shapes — it only
// re-exports them, under the names the rest of the window already imports,
// and keeps what is genuinely frontend-only: a derived union, a literal type
// for a field Rust deliberately widened to `string` for its own forward
// compatibility (see the Rust doc comments on `OverviewConfig`, `StyleBrick`,
// `Comment`).
//
// To pick up a change on the Rust side: `cd src-tauri && KILNA_BLESS=1 cargo
// test --test bindings`, then fix whatever `pnpm exec tsc` finds.

import type { JsonValue } from './generated/serde_json/JsonValue'
import type { Proposal } from './generated/Proposal'

// Vocabulary: the words a craft is judged and shipped by.
export type { Label } from './generated/Label'
export type { AxisKind } from './generated/AxisKind'
export type { AxisOption } from './generated/AxisOption'
export type { Axis } from './generated/Axis'
export type { AxisMark } from './generated/AxisMark'
export type { Tier } from './generated/Tier'
export type { Kind } from './generated/Kind'
export type { Derive } from './generated/Derive'
export type { MarkColour } from './generated/MarkColour'
export type { Status } from './generated/Status'
export type { Mark } from './generated/Mark'
export type { VersionRole } from './generated/VersionRole'
export type { MetaField } from './generated/MetaField'
export type { PromptTemplate } from './generated/PromptTemplate'
export type { ReleaseKind } from './generated/ReleaseKind'
export type { ReleaseFieldType } from './generated/ReleaseFieldType'
export type { ReleaseField } from './generated/ReleaseField'
export type { Rhythm } from './generated/Rhythm'
export type { SceneBlock } from './generated/SceneBlock'
export type { StyleType } from './generated/StyleType'
export type { StyleForm } from './generated/StyleForm'
export type { StyleFamily } from './generated/StyleFamily'
export type { StyleOrigin } from './generated/StyleOrigin'
export type { WorkKind } from './generated/WorkKind'
export type { Stage } from './generated/Stage'
export type { WidgetPlacement } from './generated/WidgetPlacement'
export type { OverviewConfig } from './generated/OverviewConfig'
export type { ProfileConfig } from './generated/ProfileConfig'

// `layout` and `size` are plain `string` on the Rust side on purpose — a
// document written by a later build, naming a layout or a size this one does
// not know, must still load (see `profile::config::OverviewConfig`). These
// stay narrow here for the picker that offers only the ones this build draws.
export type OverviewLayout = 'grid' | 'lead' | 'bands' | 'mosaic' | 'sheet'
export type WidgetSize = 's' | 'm' | 'l'

// Workspace and profiles.
export type { Profile } from './generated/Profile'
export type { Workspace } from './generated/Workspace'

/** Free-form fields on a row: a work's `meta`, a release's `meta`. */
export type Meta = Record<string, JsonValue>

// Works.
export type { Work } from './generated/Work'
export type { NewWork } from './generated/NewWork'
export type { WorkPatch } from './generated/WorkPatch'
export type { WorkFilter } from './generated/WorkFilter'
export type { Change as StatusChange } from './generated/Change'
export type { Discarded } from './generated/Discarded'
export type { BulkOutcome } from './generated/BulkOutcome'
export type { ScoredWork } from './generated/ScoredWork'
export type { Counts as CardCounts } from './generated/Counts'
export type { Made } from './generated/Made'
export type { Frame } from './generated/Frame'
export type { FramePrompts } from './generated/FramePrompts'
export type { FrameView } from './generated/FrameView'

// A publication's cover: the concept, its built frame, the views drawn from it.
export type { Cover } from './generated/Cover'
export type { CoverHero } from './generated/CoverHero'
export type { CoverBricks } from './generated/CoverBricks'
export type { CoverAccent } from './generated/CoverAccent'
export type { CoverLettering } from './generated/CoverLettering'
export type { CoverMark } from './generated/CoverMark'
export type { CoverMarkPlace } from './generated/CoverMarkPlace'
export type { CoverMarkWay } from './generated/CoverMarkWay'
export type { CoverSent } from './generated/CoverSent'
export type { Framing } from './generated/Framing'
export type { CoverLayout } from './generated/CoverLayout'
export type { CoverColumn } from './generated/CoverColumn'
export type { CoverRow } from './generated/CoverRow'
export type { CoverSize } from './generated/CoverSize'
export type { CoverCrop } from './generated/CoverCrop'
export type { CoverPlace } from './generated/CoverPlace'
export type { CoverCorner } from './generated/CoverCorner'
export type { Scheme } from './generated/Scheme'
export type { SchemeRect } from './generated/SchemeRect'
export type { SchemePaint } from './generated/SchemePaint'
export type { SchemeShape } from './generated/SchemeShape'
export type { SchemeColours } from './generated/SchemeColours'
export type { CoverPrompts } from './generated/CoverPrompts'
export type { CoverView } from './generated/CoverView'
export type { CoverFormat } from './generated/CoverFormat'
export type { CoverLayoutOption } from './generated/CoverLayoutOption'
export type { CoverDetail } from './generated/CoverDetail'
export type { CoverMarkOption } from './generated/CoverMarkOption'
export type { CoverPaletteColour } from './generated/CoverPaletteColour'
export type { CoverSlot } from './generated/CoverSlot'
export type { CoverHeroState } from './generated/CoverHeroState'
export type { CoverReference } from './generated/CoverReference'
export type { CoverProblem } from './generated/CoverProblem'
export type { SceneFrameView } from './generated/SceneFrameView'
// The board of ideas for a publication's cover (v0.89, ADR 0050).
export type { CoverBoard } from './generated/CoverBoard'
export type { CoverIdea } from './generated/CoverIdea'
export type { IdeaCard } from './generated/IdeaCard'
export type { IdeaLook } from './generated/IdeaLook'
export type { IdeaBrick } from './generated/IdeaBrick'
export type { IdeaSource } from './generated/IdeaSource'
export type { IdeaVerdict } from './generated/IdeaVerdict'
export type { IdeaRequest } from './generated/IdeaRequest'
export type { IdeaDropped } from './generated/IdeaDropped'
export type { PackagedIdea } from './generated/PackagedIdea'
export type { SiblingCover } from './generated/SiblingCover'
export type { Publication } from './generated/Publication'
export type { Publications } from './generated/Publications'
export type { Basis as PublicationBasis } from './generated/Basis'
export type { Cloned } from './generated/Cloned'

// Versions.
export type { VersionSummary } from './generated/VersionSummary'
export type { Version } from './generated/Version'
export type { NewVersion } from './generated/NewVersion'

// Notes.
export type { Note } from './generated/Note'
export type { NewNote } from './generated/NewNote'
export type { NotePatch } from './generated/NotePatch'
export type { NoteFilter } from './generated/NoteFilter'
export type { Promotion } from './generated/Promotion'
export type { Promoted } from './generated/Promoted'
export type { ResolvedLink } from './generated/ResolvedLink'
export type { NoteState } from './generated/NoteState'

// The register of repeats, and a text checked against it (ADR 0044).
export type { Term } from './generated/Term'
export type { NewTerm } from './generated/NewTerm'
export type { TermPatch } from './generated/TermPatch'
export type { TermKind } from './generated/TermKind'
export type { Strictness } from './generated/Strictness'
export type { RegisterEntry } from './generated/RegisterEntry'
export type { TermUse } from './generated/TermUse'
export type { TextCheck } from './generated/TextCheck'
export type { TextMark } from './generated/TextMark'
export type { TermHit } from './generated/TermHit'
export type { RepeatGroup } from './generated/RepeatGroup'

// The canon: cards (notes of a kind with sections), their facts and
// relations (ADR 0043).
export type { NoteKind } from './generated/NoteKind'
export type { CanonSection } from './generated/CanonSection'
export type { SectionShape } from './generated/SectionShape'
export type { Lens } from './generated/Lens'
export type { Layer } from './generated/Layer'
export type { FactStatus } from './generated/FactStatus'
export type { Fact } from './generated/Fact'
export type { NewFact } from './generated/NewFact'
export type { FactPatch } from './generated/FactPatch'
export type { Source as FactSource } from './generated/Source'
export type { SourceKind as FactSourceKind } from './generated/SourceKind'
export type { When as WorldTime } from './generated/When'
export type { CanonLink } from './generated/CanonLink'
export type { NewCanonLink } from './generated/NewCanonLink'
export type { CanonLinkPatch } from './generated/CanonLinkPatch'
export type { CardFilter } from './generated/CardFilter'
export type { CardSummary } from './generated/CardSummary'
export type { CardView } from './generated/CardView'
export type { ReadFact } from './generated/ReadFact'
export type { Relation } from './generated/Relation'
export type { Appearance } from './generated/Appearance'
export type { Dated } from './generated/Dated'
export type { Package as CanonPackage } from './generated/Package'
export type { ProposedCard } from './generated/ProposedCard'
export type { ProposedFact } from './generated/ProposedFact'
export type { ProposedLink } from './generated/ProposedLink'
export type { FactChange } from './generated/FactChange'
export type { Contradiction } from './generated/Contradiction'
export type { FactReview } from './generated/FactReview'
export type { CanonProposal } from './generated/CanonProposal'

// Comments.
export type { Comment } from './generated/Comment'
export type { NewComment } from './generated/NewComment'
export type { CommentPatch } from './generated/CommentPatch'
export type { CommentFilter } from './generated/CommentFilter'

// `state` is plain `string` on the Rust side (`comment::Comment`); kept
// narrow here for the screen's own switch over the states it draws.
export type CommentState = 'open' | 'posted' | 'archived'

// The style dictionary.
export type { StyleBrick } from './generated/StyleBrick'
export type { NewStyleBrick } from './generated/NewStyleBrick'
export type { StyleBrickPatch } from './generated/StyleBrickPatch'
export type { StyleBrickFilter } from './generated/StyleBrickFilter'

// `status` is plain `string` on the Rust side (`style_brick::StyleBrick`);
// kept narrow here the same way `CommentState` is.
export type StyleBrickStatus = 'draft' | 'ready' | 'dropped'

// The focus board.
export type { Dismissal } from './generated/Dismissal'
export type { DismissalKey } from './generated/DismissalKey'
export type { FocusNote } from './generated/FocusNote'
export type { NewFocusNote } from './generated/NewFocusNote'
export type { FocusNotePatch } from './generated/FocusNotePatch'

// Scores.
export type { Score } from './generated/Score'
export type { NewScore } from './generated/NewScore'
export type { KindVerdict } from './generated/KindVerdict'

// Releases and the calendar.
export type { Release } from './generated/Release'
export type { NewRelease } from './generated/NewRelease'
export type { ReleasePatch } from './generated/ReleasePatch'
export type { RoleMark } from './generated/RoleMark'
export type { Readiness } from './generated/Readiness'
export type { ScheduledRelease } from './generated/ScheduledRelease'
export type { Verdict as SlotVerdict } from './generated/Verdict'
export type { SlotPreview } from './generated/SlotPreview'
export type { Scheduling } from './generated/Scheduling'
export type { Placement } from './generated/Placement'
export type { Field as ReleaseFieldValue } from './generated/Field'
export type { ReleaseProposal } from './generated/ReleaseProposal'
export type { ProposedField } from './generated/ProposedField'
export type { Refusal as ReleaseFieldRefusal } from './generated/Refusal'
export type { Generated as GeneratedFields } from './generated/Generated'
export type { BatchRefusal as BatchFieldRefusal } from './generated/BatchRefusal'
export type { GeneratedBatch } from './generated/GeneratedBatch'

// Collections.
export type { Collection } from './generated/Collection'
export type { NewCollection } from './generated/NewCollection'
export type { CollectionPatch } from './generated/CollectionPatch'

// Links between works.
export type { Link } from './generated/Link'
export type { Derived } from './generated/Derived'
export type { Links } from './generated/Links'
export type { NewLink } from './generated/NewLink'

// The storyboard.
export type { Scene } from './generated/Scene'
export type { NewScene } from './generated/NewScene'
export type { ScenePatch } from './generated/ScenePatch'
export type { SceneNote } from './generated/SceneNote'
export type { SceneFrame } from './generated/SceneFrame'

// Cuts (a short's stretches).
export type { Cut } from './generated/Cut'
export type { NewCut } from './generated/NewCut'
export type { CutPatch } from './generated/CutPatch'
export type { Shot } from './generated/Shot'

// Files.
export type { Asset } from './generated/Asset'
export type { NewAsset } from './generated/NewAsset'

// Undo and the trash.
export type { Undoable } from './generated/Undoable'
export type { Entity as DeletedEntity } from './generated/Entity'
export type { Deletion } from './generated/Deletion'

// The journal.
export type { Level as JournalLevel } from './generated/Level'
export type { Entry as JournalEntry } from './generated/Entry'

// Search.
export type { HitKind } from './generated/HitKind'
export type { Hit } from './generated/Hit'

// The assistant: chats, runs, proposals.
export type { Availability } from './generated/Availability'
export type { Chat } from './generated/Chat'
export type { ChatSummary } from './generated/ChatSummary'
export type { NewChat } from './generated/NewChat'
export type { Message } from './generated/Message'
export type { Transcript } from './generated/Transcript'
export type { RunState } from './generated/RunState'
export type { Run } from './generated/Run'
export type { Event as RunEvent } from './generated/Event'
export type { Emission as RunEmission } from './generated/Emission'
export type { Proposal } from './generated/Proposal'
export type { BoardChange } from './generated/BoardChange'
export type { Marks } from './generated/Marks'
export type { PackagedVersion } from './generated/PackagedVersion'
export type { PackagedNote } from './generated/PackagedNote'
export type { PackagedScene } from './generated/PackagedScene'
export type { PackagedRelease } from './generated/PackagedRelease'
export type { Overrides as ProposalOverrides } from './generated/Overrides'
export type { Outcome as Applied } from './generated/Outcome'
export type { Pending as PendingProposal } from './generated/Pending'
export type { CommentProposal as PendingCommentProposal } from './generated/CommentProposal'
export type { Composed as ComposedTask } from './generated/Composed'
export type { TaskAbout } from './generated/TaskAbout'
export type { StartedTask } from './generated/StartedTask'
export type { StartedBatch } from './generated/StartedBatch'
export type { TaskQueue } from './generated/TaskQueue'
export type { Skipped } from './generated/Skipped'

/**
 * One variant of {@link Proposal}, narrowed by its `kind` tag. The backend
 * carries all eight as one tagged union (`assistant::proposal::Proposal`);
 * these are how a component that only ever handles one of them says so.
 */
export type ScoreProposal = Extract<Proposal, { kind: 'score' }>
export type VersionProposal = Extract<Proposal, { kind: 'version' }>
export type NoteProposal = Extract<Proposal, { kind: 'note' }>
export type WorkProposal = Extract<Proposal, { kind: 'work' }>
export type ScenesProposal = Extract<Proposal, { kind: 'scenes' }>
export type CommentProposal = Extract<Proposal, { kind: 'comment' }>
export type ReplyProposal = Extract<Proposal, { kind: 'reply' }>
export type DescriptionProposal = Extract<Proposal, { kind: 'description' }>
export type CanonProposalKind = Extract<Proposal, { kind: 'canon' }>
export type CardPromptProposal = Extract<Proposal, { kind: 'cardPrompt' }>
export type CoverIdeasProposal = Extract<Proposal, { kind: 'coverIdeas' }>

// Data in and out.
export type { ExportReport } from './generated/ExportReport'
export type { ImportReport } from './generated/ImportReport'
export type { PackageReport } from './generated/PackageReport'

// Plugins.
export type { Command as PluginCommand } from './generated/Command'
export type { Manifest as PluginManifest } from './generated/Manifest'
export type { Target } from './generated/Target'
export type { Plugin } from './generated/Plugin'
