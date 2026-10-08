import { describe, expect, it, vi } from 'vitest'
import type { Asset } from '@/lib/api/types'
import { insertAt, isPicturePath, pictureLine, storedNameOf, urlOf } from '@/lib/notePictures'

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://localhost/${encodeURIComponent(path)}`,
  invoke: vi.fn(),
}))

function asset(fields: Partial<Asset>): Asset {
  return {
    id: 'a1',
    profile_id: 'p',
    work_id: null,
    release_id: null,
    kind: 'inline',
    path: 'C:\\ws\\media\\a1.png',
    label: null,
    original_name: 'board.png',
    style_brick_id: null,
    note_id: 'n1',
    canon_fact_id: null,
    created_at: '2026-10-08T10:00:00Z',
    ...fields,
  }
}

describe("a picture in a note's body", () => {
  it('is written as a line naming the file it was stored as', () => {
    expect(pictureLine(asset({}))).toBe('![board](media/a1.png)')
    // What a clipboard calls every picture says nothing: no words for it.
    expect(pictureLine(asset({ original_name: 'image.png' }))).toBe('![](media/a1.png)')
    expect(pictureLine(asset({ original_name: 'a [b].png' }))).toBe('![a b](media/a1.png)')
  })

  it('is read back as a file of the workspace, and nothing else is', () => {
    expect(storedNameOf('media/a1.png')).toBe('a1.png')
    expect(storedNameOf('media/../kilna.db')).toBeNull()
    expect(storedNameOf('media/sub/a1.png')).toBeNull()
    expect(storedNameOf('media/')).toBeNull()
    expect(storedNameOf('https://example.com/media/a1.png')).toBeNull()
    expect(urlOf('C:\\ws\\media', 'media/a1.png')).toBe(
      `asset://localhost/${encodeURIComponent('C:\\ws\\media\\a1.png')}`,
    )
    expect(urlOf('/home/me/ws/media/', 'media/a1.png')).toBe(
      `asset://localhost/${encodeURIComponent('/home/me/ws/media/a1.png')}`,
    )
    expect(urlOf('C:\\ws\\media', 'elsewhere.png')).toBeNull()
  })

  it('stands on a line of its own wherever the caret was', () => {
    expect(insertAt('', 0, 'P')).toBe('P')
    expect(insertAt('ab', 1, 'P')).toBe('a\nP\nb')
    expect(insertAt('a\n', 2, 'P')).toBe('a\nP')
    expect(insertAt('a\nb', 2, 'P')).toBe('a\nP\nb')
    expect(insertAt('ab', 99, 'P')).toBe('ab\nP')
  })

  it('is taken from a dropped file only when the file is a picture', () => {
    expect(isPicturePath('D:\\shots\\board.PNG')).toBe(true)
    expect(isPicturePath('/shots/take.mp4')).toBe(false)
    expect(isPicturePath('/shots/.png')).toBe(false)
  })
})
