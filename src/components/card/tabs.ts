/**
 * The card's tabs, in the order the mockup puts them.
 *
 * One list, used by the tab bar, by the router and by the "is this a real tab"
 * check — a second copy would be a second chance for a URL to name a tab the bar
 * does not draw.
 */
export const TABS = ['overview', 'lyrics', 'score', 'releases', 'notes', 'assistant', 'history'] as const

export type Tab = (typeof TABS)[number]

/**
 * The tab a card opens on when the URL does not say.
 *
 * The mockup opens on Lyrics, because there the header holds the editable
 * fields. Here they live on Overview instead — a header that can be typed into
 * is a header that shifts under the cursor while it saves — so Overview is what
 * a card has to open on, or renaming a work would be behind a tab.
 */
export const DEFAULT_TAB: Tab = 'overview'

export function isTab(value: string | undefined): value is Tab {
  return value !== undefined && (TABS as readonly string[]).includes(value)
}
