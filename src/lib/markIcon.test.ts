import { readFileSync, readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { MARK_ICON_NAMES } from './markIcon'

// The eleven marks every shipped profile but the blank one carries (v0.139,
// FR-12 12.1), each with a glyph the screen knows and a word in both
// languages: a glyph renamed here and not there falls back to the tag in
// silence, which is the reason this test exists.
const ELEVEN = [
  'working',
  'unsure',
  'favourite',
  'priority',
  'next-up',
  'needs-polish',
  'experiment',
  'redo',
  'waiting',
  'flagship',
  'for-shorts',
]

describe('the shipped marks', () => {
  const dir = fileURLToPath(new URL('../../src-tauri/profiles/', import.meta.url))

  it('are the same eleven, named and drawn, in every profile but the blank one', () => {
    const files = readdirSync(dir).filter((name) => name.endsWith('.json') && name !== 'blank.json')
    expect(files.length).toBeGreaterThan(4)
    for (const file of files) {
      type Shipped = { key: string; label: { en?: string; ru?: string }; icon?: string }
      const profile = JSON.parse(readFileSync(dir + file, 'utf8')) as {
        config: { marks?: Shipped[] }
      }
      const marks = profile.config.marks ?? []
      expect([file, marks.map((mark) => mark.key)]).toEqual([file, ELEVEN])
      for (const mark of marks) {
        expect([file, mark.key, MARK_ICON_NAMES.includes(mark.icon ?? '')]).toEqual([
          file,
          mark.key,
          true,
        ])
        expect([file, mark.key, Boolean(mark.label.en), Boolean(mark.label.ru)]).toEqual([
          file,
          mark.key,
          true,
          true,
        ])
      }
    }
  })
})
