import { describe, expect, it } from 'vitest'
import type { ReleaseKind } from '@/lib/api/types'
import { studio } from '@/test/workspace'
import { readLink } from './releaseLink'

const doorsOf = (kind: string): ReleaseKind[] =>
  studio().profile.config.work_kinds.find((entry) => entry.key === kind)?.release_kinds ?? []

const door = (key: string, links: string[]): ReleaseKind => ({
  key,
  label: key,
  requires: [],
  links,
})

describe('readLink, where it went', () => {
  it("tells an audio release's upload from its streaming page by the shipped patterns", () => {
    const audio = doorsOf('audio')
    expect(readLink('https://www.youtube.com/watch?v=abc', audio).kind).toBe('youtube')
    expect(readLink('https://youtu.be/abc', audio).kind).toBe('youtube')
    expect(readLink('https://open.spotify.com/track/1', audio).kind).toBe('streaming')
    // A longer host is a closer match: YouTube Music is a streaming page.
    expect(readLink('https://music.youtube.com/watch?v=abc', audio).kind).toBe('streaming')
  })

  it('says nothing when two doors match equally well', () => {
    // A premiere and an upload are both a watch page.
    expect(readLink('https://www.youtube.com/watch?v=abc', doorsOf('video')).kind).toBeNull()
  })

  it('lets the longer path win on one host', () => {
    const doors = [door('video', ['youtube.com']), door('short', ['youtube.com/shorts/'])]
    expect(readLink('https://m.youtube.com/shorts/abc', doors).kind).toBe('short')
    expect(readLink('https://youtube.com/@channel', doors).kind).toBe('video')
  })

  it('holds a host and its subdomains, never a host that only ends the same', () => {
    const doors = [door('newsletter', ['substack.com'])]
    expect(readLink('https://someone.substack.com/p/post', doors).kind).toBe('newsletter')
    expect(readLink('https://notsubstack.com/p/post', doors).kind).toBeNull()
  })

  it('reads nothing from what is not a web link', () => {
    expect(readLink('youtube.com/watch?v=abc', doorsOf('audio'))).toEqual({ kind: null, day: null })
    expect(readLink('file:///C:/video.mp4', doorsOf('audio'))).toEqual({ kind: null, day: null })
  })
})

describe('readLink, when it went', () => {
  it('reads a day the path carries, in either spelling', () => {
    expect(readLink('https://blog.example/2026/10/14/the-post', []).day).toBe('2026-10-14')
    expect(readLink('https://example.com/posts/2026-10-14-the-post', []).day).toBe('2026-10-14')
  })

  it('reads no day where there is none, or where the numbers are no day', () => {
    expect(readLink('https://youtu.be/abc', []).day).toBeNull()
    expect(readLink('https://example.com/2026/02/31/no', []).day).toBeNull()
    expect(readLink('https://example.com/item/20261014', []).day).toBeNull()
  })
})
