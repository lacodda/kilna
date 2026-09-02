import { describe, expect, it } from 'vitest'
import { cn } from '@/lib/utils'

/*
 * What `cn` is allowed to throw away.
 *
 * `tailwind-merge` resolves conflicts by keeping the last class of a group, and
 * it is right to: that is how a caller overrides a component. The trap is that
 * `position` is one group, so a wrapper adding `relative` for its own absolutely
 * positioned child silently removes the `fixed` the overlay is built on.
 *
 * That is not a hypothesis. kilna's dialog wrapper added `relative` so its close
 * button could sit in the corner, `fixed` was dropped, and every dialog started
 * measuring `top: 50%` against the document instead of the window - which on a
 * long screen put it below the fold. It took three wrong diagnoses to find,
 * because the class is present in the stylesheet and absent only after merging.
 */
describe('what survives a class merge', () => {
  // The classes the dialog popup is built from, in the order the component
  // emits them. Shortened to the ones that matter for positioning.
  const POPUP = 'fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 max-h-[calc(100dvh-2rem)]'

  it('keeps an overlay fixed when a caller adds classes of its own', () => {
    const merged = cn(POPUP, '[z-index:var(--z-modal)]', 'p-4')
    expect(merged, 'an overlay that is not fixed measures against the document').toMatch(
      /\bfixed\b/,
    )
  })

  // The specific mistake, kept by name so making it again fails here rather
  // than on someone's screen.
  it('loses fixed when something adds relative — which is why nothing may', () => {
    const merged = cn(POPUP, 'relative')
    expect(merged, 'tailwind-merge stopped treating position as one group').not.toMatch(
      /\bfixed\b/,
    )
  })

  // An overlay is positioned already; a descendant needs no `relative` on it.
  // This is the rule the test above exists to enforce, stated where it can be
  // read: if a wrapper needs one, the wrapper is the wrong place for it.
  it('positions an absolute child from a fixed parent, with no relative needed', () => {
    const merged = cn(POPUP)
    expect(merged).toMatch(/\bfixed\b/)
    expect(merged).not.toMatch(/\brelative\b/)
  })
})
