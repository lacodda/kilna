/**
 * Keep the window's own page where it is.
 *
 * The page never scrolls: every screen scrolls inside itself, and `html` and
 * `body` are sealed against it in the stylesheet. `overflow: hidden` stops a
 * wheel and a scrollbar, not the browser itself - focus moving into a popup
 * anchored to something off the edge of the window scrolls the page to show
 * it, and the whole window slid sideways by the width of a table (found on a
 * catalogue row scrolled out of sight, v0.119). Nothing a person can reach
 * with a pointer does it today, and a keyboard or a script can; whatever does,
 * the page is put back the moment it moves.
 */
export function initSealedRoot(): void {
  document.addEventListener(
    'scroll',
    (event) => {
      // Scrolling inside a screen is a screen's business; only the page's
      // own scroll is put back.
      if (event.target !== document) return
      for (const root of [document.documentElement, document.body]) {
        if (root.scrollLeft !== 0) root.scrollLeft = 0
        if (root.scrollTop !== 0) root.scrollTop = 0
      }
    },
    { capture: true, passive: true },
  )
}
