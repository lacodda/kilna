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
 * here: Base UI makes no element, and the theme carries the rule.
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

  it('comes from the theme instead', () => {
    const css = readFileSync('node_modules/dowel-ui/dist/theme.css', 'utf8')
    expect(css).toMatch(/\.base-ui-disable-scrollbar\s*\{\s*scrollbar-width:\s*none;?\s*\}/)
    expect(css).toMatch(
      /\.base-ui-disable-scrollbar::-webkit-scrollbar\s*\{\s*display:\s*none;?\s*\}/,
    )
  })
})

/*
 * Without `connect-src` for Tauri's own protocol the window's IPC fell back to
 * postMessage - "IPC custom protocol failed" in the console, every command
 * and channel on the slow path, and a raw response arriving as an array of
 * numbers (wishes 3936, 5792). The policy is the configuration's; this holds
 * the line in it.
 */
describe("the window's policy", () => {
  it('lets the IPC protocol through, and nothing else from outside', () => {
    const conf = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')) as {
      app: { security: { csp: string } }
    }
    const connect = conf.app.security.csp
      .split(';')
      .map((directive) => directive.trim())
      .find((directive) => directive.startsWith('connect-src'))
    expect(connect?.split(/\s+/).sort()).toEqual(
      ['connect-src', "'self'", 'ipc:', 'http://ipc.localhost'].sort(),
    )
  })
})
