import type { CardCounts } from '@/lib/api/types'

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
 */
export const TABS = [
  'overview',
  'versions',
  'scenes',
  'cuts',
  'score',
  'releases',
  'files',
  'links',
  'notes',
  'comments',
  'assistant',
  'history',
] as const

export type Tab = (typeof TABS)[number]

interface TabRules {
  /**
   * What a work must have for the tab to exist: a storyboard is a fact of the
   * kind - a song has none - and a splice a fact of the work, whether it was
   * cut out of another. Without it the tab is not drawn, and an address
   * naming it goes to the default tab.
   */
  needs?: 'storyboard' | 'splice'
}

// Every tab is held by the card since v0.78: none is a page that scrolls
// whole, so there is no rule for it here any more - each lays itself out on
// `components/frame`.
const RULES: Readonly<Record<Tab, TabRules>> = {
  overview: {},
  versions: {},
  scenes: { needs: 'storyboard' },
  cuts: { needs: 'splice' },
  score: {},
  releases: {},
  files: {},
  links: {},
  notes: {},
  comments: {},
  assistant: {},
  history: {},
}

/**
 * The tab a card opens on when the URL does not say.
 *
 * The mockup opens on its Lyrics tab, because there the header holds the
 * editable fields. Here they live on Overview instead — a header that can be
 * typed into is a header that shifts under the cursor while it saves — so
 * Overview is what a card has to open on, or renaming a work would be behind a
 * tab.
 */
export const DEFAULT_TAB: Tab = 'overview'

export function isTab(value: string | undefined): value is Tab {
  return value !== undefined && (TABS as readonly string[]).includes(value)
}

/** What a work has that decides whether its tabs exist. */
export interface CardFacts {
  storyboard: boolean
  splice: boolean
}

/** The tabs this work draws, in order. */
export function tabsOf(facts: CardFacts): Tab[] {
  return TABS.filter((tab) => {
    const needs = RULES[tab].needs
    return needs === undefined || facts[needs]
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
    releases: counts.releases,
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

/**
 * The tabs a person may make the card open on.
 *
 * Every tab every work has: a default of Scenes on a song would open on
 * nothing. The choice is a machine setting (`cardView`), because where a card
 * opens is a habit of the person, not a fact of the craft.
 */
export const DEFAULT_TAB_CHOICES: readonly Tab[] = TABS.filter(
  (tab) => RULES[tab].needs === undefined,
)
