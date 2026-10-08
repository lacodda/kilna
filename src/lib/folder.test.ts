import { describe, expect, it } from 'vitest'
import type { TFunction } from 'i18next'
import { folderProblem } from '@/lib/folder'

// The key the check answers with, so a test reads which rule spoke.
const t = ((key: string) => key) as unknown as TFunction

describe('a folder template', () => {
  it('names a folder under the media folder from a title and fields', () => {
    for (const template of [
      '',
      '{title}',
      'songs/{title}',
      'songs/{origin.source_id}/clip',
      'songs\\{origin.title} (draft)',
    ]) {
      expect(folderProblem(template, t), template).toBeNull()
    }
  })

  it('is refused where the profile would refuse it, with the reason', () => {
    const cases: [string, string][] = [
      ['/songs/{title}', 'editor.folderAbsolute'],
      ['C:/songs', 'editor.folderAbsolute'],
      ['\\\\server\\share', 'editor.folderAbsolute'],
      ['songs//{title}', 'editor.folderEmptyStep'],
      ['songs/../{title}', 'editor.folderClimbs'],
      ['./{title}', 'editor.folderClimbs'],
      ['songs/{title', 'editor.folderBraces'],
      ['songs/title}', 'editor.folderBraces'],
      ['songs/{a b}', 'editor.folderPlaceholder'],
      ['songs/{}', 'editor.folderPlaceholder'],
      ['songs?/{title}', 'editor.folderForbidden'],
    ]
    for (const [template, reason] of cases) {
      expect(folderProblem(template, t), template).toBe(reason)
    }
  })
})
