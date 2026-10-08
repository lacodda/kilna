import type { CardCounts, ProfileConfig, Work } from '@/lib/api/types'
import { canBeCut } from '@/lib/cuts'
import { vocabularyOf } from '@/lib/useProfile'
import { coverHoldsAnything } from '@/features/work/tabs/cover/useCoverEdit'

/**
 * The card's tabs, in the order the mockup puts them.
 *
 * One list, read by the tab bar, the router, the card that draws the body,
 * and the setting that picks where a card opens. Until v0.77 four places kept
 * a piece of it each - the list here, which tabs lay out two columns in the
 * card, which ones a kind may lack in the bar and in the card, which ones a
 * card may open on in the settings - and a tab added to one could be missing
 * from the next. What each tab draws is `TabBody`, a table the compiler holds
 * to this list.
 *
 * The cover and the frame stand between the storyboard and the splice since
 * v0.86: what a publication looks like is written after its board and before
 * it is cut and sent out.
 */
export const TABS = [
  'overview',
  'versions',
  'scenes',
  'cover',
  'frame',
  'cuts',
  'score',
  'files',
  'links',
  'notes',
  'comments',
  'assistant',
  'history',
] as const

export type Tab = (typeof TABS)[number]

/**
 * What decides whether a work's tabs exist: what its kind names, and what
 * the work already holds.
 *
 * Both, because either alone strands something. The kind says what a work of
 * it is made of - a song has lyrics and axes but no doors (ADR 0047), an audio
 * release a cover and a frame but no storyboard. The rows say what was
 * written: a work whose kind was changed, or a profile that dropped a list,
 * keeps what it had, and a tab that hid them would hide data nobody can reach
 * any other way. So a tab exists when the kind names what it is for **or**
 * the work holds rows only that tab shows.
 */
export interface CardFacts {
  /** What the work's kind names. */
  names: {
    /** Version roles: lyrics, a plot. */
    roles: boolean
    /** Axes it is scored on. */
    axes: boolean
    /** Kinds of shot or prompt blocks for a board. */
    storyboard: boolean
    /** Kinds of release - whether it goes out itself (`hasDoors`). */
    doors: boolean
    /** The parts its cover prompt is written in. */
    cover: boolean
    /** A still and a loop it plays under (v0.86). */
    frame: boolean
    /** A folder on disk its files are found in (v0.93, ADR 0057). */
    folder: boolean
  }
  /** What the work holds, whatever its kind says now. */
  holds: {
    versions: boolean
    scores: boolean
    scenes: boolean
    files: boolean
    /** Any block of a cover prompt with words in it. */
    cover: boolean
    /** Stretches, a donor to take them from, or stretches taken from it
        (`canBeCut`). A donor counts only for a work cut from pictures - a
        kind with a storyboard: an audio release is made from its song
        whole, under one frame, and has nothing to splice. */
    splice: boolean
  }
}

/** Whether a tab exists for these facts; `null` for a tab every work has. */
type Rule = ((facts: CardFacts) => boolean) | null

// Every tab is held by the card since v0.78: none is a page that scrolls
// whole, so there is no rule for it here any more - each lays itself out on
// `components/frame`.
const RULES: Readonly<Record<Tab, Rule>> = {
  overview: null,
  versions: ({ names, holds }) => names.roles || holds.versions,
  scenes: ({ names, holds }) => names.storyboard || holds.scenes,
  cover: ({ names, holds }) => names.cover || holds.cover,
  // Nothing is held here that another kind could strand: the backend refuses
  // a frame on a kind without one (`work.noFrame`).
  frame: ({ names }) => names.frame,
  cuts: ({ holds }) => holds.splice,
  score: ({ names, holds }) => names.axes || holds.scores,
  // A work's files are its cover, what its releases go out with, and what
  // lies in its folder on disk - which a song has too (v0.93), though it
  // goes out only as what is made from it.
  files: ({ names, holds }) => names.doors || names.folder || holds.files,
  links: null,
  notes: null,
  comments: null,
  assistant: null,
  history: null,
}

/**
 * The tab a card opens on when neither the URL nor the work's kind says.
 *
 * The mockup opens on its Lyrics tab. Here a card opens on Overview: since
 * v0.82 it is the board that says where the work stands - its score, its
 * stage, its text, its fields edited in place - and each widget leads to the
 * tab that owns it. The title is renamed in the header (since v0.80).
 */
export const DEFAULT_TAB: Tab = 'overview'

/**
 * The tab a work opens on when the URL names none: the one its kind names
 * (`open_on`, v0.90.1), or the overview.
 *
 * Per kind, because a song is lived in through its text and a short is made
 * on its board - one tab for every card sent the owner through a click on
 * every song or on every short. On the profile, beside the catalogue's
 * columns by kind, rather than on the machine where one tab for all of them
 * was kept until now. A word this build does not know opens the overview; a
 * tab the work has not got is corrected by the card, by the rule the bar
 * hides it.
 */
export function openingTab(config: ProfileConfig, kind: string): Tab {
  const named = config.work_kinds.find((entry) => entry.key === kind)?.open_on ?? undefined
  return isTab(named) ? named : DEFAULT_TAB
}

