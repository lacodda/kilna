// These mirror the Rust structs in src-tauri/src. Nothing enforces that they
// agree — see ADR 0003 — so a change on one side means a change here.

/**
 * A word of the craft's vocabulary, in the languages the profile carries it in.
 *
 * A profile's words are the author's data, not the interface's strings: they
 * are edited in Settings, they outlive an upgrade, and a shipped profile has
 * no business guessing which of them someone renamed. But a shipped profile
 * IS written by us, and writing it in English only meant a Russian window
 * read "Scored · Song" — the interface translated around a hole.
 *
 * So a label is either one string, as every profile written before this said
 * it, or a small map from locale to string. `labelOf` is the single place
 * that resolves one, which is why this could be done without touching the
 * hundred call sites that ask a vocabulary what it is called.
 */
export type Label = string | Record<string, string>

/** The shape of an answer along an axis: a number, yes/no, or one of a list. */
export type AxisKind = 'scale' | 'flag' | 'choice'

/** One answer a `choice` axis offers, worth `value` on the axis's scale. */
export interface AxisOption {
  key: string
  label: Label
  value: number
}

export interface Axis {
  key: string
  label: Label
  weight: number
  scale: number
  description?: Label
  /** Absent in a profile written before kinds existed: a scale. */
  kind?: AxisKind
  /** Only a `choice` axis has any. */
  options?: AxisOption[]
  /** What the landmark marks on this axis mean; not every mark has one. */
  rubric?: AxisMark[]
}

/** What one mark on an axis means. `at` is on the axis's scale, not the total. */
export interface AxisMark {
  at: number
  label: Label
}

export interface Tier {
  key: string
  label: Label
  min: number
}

export interface Kind {
  key: string
  label: Label
}

// What a status means to the automation, in descending finality. `manual` is
// never derived: it is a decision someone made, with no fact behind it.
export type Derive = 'manual' | 'draft' | 'scored' | 'scheduled' | 'released'

/** A palette role rather than a colour, so it reads in both themes. */
export type MarkColour = 'plain' | 'accent' | 'good' | 'warn' | 'bad' | 'info'

export interface Status extends Kind {
  derive: Derive
  /** The badge's emphasis; absent draws it in outline. The word is always there. */
  colour?: MarkColour | null
}

/** A flag the author raises by hand, beside the status the app derives. */
export interface Mark {
  key: string
  label: Label
  colour?: MarkColour
  /** A glyph beside the word, by name — see `lib/markIcon` for the list. */
  icon?: string
}

/** An independent body a work carries. */
export interface VersionRole extends Kind {
  /** The role this one discusses. A review is read beside what it reviews,
      not in its place; a role without this stands alone. */
  comments_on?: string
  /** How a body in this role reads. Absent, or anything but `markdown`, is
      plain: a monospace column, exactly as typed. */
  body?: 'plain' | 'markdown' | (string & {})
  /** Whether a body in this role is a time the WORK was written, and so
      belongs in the count the catalogue shows. A style prompt is written
      about the song rather than being a draft of it. Absent means "yes,
      unless it comments on something". */
  counts_as_version?: boolean
}

export interface MetaField {
  key: string
  label: Label
  type: 'text' | 'multiline' | 'number' | 'date' | 'boolean'
}

/** What an answer proposed, when its action asked for something applicable. */
export interface ScoreProposal {
  kind: 'score'
  /** Axis key to value, already checked against the profile. */
  axes: Record<string, number>
  note?: string
  /** Axes the answer named that the profile does not have. */
  unknown?: string[]
  /** Axes of the profile the answer skipped. */
  missing?: string[]
}

/** A version an agent proposed from outside the window; the text is the message body. */
export interface VersionProposal {
  kind: 'version'
  role: string
  label?: string
}

/** A note an agent proposed; the note is the message body. */
export interface NoteProposal {
  kind: 'note'
  title?: string
}

/** A version inside a package; the text travels with it. */
export interface PackagedVersion {
  role: string
  body: string
  label?: string
}

export interface PackagedNote {
  title?: string
  body: string
}

/** A scene inside a proposal: the fields of a row, checked against the kind's words. */
export interface PackagedScene {
  /** The number on the board; after the last when omitted on an added scene. */
  position?: number
  section?: string
  starts_at?: number
  ends_at?: number
  shot_type?: string
  description?: string
  /** Prompt blocks by the kind's `scene_blocks` key. */
  blocks?: Record<string, string>
}

/** A storyboard for the chat's work: scenes added after the last, or the board replaced. */
/** What a scenes proposal does to the board that is there. */
export type BoardChange = 'add' | 'replace' | 'revise'

export interface ScenesProposal {
  kind: 'scenes'
  scenes: PackagedScene[]
  /** `add` after the last (the default); `replace` the board — same numbers
      rewritten in place, the rest to the trash; `revise` only the numbered
      scenes, only in the fields given. */
  change?: BoardChange
  /** The v0.62 flag on a stored proposal; `change` supersedes it. */
  replace?: boolean
}

