import {
  ArrowRightLeft,
  CalendarClock,
  CalendarPlus,
  CalendarRange,
  CalendarX,
  Clapperboard,
  Copy,
  Dot,
  Lightbulb,
  Link2,
  ListChecks,
  ListOrdered,
  Paperclip,
  PenLine,
  Pin,
  PinOff,
  Plus,
  RefreshCw,
  RotateCcw,
  Scissors,
  Send,
  Sparkles,
  Star,
  Timer,
  Trash2,
  TriangleAlert,
  Type,
  Undo2,
  type LucideIcon,
} from 'lucide-react'
import type { JournalEntry } from '@/lib/api/types'

/*
 * How a line of history looks before it is read: a glyph on a tinted tile, and
 * a word for what it is about.
 *
 * The feed is scanned, not read - the owner looks down it for "was that
 * scored?" or "what moved?", and a column of sentences in one weight made him
 * read every one. The mockup gives each line a 26px tile tinted by what
 * happened, in four tones:
 *
 * - `good` - something landed: a score, a release that went out.
 * - `accent` - something was made or asked for: a work, a version, a scene, a
 *   proposal from the assistant.
 * - `warn` - something moved, or asks for a look: a status, a day in the
 *   calendar, a release that is not ready. Every line written as a warning is
 *   warn whatever its action says.
 * - `dim` - something went, was taken back, or was tidied: a deletion, an
 *   undo, a restore, a rename.
 *
 * The glyph is the second channel, for a reader who does not separate the
 * hues, and the word under the sentence names the area in text.
 */

export type JournalTone = 'good' | 'accent' | 'warn' | 'dim'

export interface JournalLook {
  tone: JournalTone
  glyph: LucideIcon
  /** The key of the word for what the line is about, or null when its action
   * names no area the window knows. */
  kind: string | null
}

/**
 * The word for each area, one key per area.
 *
 * Written out rather than built from the prefix, for the reason the trash
 * gives for its kinds: a key spelled at run time is invisible to the compiler
 * and to the locale check, and a new area would show as `journal.kind.x`.
 */
const KIND_KEYS = {
  work: 'journal.kind.work',
  version: 'journal.kind.version',
  score: 'journal.kind.score',
  release: 'journal.kind.release',
  calendar: 'journal.kind.calendar',
  scene: 'journal.kind.scene',
  cut: 'journal.kind.cut',
  link: 'journal.kind.link',
  file: 'journal.kind.file',
  note: 'journal.kind.note',
  comment: 'journal.kind.comment',
  style: 'journal.kind.style',
  collection: 'journal.kind.collection',
  canon: 'journal.kind.canon',
  register: 'journal.kind.register',
  cover: 'journal.kind.cover',
  assistant: 'journal.kind.assistant',
  trash: 'journal.kind.trash',
  undo: 'journal.kind.undo',
} as const

type Kind = keyof typeof KIND_KEYS

/**
 * The area an action is about, read from its prefix.
 *
 * The prefix rather than the whole action, as the journal's links read it: a
 * new `score.something` is about a score without anyone coming back here.
 */
const KIND_OF_PREFIX: Readonly<Record<string, Kind>> = {
  work: 'work',
  status: 'work',
  version: 'version',
  score: 'score',
  tier: 'score',
  release: 'release',
  upgrade: 'release',
  layout: 'calendar',
  scene: 'scene',
  cut: 'cut',
  link: 'link',
  asset: 'file',
  note: 'note',
  comment: 'comment',
  style: 'style',
  collection: 'collection',
  fact: 'canon',
  term: 'register',
  idea: 'cover',
  proposal: 'assistant',
  assistant: 'assistant',
  trash: 'trash',
  undo: 'undo',
}

/**
 * The tone and the glyph of every action the window words.
 *
 * Whole actions rather than verbs: `release.pinned` is a decision and
 * `release.unpinned` the taking of one back, and no rule about suffixes says
 * so. A test holds this table to the locale, so an action given a sentence
 * and no look fails there rather than drawing the generic dot.
 */
