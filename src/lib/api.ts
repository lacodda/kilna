import { invoke } from '@tauri-apps/api/core'

// These mirror the Rust structs in src-tauri/src. Nothing enforces that they
// agree — see ADR 0003 — so a change on one side means a change here.

export interface Axis {
  key: string
  label: string
  weight: number
  scale: number
  description?: string
}

export interface Tier {
  key: string
  label: string
  min: number
}

export interface Kind {
  key: string
  label: string
}

// What a status means to the automation, in descending finality. `manual` is
// never derived: it is a decision someone made, with no fact behind it.
export type Derive = 'manual' | 'draft' | 'scored' | 'scheduled' | 'released'

export interface Status extends Kind {
  derive: Derive
}

/** A flag the author raises by hand, beside the status the app derives. */
export interface Mark {
  key: string
  label: string
  /** A palette role rather than a colour, so it reads in both themes. */
  colour?: 'plain' | 'accent' | 'good' | 'warn' | 'bad'
}

export interface MetaField {
  key: string
  label: string
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

export interface PromptTemplate {
  key: string
  label: string
  template: string
  description?: string
  /** What the action asks for beyond prose. Absent means an ordinary action. */
  produces?: 'score'
}

// What a release of this kind cannot ship without: version role keys. An empty
// list states no requirements, and readiness marks render as inapplicable.
export interface ReleaseKind extends Kind {
  requires: string[]
}

// The pace releases go out at. `default_time` (HH:MM) is a hint shown beside
// the date — slots stay whole days, the contest is per day.
export interface Rhythm {
  every_days: number
  default_time: string | null
}

export interface ProfileConfig {
  work_kinds: Kind[]
  release_kinds: ReleaseKind[]
  collection_kinds: Kind[]
  version_roles: Kind[]
  statuses: Status[]
  axes: Axis[]
  tiers: Tier[]
  work_meta_fields: MetaField[]
  // Absent in a profile written before marks existed.
  marks?: Mark[]
  prompts: PromptTemplate[]
  // Absent in a profile written before the field existed: the auto-layout then
  // has nothing to pace by, and its button says so instead of guessing.
  rhythm?: Rhythm | null
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
  marks?: string[]
  current_version_id?: string | null
}

export interface WorkFilter {
  status?: string
  kind?: string
  collection_id?: string
  search?: string
}

export interface Version {
  id: string
  work_id: string
  role: string
  revision: number
  label: string | null
  body: string
  meta: Meta
  created_at: string
}

export interface VersionSummary {
  id: string
  work_id: string
  role: string
  revision: number
  label: string | null
  length: number
  created_at: string
  is_current: boolean
}

export interface NewVersion {
  role: string
  body: string
  label?: string | null
  meta?: Meta | null
  make_current?: boolean
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
  created_at: string
  updated_at: string
}

export interface NewFocusNote {
  body: string
  work_id?: string | null
}

export interface FocusNotePatch {
  body?: string
  work_id?: string | null
  pinned?: boolean
}

export interface Score {
  id: string
  work_id: string
  version_id: string | null
  axes: Record<string, number>
  total: number
  tier: string | null
  note: string | null
  scored_at: string
  revision: number | null
}

export interface NewScore {
  axes: Record<string, number>
  version_id?: string | null
  note?: string | null
}

export interface ScoredWork {
  work_id: string
  title: string
  kind: string
  status: string
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
  total: number | null
  tier: string | null
  readiness: Readiness
}

export interface NewRelease {
  work_id: string
  kind: string
  title?: string | null
  scheduled_at?: string | null
  meta?: Meta | null
}

export interface Scheduling {
  release: Release
  /** The release that lost the slot, if this one displaced something. */
  displaced: Release | null
}

// The dry run of a claim: the same verdict `schedule_release` would act on,
// shown before the drop instead of announced after it.
export type SlotVerdict = 'empty' | 'displaces' | 'held' | 'pinned'

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
  created_at: string
  updated_at: string
  works: number
}

export interface NewCollection {
  kind: string
  title: string
  description?: string | null
  meta?: Meta | null
}

export const getWorkspace = () => invoke<Workspace>('get_workspace')
export const listProfiles = () => invoke<Profile[]>('list_profiles')
export const activateProfile = (id: string) => invoke<void>('activate_profile', { id })
export const updateProfileConfig = (id: string, config: ProfileConfig) =>
  invoke<Profile>('update_profile_config', { id, config })

