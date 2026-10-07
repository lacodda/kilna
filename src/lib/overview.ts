import type {
  Label,
  MetaField,
  OverviewLayout,
  ProfileConfig,
  Publication,
  ReleaseKind,
  ScheduledRelease,
  Score,
  Stage,
  VersionRole,
  VersionSummary,
  WidgetPlacement,
  WidgetSize,
} from '@/lib/api/types'
import { predecessor } from '@/lib/history'
import { stageAt, stagesOf } from '@/lib/stages'

/*
 * The overview as a board of widgets (v0.82): which widgets a work's overview
 * draws, in what order and at what size, and the facts each one reads.
 *
 * The board is laid out by the profile (`config.overview`): a layout - one of
 * the five the owner was shown - and a list of placements, `{id, size,
 * position}`. The list is there from the start even though nothing edits it
 * yet (the owner's rule of 20.09, "a widget knows its size"): when a setting
 * to drag and resize arrives, it writes this list and nothing that draws the
 * board has to change.
 *
 * Everything here is a function of plain data, so the rules of the board are
 * tested without a window.
 */

/** Every widget the board can draw, by the id a placement names it with. */
export const WIDGETS = [
  'score',
  'stage',
  'text',
  'style',
  'fields',
  'axes',
  'hook',
  'release',
  'links',
  'recent',
  'storyboard',
  'cover',
  'findings',
  'trend',
  'publications',
  'repeats',
] as const

export type WidgetId = (typeof WIDGETS)[number]

/** The five arrangements, in the order the owner was shown them. */
export const LAYOUTS: readonly OverviewLayout[] = ['grid', 'lead', 'bands', 'mosaic', 'sheet']

/** The owner's choice of 24.09: the lead column and the rail beside it. */
const DEFAULT_LAYOUT: OverviewLayout = 'lead'

function isWidgetId(value: string): value is WidgetId {
  return (WIDGETS as readonly string[]).includes(value)
}

function isLayout(value: unknown): value is OverviewLayout {
  return typeof value === 'string' && (LAYOUTS as readonly string[]).includes(value)
}

/** One widget as the board draws it. */
export interface Placed {
  id: WidgetId
  size: WidgetSize
}

/**
 * The placement kilna ships, for a profile that names none.
 *
 * One list for every layout, so switching the layout never has to invent a
 * placement (the contract's rule): each layout reads the sizes its own way.
 * The sizes are the mosaic's, where a size shows most plainly - the text two
 * by two, the fields and the idea two across, a number one cell - and they
 * are also what puts a widget in the lead column or on the rail (`leadOf`),
 * which is why the style prompt and the fields are two across: the owner's
 * lead layout pairs them under the text.
 *
 * What needs attention comes first, so on any layout it is the first thing
 * read; it is only drawn while there is something to say (`applies`). A
 * song's publications come next and across the lead (v0.86): a song goes
 * out as what is made from it, and where those stand is where the song
 * stands - the first thing a song's card is opened to check. A publication's
 * release stands in the same place on its own card (v0.90): it goes out
 * once, and the release - where, when, under what words - is what its card
 * is opened to write.
 *
 * The guard's findings stand first of all, beside "Needs attention" (v0.90,
 * ADR 0054): two across, so the two pair up in the lead column - both ask
 * for a decision - and either takes the width alone when it is the only one
 * with something to say.
 */
export const DEFAULT_PLACEMENT: readonly WidgetPlacement[] = (
  [
    ['repeats', 'm'],
    ['findings', 'm'],
    ['publications', 'l'],
    ['release', 'l'],
    ['text', 'l'],
    ['score', 's'],
    ['axes', 's'],
    ['stage', 's'],
    ['style', 'm'],
    ['fields', 'm'],
    ['hook', 'm'],
    ['links', 's'],
    ['recent', 's'],
    ['storyboard', 'm'],
    ['trend', 's'],
    ['cover', 's'],
  ] as const
).map(([id, size], position) => ({ id, size, position }))