/** Marks along the axes, checked against the kind — the inside of a score proposal. */
export interface Marks {
  axes: Record<string, number>
  note?: string
  unknown?: string[]
  missing?: string[]
}

/** A whole work, or a package of changes to the chat's work, applied with one
    click. A chat on nothing receives new works — then `title` and `work_kind`
    say what it is; a chat on a work receives packages for it. */
export interface WorkProposal {
  kind: 'work'
  title?: string
  work_kind?: string
  /** Overview field key to value, already checked against the profile. */
  fields?: Record<string, unknown>
  unknown_fields?: string[]
  versions?: PackagedVersion[]
  score?: Marks
  notes?: PackagedNote[]
  /** The storyboard of a new video, or scenes added to an existing one's board. */
  scenes?: PackagedScene[]
  /** Releases to plan, with what each goes out as. */
  releases?: PackagedRelease[]
}

/** A release inside a package: the kind it ships as, when, and what it says
    about itself. */
export interface PackagedRelease {
  kind: string
  scheduled_at?: string | null
  /** Field key to value, already checked against the release kind. */
  fields?: Record<string, unknown>
  /** Field keys the package named that this kind of release does not have. */
  unknown_fields?: string[]
}

/** A comment read off a screenshot, waiting to be kept. The channel and the
 *  work are where the screenshot was pasted, never the answer's guess. */
export interface CommentProposal {
  kind: 'comment'
  channel: string
  work_id?: string
  author?: string
  body: string
  commented_on?: string
  /** What the picture says it was written under, when no work was given. */
  about?: string
}

/** A reply to one comment; the text is the message body. */
export interface ReplyProposal {
  kind: 'reply'
  comment_id: string
}

/** A style brick's description; the text is the message body. */
export interface DescriptionProposal {
  kind: 'description'
  style_id: string
}

export type Proposal =
  | ScoreProposal
  | VersionProposal
  | NoteProposal
  | WorkProposal
  | ScenesProposal
  | CommentProposal
  | ReplyProposal
  | DescriptionProposal

/** What applying a proposal made — stamped on the message as `meta.applied`. */
export interface Applied {
  message_id: string
  at: string
  work_id?: string
  /** The package created the work rather than adding to one. */
  created_work?: boolean
  versions?: string[]
  score?: string
  notes?: string[]
  /** The overview fields written, by key. */
  fields?: string[]
  /** Scenes written onto the board: created, or rewritten in place on a replaced board. */
  scenes?: string[]
  /** Scenes a replaced board sent to the trash, by trash entry. */
  removed_scenes?: string[]
  /** Releases planned by the package. */
  releases?: string[]
  /** The comment kept from a screenshot, or whose reply was written. */
  comment?: string
  /** The style brick whose description was written. */
  style_brick?: string
}

/** What a person may change about a proposed version on the way in. */
export interface ProposalOverrides {
  role?: string
  label?: string
  make_current?: boolean
  /** A comment read off a screenshot, as corrected before keeping. */
  comment?: NewComment
  /** A drafted reply, as edited before keeping. */
  reply?: string
}

export interface PromptTemplate {
  key: string
  label: Label
  template: string
  description?: Label
  /** Name of the glyph the button is drawn with, from the fixed set in
      `lib/actionIcon`. Absent, or a name this build does not know, draws the
      generic spark. */
  icon?: string | null
  /** What the action asks for beyond prose: `score`, or `version:<role>` —
      the whole answer kept as a version in that role. Absent means prose;
      a value this build does not know reads as prose too. */
  produces?: string
  /** How the action is done — the role the assistant takes, what it checks,
      the shape of the answer, what it must never say. Reaches the model as a
      system instruction, not as part of the message. See ADR 0021. */
  method?: string
  /** The kinds of work the action is for; absent or empty means every kind. */
  kinds?: string[]
  /** `scene` for an action started from a row of the board, `style` for one
      about a brick of the style dictionary; otherwise the work. */
  scope?: string
}

// What a release of this kind cannot ship without: version role keys. An empty
// list states no requirements, and readiness marks render as inapplicable.
export interface ReleaseKind extends Kind {
  requires: string[]
  /** Name of the glyph the calendar draws this kind with, from the fixed set
      in `lib/releaseIcon.ts`. Absent in a profile written before the field, and
      absent for a kind the owner invented; both fall back to a neutral mark. */
  icon?: string | null
  /** Axis weights that apply when a work is judged for this kind of release,
      keyed by axis key. An axis not named keeps its own weight. Absent or
      empty means one tier for every kind. */
  axis_weights?: Record<string, number>
  /** What a release of this kind says about itself. Absent or empty means it
      says nothing, and the tab shows no metadata for it. */
  fields?: ReleaseField[]
}

/** The shape of the box a release field is typed in. */
export type ReleaseFieldType = 'line' | 'text' | 'tags'