const LOOKS: Readonly<Record<string, readonly [JournalTone, LucideIcon]>> = {
  'work.created': ['accent', Plus],
  'work.cloned': ['accent', Copy],
  'work.renamed': ['dim', Type],
  'work.status': ['warn', RefreshCw],
  'work.restated': ['warn', RefreshCw],
  'work.statusBatch': ['warn', RefreshCw],
  'status.resynced': ['dim', RefreshCw],
  'work.deleted': ['dim', Trash2],
  'work.deletedBatch': ['dim', Trash2],
  'tier.pinned': ['accent', Pin],
  'tier.unpinned': ['dim', PinOff],

  'version.created': ['accent', PenLine],
  'version.deleted': ['dim', Trash2],

  'score.added': ['good', Star],
  'score.deleted': ['dim', Trash2],

  'release.created': ['accent', CalendarPlus],
  'release.scheduled': ['warn', CalendarClock],
  'release.moved': ['warn', CalendarClock],
  'release.pinned': ['accent', Pin],
  'release.unpinned': ['dim', PinOff],
  'release.notReady': ['warn', TriangleAlert],
  'release.released': ['good', Send],
  'release.unreleased': ['dim', Undo2],
  'release.deleted': ['dim', Trash2],
  'release.unscheduledBatch': ['dim', CalendarX],
  'release.fieldsBatch': ['dim', ListChecks],
  'release.metaWritten': ['accent', ListChecks],
  'layout.applied': ['warn', CalendarRange],
  'upgrade.doorsMoved': ['dim', ArrowRightLeft],
  'upgrade.publicationsMoved': ['dim', ArrowRightLeft],
  'upgrade.publicationDisputed': ['warn', TriangleAlert],

  'scene.created': ['accent', Clapperboard],
  'scene.framed': ['accent', Clapperboard],
  'scene.timed': ['dim', Timer],
  'scene.renumbered': ['dim', ListOrdered],
  'scene.deleted': ['dim', Trash2],
  'cut.created': ['accent', Scissors],
  'cut.deleted': ['dim', Trash2],

  'link.created': ['accent', Link2],
  'link.removed': ['dim', Trash2],
  'asset.attached': ['accent', Paperclip],
  'note.promoted': ['accent', Lightbulb],
  'note.deleted': ['dim', Trash2],
  'comment.deleted': ['dim', Trash2],
  'style.deleted': ['dim', Trash2],
  'collection.deleted': ['dim', Trash2],
  'fact.deleted': ['dim', Trash2],
  'term.deleted': ['dim', Trash2],
  'idea.deleted': ['dim', Trash2],

  'proposal.work': ['accent', Sparkles],
  'proposal.version': ['accent', Sparkles],
  'proposal.score': ['accent', Sparkles],
  'proposal.note': ['accent', Sparkles],
  'proposal.freeNote': ['accent', Sparkles],
  'proposal.package': ['accent', Sparkles],
  'proposal.scenes': ['accent', Sparkles],
  'proposal.canon': ['accent', Sparkles],
  'proposal.freeCanon': ['accent', Sparkles],
  'proposal.release': ['accent', Sparkles],
  'proposal.coverIdeas': ['accent', Sparkles],
  'assistant.batchStarted': ['accent', Sparkles],

  'trash.restored': ['dim', RotateCcw],
  'undo.done': ['dim', Undo2],
}

/** The actions with a look of their own - for the test that holds the table
 * to the locale. */
export const LOOKED_UP_ACTIONS: readonly string[] = Object.keys(LOOKS)

/**
 * What a line looks like.
 *
 * An action the table does not hold - one a newer backend wrote, or a key
 * nobody worded - is still drawn: a deletion as one, a creation as one, and
 * anything else as a plain dot. A line of history is never left without a
 * mark; a blank tile beside its neighbours would read as broken.
 */
export function journalLook(entry: Pick<JournalEntry, 'action' | 'level'>): JournalLook {
  const prefix = entry.action.split('.')[0] ?? ''
  const kind = Object.hasOwn(KIND_OF_PREFIX, prefix) ? KIND_KEYS[KIND_OF_PREFIX[prefix]!] : null
  // `hasOwn`, as the release glyphs learned: `LOOKS['constructor']` is not
  // undefined, and React would try to draw what it is.
  const [tone, glyph] = Object.hasOwn(LOOKS, entry.action)
    ? LOOKS[entry.action]!
    : fallback(entry.action)
  const warned = entry.level === 'warn'
  return {
    tone: warned ? 'warn' : tone,
    glyph: warned && glyph === Dot ? TriangleAlert : glyph,
    kind,
  }
}

function fallback(action: string): readonly [JournalTone, LucideIcon] {
  const verb = action.split('.').at(-1) ?? ''
  if (verb.startsWith('deleted')) return ['dim', Trash2]
  if (verb === 'created') return ['accent', Plus]
  return ['dim', Dot]
}

/**
 * Whether a line still asks to be looked at: a warning nobody has marked seen.
 *
 * One answer for the dot on the line, the count on the filter and the filter
 * itself, which used to be three copies of one condition.
 */
export function needsALook(entry: Pick<JournalEntry, 'level' | 'read_at'>): boolean {
  return entry.level === 'warn' && entry.read_at === null
}