export const listWorks = (filter?: WorkFilter) => invoke<Work[]>('list_works', { filter })
export const getWork = (id: string) => invoke<Work | null>('get_work', { id })
export const createWork = (work: NewWork) => invoke<Work>('create_work', { work })
export const updateWork = (id: string, patch: WorkPatch) => invoke<Work>('update_work', { id, patch })
export const deleteWork = (id: string) => invoke<string>('delete_work', { id })

// A status the automation would change, or did.
export interface StatusChange {
  work_id: string
  title: string
  from: string
  to: string
}

export const statusDrift = () => invoke<StatusChange[]>('status_drift')
export const resyncStatuses = () => invoke<StatusChange[]>('resync_statuses')
export const unpinStatus = (id: string) => invoke<Work>('unpin_status', { id })

export const listVersions = (workId: string) => invoke<VersionSummary[]>('list_versions', { workId })
export const getVersion = (id: string) => invoke<Version | null>('get_version', { id })
export const createVersion = (workId: string, version: NewVersion) =>
  invoke<Version>('create_version', { workId, version })
export const setCurrentVersion = (workId: string, versionId: string) =>
  invoke<void>('set_current_version', { workId, versionId })
export const deleteVersion = (id: string) => invoke<string>('delete_version', { id })

export const listNotes = (filter?: NoteFilter) => invoke<Note[]>('list_notes', { filter })
export const createNote = (note: NewNote) => invoke<Note>('create_note', { note })
export const updateNote = (id: string, patch: NotePatch) => invoke<Note>('update_note', { id, patch })
export const deleteNote = (id: string) => invoke<string>('delete_note', { id })
export const listTags = () => invoke<[string, number][]>('list_tags')

/** Tags in use on works, most used first — what the tag box offers. */
export const workTags = () => invoke<[string, number][]>('work_tags')

export const dismissedFindings = () => invoke<Dismissal[]>('dismissed_findings')
export const dismissFinding = (key: DismissalKey) => invoke<Dismissal>('dismiss_finding', { key })
export const restoreFinding = (key: DismissalKey) => invoke<void>('restore_finding', { key })
export const listFocusNotes = () => invoke<FocusNote[]>('list_focus_notes')
export const createFocusNote = (note: NewFocusNote) =>
  invoke<FocusNote>('create_focus_note', { note })
export const updateFocusNote = (id: string, patch: FocusNotePatch) =>
  invoke<FocusNote>('update_focus_note', { id, patch })
export const reorderFocusNotes = (order: string[]) => invoke<void>('reorder_focus_notes', { order })
export const deleteFocusNote = (id: string) => invoke<void>('delete_focus_note', { id })

export const scoreWork = (workId: string, score: NewScore) =>
  invoke<Score>('score_work', { workId, score })
export const scoreHistory = (workId: string) => invoke<Score[]>('score_history', { workId })
export const latestScore = (workId: string) => invoke<Score | null>('latest_score', { workId })
export const deleteScore = (id: string) => invoke<string>('delete_score', { id })
export const catalogue = () => invoke<ScoredWork[]>('catalogue')

export const createRelease = (release: NewRelease) => invoke<Release>('create_release', { release })
export const deleteRelease = (id: string) => invoke<string>('delete_release', { id })
// A field left out is untouched; `null` inside a nullable field clears it.
export interface ReleasePatch {
  kind?: string
  title?: string | null
  scheduled_at?: string | null
  url?: string | null
  meta?: Meta
}

export const updateRelease = (id: string, patch: ReleasePatch) =>
  invoke<Release>('update_release', { id, patch })

export const scheduleRelease = (id: string, slot: string) =>
  invoke<Scheduling>('schedule_release', { id, slot })
export const previewSchedule = (id: string, slot: string) =>
  invoke<SlotPreview>('preview_schedule', { id, slot })
// `today` is the user's local date: the backend only knows UTC, which at a
// negative offset is already tomorrow. Returns how many gaps are standing.
export const warnUnreadyReleases = (today: string) =>
  invoke<number>('warn_unready_releases', { today })
export const setSlotPin = (id: string, pinned: boolean) =>
  invoke<Release>('set_slot_pin', { id, pinned })
export const unscheduleRelease = (id: string) => invoke<Release>('unschedule_release', { id })
export const markReleased = (id: string, url?: string | null) =>
  invoke<Release>('mark_released', { id, url })
// One line of an auto-layout plan: this release lands on this day. What
// `planLayout` returns is exactly what `applyLayout` takes back — the preview
// is the contract, not a sketch.
export interface Placement {
  release_id: string
  date: string
}

export const planLayout = (today: string) => invoke<Placement[]>('plan_layout', { today })
export const applyLayout = (placements: Placement[]) =>
  invoke<number>('apply_layout', { placements })