/** One thing written about a release: the box, and the template it starts
    from. The key is what the value is stored under in `release.meta`, so
    renaming the label never loses what was written. */
export interface ReleaseField extends Kind {
  type: ReleaseFieldType
  /** What the field is filled with when generated, in the prompt language
      (`{title}`, `{role:lyrics}`, `{scenes}`). Absent means it is only ever
      typed by hand. */
  template?: string | null
  /** A line under the box saying what goes in it. */
  hint?: Label | null
  /** How many characters the place this is going will accept. Counted beside
      the box, never enforced — kilna is not the authority on what a platform
      accepts this month. */
  limit?: number | null
}

// The pace releases go out at. `default_time` (HH:MM) is a hint shown beside
// the date — slots stay whole days, the contest is per day.
export interface Rhythm {
  every_days: number
  default_time: string | null
}

/** A kind of work with the vocabulary it is judged and shipped by (format 2).
    A list a kind leaves out is empty for works of that kind — a video with
    no axes is scored empty, not on the song's. */
/** One prompt block a scene carries: a still frame, an animation, a negative. */
export interface SceneBlock extends Kind {
  /** A line under the box saying what goes in it. */
  hint?: Label | null
}

/**
 * One type of style brick: an image style, a character, an environment.
 *
 * The `hint` says what to describe when a brick is of this type — the
 * difference between one rich dictionary and several flat ones. It reaches the
 * assistant when a brick is described and never a generator. See ADR 0031.
 */
export interface StyleType extends Kind {
  hint?: Label | null
  /** A glyph from the fixed set the window knows. */
  icon?: string | null
}

export interface WorkKind extends Kind {
  axes?: Axis[]
  tiers?: Tier[]
  version_roles?: VersionRole[]
  release_kinds?: ReleaseKind[]
  statuses?: Status[]
  /** Kinds of shot a scene can be; a kind naming none has no storyboard. */
  shot_types?: Kind[]
  /** The prompt blocks a scene carries, each edited and copied on its own. */
  scene_blocks?: SceneBlock[]
  /** The parts of the prompt a work's cover is drawn from — what to draw,
      what to keep out, what words go on it. A kind naming none has no cover
      prompt, and the tab that edits one does not appear. */
  cover_blocks?: SceneBlock[]
}

/** One stop on the way from an idea to a finished work. */
export interface Stage {
  key: string
  label: Label
  /** Where this stop sits, 0..=100 — what the dial draws. */
  percent: number
  colour?: string
}

export interface ProfileConfig {
  /** The shape of the document; 2 since v0.57. */
  format: number
  work_kinds: WorkKind[]
  collection_kinds: Kind[]
  work_meta_fields: MetaField[]
  // Absent in a profile written before marks existed.
  marks?: Mark[]
  /** The stops of the stage dial. Absent, or empty, means the craft names
      none and the line's own stops are used — see `lib/stages`. */
  stages?: Stage[]
  prompts: PromptTemplate[]
  // Absent in a profile written before the field existed: the auto-layout then
  // has nothing to pace by, and its button says so instead of guessing.
  rhythm?: Rhythm | null
  /** Which catalogue columns to show, by id, in order. Absent means the
      catalogue's own default. A craft reads down its own columns, which is
      why this is on the profile and not on the machine. */
  catalogue_columns?: string[] | null
  /** The columns shown while the catalogue is narrowed to one kind of work,
      by kind key; a kind without an entry reads down `catalogue_columns`. */
  catalogue_columns_by_kind?: Record<string, string[]> | null
  /** Kinds a note can take — a character, a place, a piece of lore. A scene
      points at notes of these kinds. Absent in a profile written before
      them; a note still takes any kind a person writes. */
  note_kinds?: Kind[]
  /** The types a style brick can be. On the profile rather than on a kind:
      the same character stands in the videos and in the shorts. Absent means
      the craft has no style dictionary and the screen does not appear. */
  style_types?: StyleType[]
}

export interface Profile {
  id: string
  key: string
  name: string
  description: string | null
  config: ProfileConfig
  is_active: boolean
  is_builtin: boolean
}

export interface Workspace {
  schema_version: number
  profile: Profile | null
  works: number
  releases: number
}

export type Meta = Record<string, unknown>

export interface Work {
  id: string
  profile_id: string
  collection_id: string | null
  kind: string
  title: string
  status: string
  // Set when a person chose the status by hand; while it holds a value the
  // automation leaves this work alone.
  status_pinned_at: string | null
  meta: Meta
  /** The author's own words for what this is, in the order they were added. */
  tags: string[]
  /** Keys into the profile's `marks`; one the profile no longer defines is
      kept on the work but not drawn. */
  marks: string[]
  current_version_id: string | null
  position: number
  /** The tier a person is holding the work at, overruling the score; null
      means the score speaks. The reason is on record beside it. */
  tier_pinned: string | null
  tier_pinned_at: string | null
  tier_pin_reason: string | null
  /** Set when marked to come back to. */
  bookmarked_at: string | null
  /** How finished the work is, 0..=100, as the author judges it. `null` means
      nobody has said yet — which is not a judgement of zero. */
  stage: number | null
  /** The prompt the cover picture is drawn from, by the kind's `cover_blocks`
      key. Empty for a craft whose covers are not written. */
  cover: Record<string, string>
  created_at: string
  updated_at: string
}

