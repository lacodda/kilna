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
   * Two columns, each scrolling on its own: a list on the left, what is
   * picked from it on the right. Scrolling twenty revisions must not move the
   * text being read. The card gives such a tab its height; every other tab is
   * a page that scrolls within the box.
   */
  held: boolean
  /**
   * What a work must have for the tab to exist: a storyboard is a fact of the
   * kind - a song has none - and a splice a fact of the work, whether it was
   * cut out of another. Without it the tab is not drawn, and an address
   * naming it goes to the default tab.
   */
  needs?: 'storyboard' | 'splice'
}

const RULES: Readonly<Record<Tab, TabRules>> = {
  overview: { held: false },
  versions: { held: true },
  scenes: { held: false, needs: 'storyboard' },
  cuts: { held: false, needs: 'splice' },
  score: { held: true },
  releases: { held: false },
  files: { held: false },
  links: { held: false },
  notes: { held: false },
  comments: { held: true },
  assistant: { held: false },
  history: { held: false },
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

export const isHeld = (tab: Tab): boolean => RULES[tab].held

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
