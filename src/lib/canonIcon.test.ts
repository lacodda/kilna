import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { CANON_ICON_NAMES } from '@/lib/canonIcon'

describe('card glyphs', () => {
  it('are all named in the profile reference, which is where an author looks', () => {
    const reference = readFileSync(
      new URL('../../docs/src/content/docs/reference/profile-document.md', import.meta.url),
      'utf8',
    )
    const row = reference
      .split('\n')
      .find((line) => line.startsWith('| `icon` | string, optional | The glyph the kind is drawn'))
    expect(row).toBeDefined()
    for (const name of CANON_ICON_NAMES) expect(row, name).toContain(`\`${name}\``)
  })
})