export interface NewWork {
  kind: string
  title: string
  status?: string | null
  collection_id?: string | null
  meta?: Meta | null
}

// A field left out is untouched; `null` inside a nullable field clears it.
export interface WorkPatch {
  title?: string
  status?: string
  kind?: string
  collection_id?: string | null
  meta?: Meta
  /** Replaces the list. The backend trims, drops blanks and deduplicates
      case-insensitively, so sending what the box holds is enough. */
  tags?: string[]
  /** `true` stamps the bookmark, `false` clears it. */
  bookmarked?: boolean
  /** How finished the work is, 0..=100; `null` takes it back to unjudged. */
  stage?: number | null
  /** Replaces the whole set, the way a scene's blocks do: the screen edits one
      block and sends them all, so an undo puts the set back. */
  cover?: Record<string, string>
  marks?: string[]
  current_version_id?: string | null
}

export interface WorkFilter {
  status?: string
  kind?: string
  collection_id?: string
  search?: string
}

/** A work this one was made from, as the card reads it. */
export interface Link {
  id: string
  /** The work that was made from the other. */
  work_id: string
  /** The work it was made from. */
  source_id: string
  role: string
  /** The source's current version when the link was made; null when it had
      none, or that version was deleted since. */
  source_version_id: string | null
  created_at: string
  source_title: string
  source_kind: string
  source_status: string
  source_current_version_id: string | null
  taken_revision: number | null
  current_revision: number | null
  /** The source has moved on since: another version is current, or the one
      taken was edited in place. A fact for the card to mark, not a verdict. */
  drifted: boolean
}

/** A work made from this one. */
export interface Derived {
  link_id: string
  work_id: string
  title: string
  kind: string
  status: string
  role: string
  created_at: string
}

export interface Links {
  sources: Link[]
  derived: Derived[]
}

export interface NewLink {
  work_id: string
  source_id: string
  /** `donor` when omitted. */
  role?: string | null
  /** The source's current version when omitted. */
  source_version_id?: string | null
}

/** One row of a work's storyboard. Owned by the work — see ADR 0020. */
export interface Scene {
  id: string
  profile_id: string
  work_id: string
  /** The scene's number on the board, from 1. */
  position: number
  /** The part of the text it plays against: intro, verse 1, chorus. */
  section: string | null
  /** Seconds from the start; null until the board is timed. */
  starts_at: number | null
  ends_at: number | null
  /** A key of the kind's `shot_types`; null when not yet decided. */
  shot_type: string | null
  description: string
  /** Prompt blocks by the kind's `scene_blocks` key. */
  blocks: Record<string, string>
  created_at: string
  updated_at: string
}

export interface NewScene {
  work_id: string
  /** After the last scene when omitted. */
  position?: number | null
  section?: string | null
  starts_at?: number | null
  ends_at?: number | null
  shot_type?: string | null
  description?: string | null
  blocks?: Record<string, string> | null
}

/** What an edit may change; `blocks` replaces the whole set. */
export interface ScenePatch {
  position?: number
  section?: string | null
  starts_at?: number | null
  ends_at?: number | null
  shot_type?: string | null
  description?: string
  blocks?: Record<string, string>
}

export interface Version {
  id: string
  work_id: string
  role: string
  revision: number
  label: string | null
  body: string
  meta: Meta
  /** The version this one was written from; null once that one is deleted. */
  parent_version_id: string | null
  created_at: string
}

export interface VersionSummary {
  id: string
  work_id: string
  role: string
  revision: number
  label: string | null
  length: number
  parent_version_id: string | null
  created_at: string
  is_current: boolean
  /** For a commentary: the version it was written about, when it was written
      about one. Paired by this first; by revision number only when absent. */
  about_version_id: string | null
}

export interface NewVersion {
  role: string
  body: string
  label?: string | null
  meta?: Meta | null
  make_current?: boolean
  /** The version this one was derived from: same work, same role. */
  parent_version_id?: string | null
}

export interface Note {
  id: string
  profile_id: string
  work_id: string | null
  kind: string
  title: string | null
  body: string
  tags: string[]
  created_at: string
  updated_at: string
}

export interface NewNote {
  body: string
  kind?: string | null
  title?: string | null
  work_id?: string | null
  tags?: string[]
}

export interface NotePatch {
  title?: string | null
  body?: string
  kind?: string
  tags?: string[]
  work_id?: string | null
}

export interface NoteFilter {
  work_id?: string
  kind?: string
  tag?: string
  search?: string
}

