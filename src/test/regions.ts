import { expect } from 'vitest'

/*
 * Every region that scrolls is reached by a height, and none sits in another.
 *
 * A region scrolls against the height its box is given; a box between it and
 * the held screen that takes its content's height instead - a bare `<div>`, the
 * wrapper a query's states are drawn in - hands it none, and the region grows
 * with what it holds and is cut off at the window's edge with no bar to reach
 * the rest. The Frame tab's prompts were drawn that way until v0.90.1. So
 * every box on the way up carries `min-h-0` (a flex or grid cell that gives
 * way), or is a bounded box (`h-*`, `max-h-*`, `size-*`) - which is where a
 * region inside another one's content stops: a widget held to its board.
 *
 * And a scroller of the browser's own (`overflow-*-auto`) never stands inside
 * another region. The theme stops every such box from passing a gesture on
 * (`styles.css`), so one nested and at rest - too short to scroll - takes the
 * wheel and does nothing with it: the palette's list did that inside its own
 * scrolling column until v0.90.1, and only the bar moved it.
 */
const NATIVE = /(^|\s)overflow-(x-|y-)?(auto|scroll)(\s|$)/
const isRegion = (el: Element) =>
  /-viewport$/.test(el.getAttribute('data-id') ?? '') || NATIVE.test(el.getAttribute('class') ?? '')
const BOUNDED = /(^|\s)(min-h-0|(max-)?h-|size-)/

export function regionsHaveHeight(box: HTMLElement, what: string) {
  for (const viewport of box.querySelectorAll('[data-id$="-viewport"]')) {
    const root = viewport.parentElement!
    const name = viewport.getAttribute('aria-label') ?? '?'
    // The region's own box takes its size from its class; what is above it
    // must hand a height down to it.
    for (let up = root.parentElement; up !== null && up !== box; up = up.parentElement) {
      if (isRegion(up)) break
      const classes = up.getAttribute('class') ?? ''
      // A box sized by its parent's `*:` - the wrapper `Loaded fill` hands on.
      const handed = /(^|\s)\*:min-h-0(\s|$)/.test(up.parentElement?.getAttribute('class') ?? '')
      expect(
        handed || BOUNDED.test(classes),
        `${what}: the region "${name}" is under a box that takes its content's height: <${up.tagName.toLowerCase()} class="${classes}">`,
      ).toBe(true)
      if (/(^|\s)((max-)?h-|size-)/.test(classes)) break
    }
  }
  noScrollerInAnother(box, what)
}

/** No scroller of the browser's own stands inside another region of `box`. */
export function noScrollerInAnother(box: HTMLElement, what: string) {
  for (const native of box.querySelectorAll('[class*="overflow-"]')) {
    if (!NATIVE.test(native.getAttribute('class') ?? '')) continue
    let up = native.parentElement
    while (up !== null && up !== box && !isRegion(up)) up = up.parentElement
    expect(
      up === null || up === box,
      `${what}: a scroller of the browser's own sits in another region and eats its wheel: <${native.tagName.toLowerCase()} class="${native.getAttribute('class')}">`,
    ).toBe(true)
  }
}