export const calendar = () => invoke<ScheduledRelease[]>('calendar')
export const releaseQueue = () => invoke<ScheduledRelease[]>('release_queue')
export const releasesForWork = (workId: string) => invoke<Release[]>('releases_for_work', { workId })

export const listCollections = () => invoke<Collection[]>('list_collections')
export const createCollection = (collection: NewCollection) =>
  invoke<Collection>('create_collection', { collection })
export const deleteCollection = (id: string) => invoke<string>('delete_collection', { id })
export const setCollectionContents = (id: string, workIds: string[]) =>
  invoke<void>('set_collection_contents', { id, workIds })

/** What a trashed entry was. Mirrors the backend's `trash::Entity`. */
export type DeletedEntity = 'work' | 'version' | 'score' | 'release' | 'note' | 'collection'

export interface Deletion {
  id: string
  entity: DeletedEntity
  entity_id: string
  label: string
  origin: string | null
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
export type HitKind = 'work' | 'version' | 'note' | 'message'

export interface Hit {
  kind: HitKind
  work_id: string
  work_title: string
  /** The hit's own line: a title, or the text it was found in. */
  title: string
  /** Where it came from — `lyrics · v2`, `note`, `assistant`. */
  detail: string
  rank: number
}

export const search = (query: string) => invoke<Hit[]>('search', { query })

export const listJournal = () => invoke<JournalEntry[]>('list_journal')
export const journalForWork = (workId: string) =>
  invoke<JournalEntry[]>('journal_for_work', { workId })
export const unreadJournal = () => invoke<number>('unread_journal')
export const markJournalRead = () => invoke<number>('mark_journal_read')

export const listDeletions = () => invoke<Deletion[]>('list_deletions')
export const restoreDeletion = (id: string) => invoke<void>('restore_deletion', { id })
export const purgeDeletion = (id: string) => invoke<void>('purge_deletion', { id })
export const emptyTrash = () => invoke<number>('empty_trash')

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
  skipped: number
}

export const exportMarkdown = (directory: string) =>
  invoke<ExportReport>('export_markdown', { directory })
export const backupWorkspace = (destination: string) =>
  invoke<string>('backup_workspace', { destination })
export const suggestedBackupName = () => invoke<string>('suggested_backup_name')
export const workspacePath = () => invoke<string>('workspace_path')
export const importLegacy = (source: string) => invoke<ImportReport>('import_legacy', { source })

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

export const listPlugins = () => invoke<Plugin[]>('list_plugins')
export const runPlugin = (
  executable: string,
  command: string,
  target: 'release' | 'work',
  id: string,
) => invoke<string | null>('run_plugin', { executable, command, target, id })

export const assistantStatus = () => invoke<Availability>('assistant_status')
export const listChatSummaries = (workId?: string) =>
  invoke<ChatSummary[]>('list_chat_summaries', { workId })
export const createChat = (chat: { work_id?: string | null; title?: string | null }) =>
  invoke<Chat>('create_chat', { chat })
export const renameChat = (id: string, title: string | null) =>
  invoke<void>('rename_chat', { id, title })
export const getTranscript = (chatId: string) =>
  invoke<Transcript | null>('get_transcript', { chatId })
export const deleteChat = (id: string) => invoke<void>('delete_chat', { id })
export const askAssistant = (chatId: string, prompt: string) =>
  invoke<Message>('ask_assistant', { chatId, prompt })
export const startRun = (chatId: string, prompt: string) =>
  invoke<Run>('start_run', { chatId, prompt })
export const cancelRun = (id: string) => invoke<void>('cancel_run', { id })
export const listRuns = (chatId: string) => invoke<Run[]>('list_runs', { chatId })
export const activeRuns = () => invoke<string[]>('active_runs')

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

export const startTask = (workId: string, action: string) =>
  invoke<StartedTask>('start_task', { workId, action })
export const startTasks = (workIds: readonly string[], action: string) =>
  invoke<StartedBatch>('start_tasks', { workIds, action })
export const activeTasks = () => invoke<string[]>('active_tasks')
export const taskQueue = () => invoke<TaskQueue>('task_queue')
export const clearTaskQueue = () => invoke<number>('clear_task_queue')
export const waitingChats = () => invoke<ChatSummary[]>('waiting_chats')
export const clearWaiting = (chatId: string) => invoke<void>('clear_waiting', { chatId })
export const renderPrompt = (workId: string, template: string) =>
  invoke<string>('render_prompt', { workId, template })