/** A complaint the person has heard and put away. */
export interface Dismissal {
  kind: string
  work_id: string
  complaint: string
  dismissed_at: string
}

/** What identifies a dismissal: the kind, the work, and what was said. */
export interface DismissalKey {
  kind: string
  work_id: string
  complaint: string
}

/** A line the person put on the focus board themselves. */
export interface FocusNote {
  id: string
  profile_id: string
  body: string
  work_id: string | null
  position: number
  pinned_at: string | null
  /** The day it is meant to be done by (YYYY-MM-DD); null when unset. */
  due_on: string | null
  created_at: string
  updated_at: string
}

export interface NewFocusNote {
  body: string
  work_id?: string | null
  due_on?: string | null
}

export interface FocusNotePatch {
  body?: string
  work_id?: string | null
  pinned?: boolean
  due_on?: string | null
}

export interface Score {
  id: string
  work_id: string
  version_id: string | null
  axes: Record<string, number>
  total: number
  tier: string | null
  note: string | null
  /** Who judged; null is the author. */
  rater: string | null
  scored_at: string
  revision: number | null
}

export interface NewScore {
  /** A number on a scale axis, a boolean on a flag, an option key on a choice. */
  axes: Record<string, number | boolean | string>
  version_id?: string | null
  note?: string | null
  rater?: string | null
}

export interface ScoredWork {
  work_id: string
  title: string
  kind: string
  status: string
  /** True when `tier` is a person's pin, not the score's verdict. */
  tier_pinned: boolean
  total: number | null
  tier: string | null
  scored_at: string | null
  stale: boolean
  /** How many releases of this work have gone out. */
  released: number
  /** How many hold a slot in the calendar. */
  scheduled: number
  /** When the work was last touched — what tells a live draft from a stalled one. */
  updated_at: string
  /** When the work was first written down. */
  created_at: string
  /** The collection this belongs to; null for most works until an album gathers them. */
  collection_id: string | null
  /** The author's own words for what this is. */
  tags: string[]
  /** Keys into the profile's marks, drawn only while the profile defines them. */
  marks: string[]
  /** Set when the work is starred: marked to come back to. */
  bookmarked_at: string | null
  /** How many versions the work holds, across every role. */
  version_count: number
  /** How finished the work is, 0..=100; `null` while nobody has said. */
  stage: number | null
}

/** What a bulk edit did. The catalogue reloads afterwards; these are for the toast. */
export interface BulkOutcome {
  changed: number
  skipped: number
}

export interface Release {
  id: string
  work_id: string
  kind: string
  status: string
  title: string | null
  scheduled_at: string | null
  released_at: string | null
  url: string | null
  /** Set when a person settled this date; a pinned slot is never contested. */
  slot_pinned_at: string | null
  /** When in the day it goes out (HH:MM); the slot itself stays a date. */
  scheduled_time: string | null
  /** Whose day: an IANA zone name such as `Europe/Lisbon`. */
  time_zone: string | null
  meta: Meta
  created_at: string
  updated_at: string
}

// Three states, not two: a role the kind does not require is neither there nor
// missing.
export interface RoleMark {
  role: string
  present: boolean | null
}

// How far a release is from shippable, judged on the backend so the chip and
// the journal warning can never disagree.
export interface Readiness {
  roles: RoleMark[]
  scored: boolean
  ready: boolean
}

// The backend flattens the release into this, so the fields sit side by side.
export interface ScheduledRelease extends Release {
  work_title: string
  /** The kind of the work — a song, a video — for a chip to say what is
      going out, not only what kind of release it is. */
  work_kind: string
  total: number | null
  tier: string | null
  readiness: Readiness
  /** How finished the work itself is, 0..=100; `null` while nobody has said.
      Not `readiness`, which asks whether the release could go out. */
  work_stage: number | null
}

export interface NewRelease {
  work_id: string
  kind: string
  title?: string | null
  scheduled_at?: string | null
  meta?: Meta | null
  scheduled_time?: string | null
  time_zone?: string | null
}

export interface Scheduling {
  release: Release
}

// The dry run of a claim: the same verdict `schedule_release` would act on,
// shown before the drop instead of announced after it. Mirrors
// `release::Verdict`; the backend's `window_unions` test holds the two
// together - this said `displaces` and `held` for three weeks after the
// backend stopped sending them.
export type SlotVerdict = 'empty' | 'taken' | 'pinned'

export interface SlotPreview {
  verdict: SlotVerdict
  holder_title: string | null
}

export interface Collection {
  id: string
  profile_id: string
  kind: string
  title: string
  description: string | null
  position: number
  meta: Meta
  /** How many works it is meant to hold when finished; null when unset. */
  target_size: number | null
  /** The day it is meant to be done by (YYYY-MM-DD); null when unset. */
  due_on: string | null
  created_at: string
  updated_at: string
  works: number
}