/** The overview a profile asks for, with everything it leaves out filled in. */
export interface Board {
  layout: OverviewLayout
  widgets: Placed[]
}

/**
 * Read `config.overview` the way the board draws it.
 *
 * Absent or null is the owner's choice: the lead layout and the placement
 * kilna ships. An empty list of widgets is that placement too. A widget id this
 * build does not know is skipped rather than refused - a profile written by a
 * newer kilna still opens - and so is a second placement of the same widget,
 * because one widget drawn twice is two places to edit one field. The order is
 * the placements' `position`, ties kept in the order they are listed.
 */
export function boardOf(config: Pick<ProfileConfig, 'overview'>): Board {
  const overview = config.overview ?? null
  const asked = overview?.layout
  const layout = isLayout(asked) ? asked : DEFAULT_LAYOUT
  const listed = overview !== null && overview.widgets.length > 0 ? overview.widgets : null
  const source = listed ?? DEFAULT_PLACEMENT

  const seen = new Set<WidgetId>()
  const widgets: Placed[] = []
  const ordered = source
    .map((placement, index) => ({ placement, index }))
    .sort((a, b) => a.placement.position - b.placement.position || a.index - b.index)
  for (const { placement } of ordered) {
    if (!isWidgetId(placement.id) || seen.has(placement.id)) continue
    seen.add(placement.id)
    widgets.push({ id: placement.id, size: sizeOf(placement.size) })
  }
  return { layout, widgets }
}

/** A size the contract knows, and the smallest for anything else. */
function sizeOf(size: unknown): WidgetSize {
  return size === 'm' || size === 'l' ? size : 's'
}

/** What a work has that decides whether a widget is drawn at all. */
export interface WidgetFacts {
  /** The kind is judged on axes: a score, its axes and its trend exist. */
  scored: boolean
  /** The kind names a role the work is written in. */
  text: boolean
  /** The kind names a role written about the work rather than as it. */
  style: boolean
  /** The profile has paragraph fields: the premise, the idea. */
  prose: boolean
  /** The kind names ways a work of it goes out. */
  releases: boolean
  /** The kind names none: a work of it goes out as what is made from it -
   *  a song as its clip, its audio and its shorts (ADR 0047). */
  publications: boolean
  /** The kind has a storyboard. */
  scenes: boolean
  /** The work can have a picture: it goes out itself, or it already holds
   *  one. A song goes out as what is made from it, and its cover is theirs -
   *  an invitation to set one would lead to a Files tab it does not have. */
  cover: boolean
  /** Something about this work is waiting for a decision. */
  findings: boolean
  /** The guard of repeats has findings about the work's song, kept or not
   *  (ADR 0054): a song held against the songs out or booked, or the song a
   *  publication is made from. */
  repeats: boolean
}

/**
 * Whether a widget belongs on this work's board.
 *
 * What does not apply is not drawn (the owner's rule): a song has no
 * storyboard, so there is no storyboard widget on its overview - not an empty
 * one saying so, which would be noise about the impossible. That is different
 * from a widget that applies and has nothing yet, which is drawn as an
 * invitation.
 */
export function applies(id: WidgetId, facts: WidgetFacts): boolean {
  switch (id) {
    case 'score':
    case 'axes':
    case 'trend':
      return facts.scored
    case 'text':
      return facts.text
    case 'style':
      return facts.style
    case 'hook':
      return facts.prose
    case 'release':
      return facts.releases
    case 'publications':
      return facts.publications
    case 'storyboard':
      return facts.scenes
    case 'findings':
      return facts.findings
    case 'repeats':
      return facts.repeats
    case 'cover':
      return facts.cover
    case 'stage':
    case 'fields':
    case 'links':
    case 'recent':
      return true
  }
}

/** One row of the lead column: a widget across it, or two side by side. */
export type LeadRow = { kind: 'one'; widget: Placed } | { kind: 'pair'; widgets: [Placed, Placed] }

