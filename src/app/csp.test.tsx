import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { readFileSync } from 'node:fs'
import { Providers } from '@/app/Providers'
import { Scroll } from '@/components/frame'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend } from '@/test/backend'

/*
 * The window's CSP refuses a `<style>` element made at run time: Tauri adds
 * hashes for the bundled code to `style-src`, and a directive with a hash in
 * it ignores `'unsafe-inline'`. Base UI hides a scroll area's own bars with a
 * rule it inserts that way, so in the window every overlay scroller drew a
 * classic bar beside its overlay one - found by measuring the live window,
 * invisible to jsdom, which has no CSP. The two halves of the fix are held
 * here: Base UI makes no element, and the bundle carries the rule.
 */
describe("Base UI's one stylesheet", () => {
  it('is not made at run time', () => {
    mockBackend({ active_tasks: () => [] })
    render(
      <Providers client={createQueryClient()}>
        <Scroll label="Lines">
          <p>A line</p>
        </Scroll>
      </Providers>,
    )
    expect(screen.getByRole('group', { name: 'Lines' })).toHaveClass('base-ui-disable-scrollbar')
    expect(document.querySelector('style[data-href="base-ui-disable-scrollbar"]')).toBeNull()
  })

  it('comes from the bundle instead', () => {
    const css = readFileSync('src/styles.css', 'utf8').replace(/\/\*[\s\S]*?\*\//g, '')
    expect(css).toMatch(/\.base-ui-disable-scrollbar\s*\{\s*scrollbar-width:\s*none;?\s*\}/)
    expect(css).toMatch(
      /\.base-ui-disable-scrollbar::-webkit-scrollbar\s*\{\s*display:\s*none;?\s*\}/,
    )
  })
})
