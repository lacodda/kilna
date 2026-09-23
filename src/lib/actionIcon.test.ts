import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { ACTION_ICON_NAMES } from '@/lib/actionIcon'

describe('action glyphs', () => {
  it('are all named in the profile reference, which is where an author looks', () => {
    const reference = readFileSync(
      new URL('../../docs/src/content/docs/reference/profile-document.md', import.meta.url),
      'utf8',
    )
    const listed = reference.slice(reference.indexOf('The names `icon` accepts:'))
    const sentence = listed.slice(0, listed.indexOf('\n\n'))
    expect(sentence.length).toBeGreaterThan(20)
    for (const name of ACTION_ICON_NAMES) expect(sentence, name).toContain(`\`${name}\``)
  })
})