/**
 * The lead layout's two columns: what is read (the lead) and what is glanced
 * at (the rail).
 *
 * A widget two by two, or two across, is something to read, and goes in the
 * lead; a one-cell widget is a figure, and stands on the rail. In the lead a
 * large widget takes the column's width, and two-across widgets that follow
 * one another pair up side by side - the owner's picture: the text across the
 * column, the style prompt and the fields in a pair under it. One left
 * without a partner takes the width alone rather than leaving half a row
 * empty.
 */
export function leadOf(widgets: readonly Placed[]): { lead: LeadRow[]; rail: Placed[] } {
  const lead: LeadRow[] = []
  const rail: Placed[] = []
  let waiting: Placed | null = null

  const flush = () => {
    if (waiting !== null) lead.push({ kind: 'one', widget: waiting })
    waiting = null
  }

  for (const widget of widgets) {
    if (widget.size === 's') {
      rail.push(widget)
    } else if (widget.size === 'l') {
      flush()
      lead.push({ kind: 'one', widget })
    } else if (waiting === null) {
      waiting = widget
    } else {
      lead.push({ kind: 'pair', widgets: [waiting, widget] })
      waiting = null
    }
  }
  flush()
  return { lead, rail }
}

/**
 * The role a work is written in: the first that is a draft of the work - not
 * a commentary on another role, and not a text written about it the way a
 * style prompt is (`counts_as_version: false`). A song's lyrics, a clip's plot.
 */
export function textRoleOf(roles: readonly VersionRole[]): VersionRole | undefined {
  return roles.find((role) => role.comments_on == null && role.counts_as_version !== false)
}

/**
 * The role written about the work rather than as it - a song's style prompt:
 * a body of its own that a craft keeps beside the text and copies elsewhere.
 */
export function styleRoleOf(roles: readonly VersionRole[]): VersionRole | undefined {
  return roles.find((role) => role.comments_on == null && role.counts_as_version === false)
}

/** Newest first: by when it was written, and by revision on the same moment. */
function newestFirst(a: VersionSummary, b: VersionSummary): number {
  return b.created_at.localeCompare(a.created_at) || b.revision - a.revision
}

/**
 * The version of `role` the overview shows: the work's current version when it
 * is in this role, the newest of the role otherwise. A work has one current
 * version, and a song whose current version is its lyrics still has a style
 * prompt to show - the newest one.
 */
export function currentIn(
  versions: readonly VersionSummary[],
  role: string,
): VersionSummary | undefined {
  const own = versions.filter((version) => version.role === role)
  return own.find((version) => version.is_current) ?? [...own].sort(newestFirst)[0]
}

/**
 * The version `version` would be compared with, by the one rule the Versions
 * tab compares by (`lib/history`'s `predecessor`): the one it was written
 * from, when that is still there in the same role, and the revision before it
 * otherwise.
 */
export function previousOf(
  versions: readonly VersionSummary[],
  version: VersionSummary,
): VersionSummary | undefined {
  const own = versions.filter((other) => other.role === version.role)
  return predecessor(own, version.id) ?? undefined
}

/**
 * The profile's fields, split the way the board draws them: the short ones
 * - a number, a word, a date, a yes or no - in the fields widget's grid, and
 * the paragraphs - a premise, an idea - in the hook widget, where a paragraph
 * has the width to be read. In a grid cell a paragraph was a column of
 * two-word lines.
 */
export function fieldsOf(fields: readonly MetaField[]): { short: MetaField[]; prose: MetaField[] } {
  return {
    short: fields.filter((field) => field.type !== 'multiline'),
    prose: fields.filter((field) => field.type === 'multiline'),
  }
}

/**
 * The last `count` scores, oldest first, from a history that comes newest
 * first: the trail a verdict is read with - `78.0 → 86.0 → 91.0`.
 */
export function trailOf(history: readonly Score[], count: number): Score[] {
  return history.slice(0, count).reverse()
}

/** The stop after the one a work is at; `undefined` at the last, or unset. */
export function nextStage(config: ProfileConfig, percent: number | null): Stage | undefined {
  const current = stageAt(config, percent)
  if (current === undefined) return undefined
  return stagesOf(config).find((stop) => stop.percent > current.percent)
}