/** What pressing undo would take back. */
export interface Undoable {
  /** The operation that would be reversed; sent back so a stale offer is refused. */
  operationId: string
  /** The i18n key naming it, e.g. `undo.work.update`. */
  action: string
  /** Values the sentence interpolates. */
  params: Record<string, unknown>
}

/**
 * One stretch of a longer work a short is spliced from.
 *
 * A short cut from a finished video and one shot for itself are the same kind
 * of work; what tells them apart is whether there are any of these. The
 * seconds are on the source's own timeline, and a short holds a list of them
 * because it is routinely spliced from more than one stretch.
 */
export interface Cut {
  id: string
  profile_id: string
  /** The short this stretch is part of. */
  work_id: string
  /** The work it is taken out of. */
  source_id: string
  /** Seconds on the source's timeline, from its start. */
  starts_at: number
  ends_at: number
  /** Its place in the splice, from 1. */
  position: number
  label: string | null
  created_at: string
  updated_at: string
  /** The source's title, so the track draws without a second read. */
  source_title: string
  /** The source's length, which the track is drawn against; `null` when the
      donor has no duration yet, and the track then has no scale. */
  source_duration: number | null
}

export interface NewCut {
  work_id: string
  source_id: string
  starts_at: number
  ends_at: number
  /** At the end of the splice when omitted. */
  position?: number | null
  label?: string | null
}

/** What an edit may change. Dragging one end sends one number. */
export interface CutPatch {
  starts_at?: number
  ends_at?: number
  position?: number
  label?: string | null
}

/** One line of what a cutter is told to do: this stretch, of this file. */
export interface Shot {
  cut_id: string
  position: number
  starts_at: number
  ends_at: number
  source_id: string
  source_title: string
  /** The donor's video on disk; `null` when it has not been rendered yet. */
  path: string | null
}

/** A file attached to a work or a release: a cover, a reference. */
export interface Asset {
  id: string
  profile_id: string
  work_id: string | null
  release_id: string | null
  /** What it is for: `cover`, or a plain attachment. */
  kind: string
  /** Where the bytes are, inside the workspace's own files directory. */
  path: string
  label: string | null
  /** The name the file arrived under — what the world outside calls it. */
  original_name: string | null
  /** The style brick it is a reference for, when it is one. */
  style_brick_id: string | null
  created_at: string
}

/** What to attach, and to what. */
export interface NewAsset {
  work_id?: string
  release_id?: string
  /** The style brick this is a reference for. */
  style_brick_id?: string
  /** `cover`, or nothing for a plain attachment. */
  kind?: string
  label?: string
}

/** A note a scene is about: who is in it, where it happens. */
export interface SceneNote {
  id: string
  scene_id: string
  note_id: string
  /** The note's own kind — `character`, `location` — as the profile names it. */
  note_kind: string
  note_title: string | null
  created_at: string
}

/** A picture drawn for a scene: one of the four a prompt came back with. */
export interface SceneFrame {
  id: string
  scene_id: string
  asset_id: string
  /** What this is: `frame` for the still, `video` for the clip animated
   * from one. */
  kind: string
  /** Its place among the scene's material of that kind, from 1. */
  position: number
  /** Whether the scene's own. At most one per kind per scene, so a scene may
   * have both its still and its clip chosen. */
  is_selected: boolean
  /** Where the bytes are, inside the workspace's own files directory. */
  path: string
  /** The name the file arrived under — what a generator wrote in it. */
  original_name: string | null
  created_at: string
}

/** What a clone came out as, for the sentence the window says afterwards. */
export interface Cloned {
  work: Work
  scenes: number
  materials: number
}

// A status the automation would change, or did.
export interface StatusChange {
  work_id: string
  title: string
  from: string
  to: string
}

/** Where a comment stands. A drafted reply is read off `reply`, not stored. */
export type CommentState = 'open' | 'posted' | 'archived'

/** What the audience said, and the reply to it. */
export interface Comment {
  id: string
  profile_id: string
  /** Where it was written, as the person names it. */
  channel: string
  work_id: string | null
  author: string | null
  body: string
  reply: string | null
  state: CommentState
  /** The day the viewer wrote it, `YYYY-MM-DD`. */
  commented_on: string | null
  created_at: string
  updated_at: string
}

export interface NewComment {
  channel: string
  body: string
  work_id?: string | null
  author?: string | null
  reply?: string | null
  commented_on?: string | null
}

export interface CommentPatch {
  channel?: string
  work_id?: string | null
  author?: string | null
  body?: string
  reply?: string | null
  state?: CommentState
  commented_on?: string | null
}

export interface CommentFilter {
  work_id?: string
  channel?: string
  /** One state; absent is every state but archived. */
  state?: CommentState
  search?: string
}

/** A comment read off a screenshot, or a drafted reply, waiting to be kept. */
export interface PendingCommentProposal {
  message_id: string
  chat_id: string
  /** The answer: the reply itself, or what the comment was read from. */
  body: string
  proposal: CommentProposal | ReplyProposal
  created_at: string
}

