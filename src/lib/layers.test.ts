import { describe, expect, it } from 'vitest'
import { readdirSync, readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { DOWEL_SCALE, FLOOR_BANDS, POPUP_FLOORS, STAGE_LAYER, type Rung } from './layers'

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

  it('puts the full-screen stage over the page and under its own popups', () => {
    // The defect this one exists for: the stage was a raw Tailwind `z-50`
    // beside the ladder, and the compare menu - a `--z-menu`, a 30 - opened
    // underneath it. The button lit up and nothing appeared.
    //
    // Both directions matter. The stage has to cover the page it is laid over,
    // and it has to stay UNDER what is opened from inside it, or the same bug
    // returns with the menu and the stage swapped.
    expect(STAGE_LAYER).toBeGreaterThan(DOWEL_SCALE.menu)
    expect(STAGE_LAYER).toBeGreaterThan(DOWEL_SCALE.floating)
    expect(POPUP_FLOORS['stage-popup']).toBeGreaterThan(STAGE_LAYER)
  })

  it('gives every overlay a floor of its own, in the overlays own order', () => {
    // A floor per overlay, and the floors in the same order as the overlays
    // they belong to: a popup opened on the stage must not cover a dialog,
    // which is above the stage and may be opened over it.
    expect(POPUP_FLOORS['stage-popup']).toBeLessThan(POPUP_FLOORS['modal-popup'])
    expect(POPUP_FLOORS['modal-popup']).toBeLessThan(POPUP_FLOORS['palette-popup'])
  })

  it('has every portalling primitive ask for the raised host', () => {
    // The numbers above were all correct while the bug was live, which is why
    // this reads the source instead.
    //
    // `LayerProvider` raises a floor and offers a host; a popup only lands on
    // it if it passes that host to its `Portal`. `MenuPopup` did not, so the
    // compare-versions menu opened at 30 under a stage at 50 and the button
    // read as dead - and five more primitives were one call site away from
    // the same thing. Which z-index variable a primitive happens to read is
    // not something its caller can know, so the asking belongs here, and a
    // new primitive that portals must not be able to forget it.
    //
    // fileURLToPath, not URL.pathname: on Windows the latter yields `/C:/...`.
    const dir = fileURLToPath(new URL('../components/ui/', import.meta.url))
    const offenders: string[] = []

    for (const file of readdirSync(dir).filter((name) => name.endsWith('.tsx'))) {
      const source = readFileSync(new URL(file, new URL('../components/ui/', import.meta.url)), 'utf8')
      if (!source.includes('.Portal')) continue
      // A dialog, a drawer, a toast and the palette ARE the overlay: they open
      // the floor rather than stand on one, so they portal to the body on
      // purpose. Everything else opens INSIDE something and must ask.
      if (/^(dialog|drawer|toast|command-palette)\.tsx$/.test(file)) continue
      if (!source.includes('usePopupContainer')) offenders.push(file)
    }

    expect(offenders, 'these portal to the body and will draw under a dialog or the stage').toEqual(
      [],
    )
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
