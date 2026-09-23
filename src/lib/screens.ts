/**
 * The nav word that names each screen, by the first segment of its address.
 *
 * A map rather than a switch with a default: the default named every new
 * screen "Catalogue" in the title bar (styles in v0.75, notes and comments in
 * v0.76 before this), and nothing failed. `screens.test.ts` holds every route
 * of `App.tsx` to an entry here.
 */
export const SCREEN_NAMES: Readonly<Record<string, string>> = Object.freeze({
  dashboard: 'nav.dashboard',
  catalogue: 'nav.catalogue',
  // An open work belongs to the catalogue, where its trail and back link lead.
  works: 'nav.catalogue',
  calendar: 'nav.calendar',
  notes: 'nav.notes',
  comments: 'nav.comments',
  styles: 'nav.styles',
  journal: 'nav.journal',
  trash: 'nav.trash',
  settings: 'nav.data',
  styleguide: 'nav.styleguide',
})

/** The nav key naming the screen at `pathname`. An address no route has is
 *  sent to the dashboard by the router, so that is what it is named. */
export function screenKey(pathname: string): string {
  return SCREEN_NAMES[pathname.split('/')[1] ?? ''] ?? 'nav.dashboard'
}