export function isTab(value: string | undefined): value is Tab {
  return value !== undefined && (TABS as readonly string[]).includes(value)
}

/** The tabs this work draws, in order. */
export function tabsOf(facts: CardFacts): Tab[] {
  return TABS.filter((tab) => {
    const rule = RULES[tab]
    return rule === null || rule(facts)
  })
}

/** A card with nothing counted yet: what its tab bar is drawn from before
 *  `card_counts` answers, so a tab appears when its rows are known to be there
 *  rather than disappearing when they are known not to be. */
export const NOTHING_COUNTED: CardCounts = {
  versions: 0,
  scores: 0,
  releases: 0,
  files: 0,
  sources: 0,
  derived: 0,
  notes: 0,
  comments: 0,
  comments_waiting: 0,
  scenes: 0,
  cuts: 0,
  cut_from: 0,
  history: 0,
}

/**
 * The facts of one work: its kind's vocabulary, its cover, and its counts.
 *
 * `counts` left out means they have not loaded, and every row is then taken
 * as held: an address naming a tab is trusted until the counts can say the
 * tab is not there, rather than bounced to the overview and lost on the way.
 * The bar is drawn from what is known (`NOTHING_COUNTED` until then).
 */
export function factsOf(
  config: ProfileConfig,
  work: Pick<Work, 'kind' | 'cover'>,
  counts?: CardCounts,
): CardFacts {
  const vocabulary = vocabularyOf(config, work.kind)
  const known = counts !== undefined
  return {
    names: namesOf(config, work.kind),
    holds: {
      versions: !known || counts.versions > 0,
      scores: !known || counts.scores > 0,
      scenes: !known || counts.scenes > 0,
      files: !known || counts.files > 0,
      // Read off the work itself, which is already here.
      cover: coverHoldsAnything(work.cover),
      splice:
        !known ||
        canBeCut(
          counts.cuts,
          vocabulary.shot_types.length > 0 || vocabulary.scene_blocks.length > 0
            ? counts.sources
            : 0,
          counts.cut_from,
        ),
    },
  }
}

/** What a kind names, read off its vocabulary. */
function namesOf(config: ProfileConfig, kind: string): CardFacts['names'] {
  const vocabulary = vocabularyOf(config, kind)
  return {
    roles: vocabulary.version_roles.length > 0,
    axes: vocabulary.axes.length > 0,
    storyboard: vocabulary.shot_types.length > 0 || vocabulary.scene_blocks.length > 0,
    doors: vocabulary.release_kinds.length > 0,
    cover: vocabulary.cover,
    frame: vocabulary.frame,
    folder: (config.work_kinds.find((entry) => entry.key === kind)?.folder ?? null) !== null,
  }
}

/**
 * The tabs every work of a kind draws, by what the kind names alone: what a
 * person may make the kind's works open on (Settings -> The work card).
 *
 * Not what one work holds - a splice exists only for a work cut from
 * something, and a song whose kind lost its roles keeps its Versions tab only
 * while it holds versions - so a choice offered here is a tab each new work
 * of the kind has. The board, the cover and the frame are offered where the
 * kind has them: a short is made on its board, and opening it there is the
 * habit the choice is for.
 */
export function tabsOfKind(config: ProfileConfig, kind: string): Tab[] {
  return tabsOf({
    names: namesOf(config, kind),
    holds: {
      versions: false,
      scores: false,
      scenes: false,
      files: false,
      cover: false,
      splice: false,
    },
  })
}

/** The number beside a tab. */
export interface TabCount {
  value: number
  /** Something here waits for the person - a comment with no answer yet -
      and the number says how many of those rather than how many in all. */
  waiting: boolean
}

/**
 * The number beside each tab, from the card's one answer (`card_counts`).
 *
 * A tab with nothing in it has none: a row of zeros reads as a row of
 * warnings. The overview and the assistant count nothing - one is the work
 * itself, the other a conversation - so they never have one. The comments'
 * number is what still waits when anything does, and the total otherwise:
 * what the audience said is about the work whichever tab is open, and what
 * waits for an answer is worth a mark.
 */
export function tabCounts(counts: CardCounts): Partial<Record<Tab, TabCount>> {
  const plain: Partial<Record<Tab, number>> = {
    versions: counts.versions,
    scenes: counts.scenes,
    cuts: counts.cuts,
    score: counts.scores,
    files: counts.files,
    // Made either way: "Links 2" is how a song shows it has clips without
    // anyone opening the tab.
    links: counts.sources + counts.derived,
    notes: counts.notes,
    history: counts.history,
  }
  const shown: Partial<Record<Tab, TabCount>> = {}
  for (const [tab, value] of Object.entries(plain) as [Tab, number][]) {
    if (value > 0) shown[tab] = { value, waiting: false }
  }
  if (counts.comments_waiting > 0) {
    shown.comments = { value: counts.comments_waiting, waiting: true }
  } else if (counts.comments > 0) {
    shown.comments = { value: counts.comments, waiting: false }
  }
  return shown
}
