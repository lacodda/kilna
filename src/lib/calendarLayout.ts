/**
 * How much of the screen the calendar gets.
 *
 * `queue` keeps the waiting releases beside the month, which is the shape for
 * planning: pick something off the list, give it a date. `full` gives the whole
 * width to the month, which is the shape for reading one — a day is half again
 * as wide, and a chip's title stops being the first thing to be truncated.
 */
export type CalendarLayout = 'queue' | 'full'

export const DEFAULT_LAYOUT: CalendarLayout = 'queue'

const LAYOUT_KEY = 'kilna.calendar.layout'

/**
 * Just enough of `localStorage` to be handed a fake one — the same shape the
 * catalogue's sort uses, and for the same reason: the test runner has no DOM.
 */
export interface LayoutStore {
  getItem: (key: string) => string | null
  setItem: (key: string, value: string) => void
}

function isLayout(value: unknown): value is CalendarLayout {
  return value === 'queue' || value === 'full'
}

/**
 * The layout survives a restart, the kind filter deliberately does not.
 *
 * Which shape the calendar takes is how a person prefers to work — the same
 * standing choice as the catalogue's sort. The filter above the grid is about
 * the next thing they are doing, and finding the month still hiding most of
 * itself a day later would read as data loss rather than as a setting.
 */
export function loadLayout(store: LayoutStore = localStorage): CalendarLayout {
  try {
    const raw = store.getItem(LAYOUT_KEY)
    return isLayout(raw) ? raw : DEFAULT_LAYOUT
  } catch {
    // A blocked or unreadable store is not worth a broken screen.
    return DEFAULT_LAYOUT
  }
}

export function saveLayout(layout: CalendarLayout, store: LayoutStore = localStorage): void {
  try {
    store.setItem(LAYOUT_KEY, layout)
  } catch {
    // Storage full or blocked: the calendar still switches, it just forgets.
  }
}

/** The other one — for a control that toggles rather than chooses. */
export function otherLayout(layout: CalendarLayout): CalendarLayout {
  return layout === 'queue' ? 'full' : 'queue'
}