/** A condition, as the line's status vocabulary names it. */
export type Tone = 'good' | 'warn' | 'info' | 'neutral'

/**
 * Where a release stands, as the colour of its dot.
 *
 * Out is good; on the calendar and ready is information - a date, nothing to
 * do; everything else wants something from the person - a day, or what the
 * release still lacks - and is drawn in the warning hue, as the owner's
 * picture draws a clip nobody has scheduled. The colour is never the only
 * thing that says it: the row's words do too.
 */
export function releaseTone(release: ScheduledRelease): Tone {
  if (release.status === 'released') return 'good'
  if (release.scheduled_at !== null && release.readiness.ready) return 'info'
  return 'warn'
}

/**
 * The first `lines` lines of a body, and whether there is more.
 *
 * Lines rather than characters: a lyric is read by the line, and a preview cut
 * mid-line reads as a line that ends there. A trailing run of blank lines is
 * not "more".
 */
export function previewOf(body: string, lines: number): { text: string; more: boolean } {
  const all = body.replace(/\s+$/, '').split('\n')
  return { text: all.slice(0, lines).join('\n'), more: all.length > lines }
}

/**
 * What a publication's chip says about it: out, with the day it went out;
 * booked, with the day it holds and the hour when one is set - late when
 * that day has passed and nothing went out; otherwise its status word, and
 * nothing to date. A publication goes out once (ADR 0051), so its one
 * release says it.
 */
export type PublicationFact =
  | { said: 'out' | 'booked' | 'late'; day: string; time: string | null }
  | { said: 'status'; day: null; time: null }

export function publicationFact(
  publication: Pick<Publication, 'release'>,
  today: string,
): PublicationFact {
  const going = publication.release
  if (going === null) return { said: 'status', day: null, time: null }
  if (going.status === 'released' && going.released_at !== null) {
    return { said: 'out', day: going.released_at, time: null }
  }
  const next = going.scheduled_at
  if (next !== null) {
    return { said: next < today ? 'late' : 'booked', day: next, time: going.scheduled_time }
  }
  return { said: 'status', day: null, time: null }
}

/** The colour each of those facts wears; the status word wears its own. */
export const FACT_TONE: Record<
  Exclude<PublicationFact['said'], 'status'>,
  Exclude<Tone, 'neutral'>
> = {
  out: 'good',
  booked: 'info',
  late: 'warn',
}

/**
 * A kind's label as a word inside a sentence - "Make an audio", "Released ·
 * as audio Sep 22": the first letter lowered, since the profile writes a
 * label to stand alone. A first word with a capital past its first letter -
 * an abbreviation, "MV", or a name, "YouTube" - keeps every capital it has:
 * lowered, it is a different word.
 */
export function wordOf(label: string, language?: string): string {
  const rest = (label.split(/\s/)[0] ?? '').slice(1)
  if (rest !== rest.toLocaleLowerCase(language)) return label
  return label.charAt(0).toLocaleLowerCase(language) + label.slice(1)
}

/**
 * Whether an English article before `word` is "an": a word that starts with
 * a vowel letter. The sentence is the locale's; only the article is chosen
 * here, and a language without articles says the same either way.
 */
export function takesAn(word: string): boolean {
  return /^[aeiou]/i.test(word)
}

/**
 * Where works of a kind go out, as one line: each door with the shape its
 * cover is drawn in - "YouTube · 16:9, Streaming · 1:1". The shape belongs
 * to the door (ADR 0047), so it is said beside the door, not once for the
 * kind. Empty for a kind with no door.
 */
export function doorsOf(
  doors: readonly Pick<ReleaseKind, 'label' | 'cover_format'>[],
  name: (label: Label) => string,
): string {
  return doors
    .map((door) =>
      door.cover_format == null || door.cover_format === ''
        ? name(door.label)
        : `${name(door.label)} · ${door.cover_format}`,
    )
    .join(', ')
}
