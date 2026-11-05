import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'

/*
 * The overlay bar takes two halves, and neither shows if the other is gone: the
 * window asks its engine for the bar, and the page measures it and tells the
 * theme to stand aside. Lose either and the window quietly goes back to a ten
 * pixel strip, which no test that renders in jsdom can see, since jsdom draws no
 * bars at all. What the bar does on screen is measured in the window.
 */
describe('the overlay scrollbar', () => {
  const asked = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).app.windows[0]
  const args: string[] = asked.additionalBrowserArgs.split(' ')

  it('is asked of the engine', () => {
    expect(args).toContain('--enable-features=OverlayScrollbar')
  })

  it("keeps the list that key would otherwise take from Tauri's defaults", () => {
    const off = args.find((arg) => arg.startsWith('--disable-features='))
    expect(off).toContain('msWebOOUI')
    expect(off).toContain('msPdfOOUI')
    expect(off).toContain('msSmartScreenProtection')
  })

  it('is measured before the first render', () => {
    const main = readFileSync('src/main.tsx', 'utf8')
    const measured = main.indexOf('overlayScrollbars()')
    expect(measured).toBeGreaterThan(-1)
    expect(measured).toBeLessThan(main.indexOf('.render('))
  })
})
