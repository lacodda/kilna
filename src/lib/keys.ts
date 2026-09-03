/**
 * Reading a keystroke as an intent.
 *
 * Every shortcut in the shell arrives at one listener on the window, and the
 * question that listener has to answer is always the same: was this keystroke
 * aimed at the application, or at whatever the person is typing into? Getting
 * that wrong is not a small bug — a `g` that navigates while someone writes a
 * lyric costs them the sentence.
 *
 * So the decision is a pure function over the event's shape rather than a
 * branch buried in the handler. It can then be tested exhaustively, which is
 * the only way to be sure about a rule with this many edges.
 */

/** What a keystroke turned out to mean. */
export type Intent =
  /** Go somewhere. `where` is a route path. */
  | { kind: 'go'; where: string }
  /** Show the list of shortcuts. */
  | { kind: 'help' }
  /** Walk the history, the way a browser's buttons do. */
  | { kind: 'history'; delta: -1 | 1 }

/**
 * Enough of a `KeyboardEvent` to decide. Taking the fields rather than the
 * event keeps the tests free of DOM construction, and keeps this honest about
 * what it actually reads.
 */
export interface Stroke {
  key: string
  ctrlKey: boolean
  metaKey: boolean
  altKey: boolean
  shiftKey: boolean
  /**
   * Whether the keystroke landed somewhere that owns the keyboard.
   *
   * A boolean rather than the element, so the rule stays arithmetic and the
   * tests need no DOM. Answering it is `typing()`'s job, and it is asked once
   * at the edge where the real event arrives.
   */
  typing: boolean
}

/** Where `g` then a letter goes. The letters are the initials of the English
 * screen names, which is what makes them memorable — and they stay put when
 * the interface is read in another language, because a shortcut that moved
 * with the translation would have to be relearned per language. */
export const DESTINATIONS: Readonly<Record<string, string>> = Object.freeze({
  d: '/dashboard',
  c: '/catalogue',
  k: '/calendar',
  j: '/journal',
  t: '/trash',
  s: '/settings',
})

/**
 * Whether a keystroke landed somewhere that owns the keyboard.
 *
 * The one place that touches the DOM, called at the edge where the real event
 * arrives so that everything downstream is arithmetic.
 *
 * Text fields are the obvious case. The two that are not: an element made
 * editable by `contenteditable`, which is not an input at all, and anything
 * inside one — a `<b>` inside an editable paragraph is what `target` reports,
 * and asking only about the element itself would miss it. `closest` answers
 * for the whole ancestry, which is why it is used rather than a tag check.
 */
export function typing(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable) return true
  return target.closest('input, textarea, select, [contenteditable="true"]') !== null
}

/**
 * What a keystroke means on its own — no `g` pressed before it.
 *
 * Returns `null` for anything that is not a shortcut, which is almost every
 * keystroke, and the caller must then leave the event alone. Two rules hold
 * everywhere:
 *
 * - a modifier other than the one the shortcut names disqualifies it, so
 *   `Ctrl+D` (a browser bookmark) is never read as the dashboard;
 * - typing wins, always, except where the shortcut is the way *out* of the
 *   field — which is why `Ctrl+K` is handled in the top bar and not here.
 */
export function readStroke(stroke: Stroke): Intent | null {
  const { key, ctrlKey, metaKey, altKey, shiftKey } = stroke

  // Alt+arrows are the browser's own back and forward, and this application is
  // routed like a browser (ADR 0006), so it answers to them. They work while
  // typing: Alt is not a character, so nothing is being interrupted.
  if (altKey && !ctrlKey && !metaKey && !shiftKey) {
    if (key === 'ArrowLeft') return { kind: 'history', delta: -1 }
    if (key === 'ArrowRight') return { kind: 'history', delta: 1 }
  }

  // Everything below is a bare key, so a field being focused settles it.
  if (ctrlKey || metaKey || altKey) return null
  if (stroke.typing) return null

  // `?` is Shift+/ on most layouts, so the shift is expected rather than
  // disqualifying — the key already carries it.
  if (key === '?') return { kind: 'help' }

  return null
}

/**
 * What the second key of a `g` chord means.
 *
 * Split from `readStroke` because the caller holds the "g was pressed" state
 * and this has no business knowing about it. An unknown letter returns `null`,
 * and the caller drops the chord rather than waiting — a chord that lingers
 * turns the next unrelated keystroke into a navigation.
 */
export function readChord(stroke: Stroke): Intent | null {
  const { key, ctrlKey, metaKey, altKey } = stroke
  if (ctrlKey || metaKey || altKey) return null
  if (stroke.typing) return null

  const where = DESTINATIONS[key.toLowerCase()]
  return where === undefined ? null : { kind: 'go', where }
}

/**
 * Whether this keystroke opens a `g` chord.
 *
 * Shift is excluded rather than ignored: `G` is not `g`, and on a keyboard
 * where the difference is deliberate it should not silently mean the same.
 */
export function opensChord(stroke: Stroke): boolean {
  const { key, ctrlKey, metaKey, altKey, shiftKey } = stroke
  if (ctrlKey || metaKey || altKey || shiftKey) return false
  if (stroke.typing) return false
  return key === 'g'
}
