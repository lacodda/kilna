import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { DOWEL_SCALE, FLOOR_BANDS, POPUP_FLOORS, type Rung } from './layers'

const rungs = Object.keys(POPUP_FLOORS) as Rung[]

describe('the stacking floors a popup inside an overlay lands on', () => {
  it('puts every floor above the overlay it belongs to', () => {
    // The defect this exists for: a date popover opened from the release
    // editor drew UNDER it, because the popover was a 40 and the dialog a 60.
    for (const rung of rungs) {
      expect(POPUP_FLOORS[rung]).toBeGreaterThan(FLOOR_BANDS[rung].above)
    }
  })

  it('keeps every floor under the next tier up', () => {
    // The other direction, and the reason a floor is not simply a large
    // number: a popup that cleared everything would cover the command palette
    // and the toasts, which are meant to cover it.
    for (const rung of rungs) {
      expect(POPUP_FLOORS[rung]).toBeLessThan(FLOOR_BANDS[rung].below)
    }
  })

  it('reads the same numbers dowel states in its stylesheet', () => {
    // The mirror above is a copy, and a copy is a claim about someone else's
    // file. dowel renumbering its scale must fail here rather than in a
    // screenshot nobody takes: a `--z-modal` raised to 62 would leave the
    // modal-popup floor of 61 underneath the dialog it is supposed to clear.
    const require = createRequire(import.meta.url)
    const css = readFileSync(require.resolve('dowel-ui/theme.css'), 'utf8')
    for (const [name, value] of Object.entries(DOWEL_SCALE)) {
      const match = new RegExp(String.raw`--z-${name}:\s*(\d+)`).exec(css)
      expect(match, `dowel no longer defines --z-${name}`).not.toBeNull()
      expect(Number(match?.[1]), `--z-${name} moved in dowel`).toBe(value)
    }
  })
})