/** What a note becomes when it grows up: a work of this kind, by this name. */
export interface Promotion {
  kind: string
  title: string
}

export interface Promoted {
  work_id: string
  version_id: string
  deletion_id: string
}

/** What a `[[work:id]]` or `[[version:id]]` link points at. A link to
    something deleted is simply absent from the answer. */
export interface ResolvedLink {
  id: string
  target: 'work' | 'version'
  title: string
  /** The work whose card opens. The work's own id for a work. */
  workId: string
}

/**
 * A brick of the workspace's style dictionary: a part a picture prompt is
 * built from, under a type the profile names. See ADR 0031.
 */
export interface StyleBrick {
  id: string
  profile_id: string
  /** A key of the profile's `style_types`. */
  type_key: string
  name: string
  /** The text that goes into a prompt verbatim; null while it is a draft. */
  description: string | null
  /** The author's steer for whoever describes it — never part of a prompt. */
  hint: string | null
  status: StyleBrickStatus
  created_at: string
  updated_at: string
  reference_count: number
}

/** Draft until it carries a description, ready once it does, dropped when
 * retired. Only ready bricks are offered for building a prompt. */
export type StyleBrickStatus = 'draft' | 'ready' | 'dropped'

export interface NewStyleBrick {
  type_key: string
  name: string
  description?: string | null
  hint?: string | null
}

export interface StyleBrickPatch {
  type_key?: string
  name?: string
  description?: string | null
  hint?: string | null
  status?: StyleBrickStatus
}

export interface StyleBrickFilter {
  type_key?: string | null
  ready_only?: boolean
  query?: string | null
}

/** What one release kind makes of a work's latest answers. */
export interface KindVerdict {
  kind: string
  total: number
  tier: string | null
  /** False when the kind weighs the axes exactly as the profile does. */
  reweighed: boolean
}

// A field left out is untouched; `null` inside a nullable field clears it.
export interface ReleasePatch {
  kind?: string
  title?: string | null
  scheduled_at?: string | null
  url?: string | null
  meta?: Meta
  scheduled_time?: string | null
  time_zone?: string | null
}

// One line of an auto-layout plan: this release lands on this day. What
// `planLayout` returns is exactly what `applyLayout` takes back — the preview
// is the contract, not a sketch.
export interface Placement {
  release_id: string
  date: string
}

/** A release field as a screen needs it: what it is, what is written in it,
    and whether the profile could write it. */
export interface ReleaseFieldValue {
  key: string
  label: Label
  type: ReleaseFieldType
  value: string
  hint?: Label | null
  limit?: number | null
  /** Whether the profile can fill this field on its own. */
  has_template: boolean
}

/** A field the profile could not fill, in the renderer's own words. */
export interface ReleaseFieldRefusal {
  key: string
  label: Label
  reason: string
}

/** What a generation produced: the values, and what it could not write. */
export interface GeneratedFields {
  values: Meta
  refused: ReleaseFieldRefusal[]
}

/** A field a batch could not fill, named by the work it is on. */
export interface BatchFieldRefusal {
  releaseId: string
  workTitle: string
  label: Label
  reason: string
}

/** What a batch generation did, and to what. */
export interface GeneratedBatch {
  filled: number
  skipped: number
  refused: BatchFieldRefusal[]
}

/** What a trashed entry was. Mirrors the backend's `trash::Entity`; the
    backend's `the_window_knows_every_kind_the_trash_holds` test holds the
    two lists together, because a kind added on one side only once showed up
    in the trash with no word for it. */
export type DeletedEntity =
  | 'work'
  | 'version'
  | 'score'
  | 'release'
  | 'note'
  | 'collection'
  | 'scene'
  | 'cut'
  | 'comment'
  | 'style'

export interface Deletion {
  id: string
  entity: DeletedEntity
  entity_id: string
  label: string
  origin: string | null
  /** The work it is or belonged to, for its cover; null when it hangs off none. */
  work_id: string | null
  reason: string
  deleted_at: string
  /** False while what it belonged to is itself in the trash. */
  restorable: boolean
}

/** How loudly an entry asks to be noticed. Only `warn` is counted unread. */
export type JournalLevel = 'info' | 'warn'

/**
 * One thing that happened.
 *
 * `action` is an i18n key and `params` its values — never a finished sentence,
 * so history written in one language still reads in another.
 */
export interface JournalEntry {
  id: string
  action: string
  params: Record<string, string | number>
  level: JournalLevel
  entity: string | null
  entity_id: string | null
  /** How many times it happened, counting the first. */
  occurrences: number
  created_at: string
  read_at: string | null
}

/** What a search hit points at. Mirrors the backend's `search::Kind`. */
export type HitKind = 'work' | 'version' | 'note' | 'message' | 'comment'

export interface Hit {
  kind: HitKind
  /** The row that matched. A note opens on the notes screen by it. */
  entity_id: string
  /** The work it belongs to; null only for a note on nothing in particular. */
  work_id: string | null
  /** The work's title, empty without one. */
  work_title: string
  /** The hit's own line: a title, or the text it was found in. */
  title: string
  /** Where it came from — `lyrics · v2`, `note`, `assistant`. */
  detail: string
  rank: number
}

export interface Availability {
  available: boolean
  version: string | null
  reason: string | null
}

export interface Chat {
  id: string
  profile_id: string
  work_id: string | null
  title: string | null
  session_id: string | null
  /** Set while a background task in this chat is waiting on an answer. */
  waiting_since?: string
  created_at: string
  updated_at: string
}

/** A chat as the list draws it: named, priced, tied to its work. */
export interface ChatSummary {
  id: string
  work_id: string | null
  /** Title of the work the chat is about. */
  work_title?: string
  title?: string
  /** The first thing asked, cut to a caption — the name of an unnamed chat. */
  first_prompt?: string
  /** What the answers in this chat have cost so far. */
  cost_usd: number
  /** Set while this chat holds an unanswered question. */
  waiting_since?: string
  updated_at: string
}

export interface Message {
  id: string
  chat_id: string
  role: 'user' | 'assistant' | 'system'
  body: string
  meta: Meta
  created_at: string
}

export interface Transcript {
  chat: Chat
  messages: Message[]
}

/**
 * Something a run said while it was going.
 *
 * Coarser than the CLI's own output on purpose: kilna shows blocks of an answer
 * and the names of tools being used. Shapes it does not know are dropped by the
 * backend, so anything arriving here is one of these.
 */
export type RunEvent =
  | { kind: 'started'; session_id: string }
  | { kind: 'text'; body: string }
  | { kind: 'tool'; name: string; detail: string }
  | { kind: 'finished'; body: string; cost_usd?: number; duration_ms?: number }
  | { kind: 'failed'; message: string }
  | { kind: 'stopped' }

export type RunState = 'running' | 'done' | 'failed' | 'cancelled' | 'broken'

export interface Run {
  id: string
  chat_id: string
  prompt: string
  state: RunState
  detail?: string
  /** Everything it has said so far, oldest first. */
  events: RunEvent[]
  started_at: string
  ended_at?: string
}

/** One event as it happens, carried on the `assistant:run` channel. */
export interface RunEmission {
  run_id: string
  chat_id: string
  /** Set when the run was started as a profile action, absent when typed. */
  task?: string
  event: RunEvent
}

export interface ExportReport {
  directory: string
  works: number
  files: number
}

export interface ImportReport {
  works: number
  versions: number
  scores: number
  releases: number
  /** Titles already here, left alone. */
  skipped: number
  /** Titles deleted here earlier, not brought back. */
  deleted: number
}

/** What a package export produced, so the person is told rather than guesses
    where it went. */
export interface PackageReport {
  /** The folder that was written, absolute — named after the work, inside the
      one that was chosen. */
  directory: string
  scenes: number
  /** Pictures and clips copied beside the text. */
  files: number
  /** Releases whose metadata went into the folder. */
  releases: number
  /** Scenes with nothing chosen: a package is also how someone finds out
      what the board is still missing. */
  withoutMaterial: number
}

export interface PluginCommand {
  key: string
  label: string
  description?: string
  target: 'release' | 'work'
}

export interface PluginManifest {
  protocol_version: number
  name: string
  version: string
  description?: string
  commands: PluginCommand[]
}

export interface Plugin {
  executable: string
  path: string
  manifest: PluginManifest | null
  usable: boolean
  reason: string | null
}

/** A proposal nobody has answered yet, listed away from the chat it came in. */
export interface PendingProposal {
  message_id: string
  chat_id: string
  chat_title: string | null
  work_id: string | null
  /** `version`, `score`, `note`, `scenes`, `work` or `package`. */
  kind: string
  created_at: string
}

/** Where a started task went, and what it is. */
export interface StartedTask {
  chatId: string
  runId: string
  taskKey: string
  title: string
}

/** What a batch became: what is going, what is waiting, what was passed over. */
export interface StartedBatch {
  started: number
  queued: number
  skipped: number
}

/** What the assistant is carrying, as task keys. */
export interface TaskQueue {
  running: string[]
  waiting: string[]
}

/** What a task is about beyond the work: the version open on the versions
    tab, the scene of a scene action, reference files the run may read. */
export interface TaskAbout {
  versionId?: string
  sceneId?: string
  /** One prompt block of that scene, when the action is about a single block
      rather than the whole scene: regenerating the animation without touching
      the still. A key of the kind's `scene_blocks`. */
  block?: string
  attachments?: string[]
  /** The styles picked for this run, in the order picked, read by a
      template's `{styles}`. See ADR 0031. */
  styleBrickIds?: string[]
}

/** What a task would send: composed by the call that starts one. */
export interface ComposedTask {
  prompt: string
  method?: string
  key: string
  title: string
}
