import { describe, expect, it } from 'vitest'
import type { Asset, FolderFile } from '@/lib/api/types'
import {
  arrange,
  categoryOf,
  DEFAULT_LOOK,
  entryOfAsset,
  entryOfFolderFile,
  loadLook,
  saveLook,
  type FileEntry,
  type FilesLook,
} from '@/lib/fileList'

/*
 * A work's files as a list (FR-14): the four categories, the two ways of
 * gathering them, the sorts, and the look a person keeps.
 */

const entry = (name: string, extra: Partial<FileEntry> = {}): FileEntry => ({
  key: name,
  name,
  path: `C:/media/${name}`,
  category: categoryOf(name),
  place: '',
  roles: [],
  size: 0,
  ...extra,
})

const look = (extra: Partial<FilesLook> = {}): FilesLook => ({ ...DEFAULT_LOOK, ...extra })
const names = (entries: FileEntry[]) => entries.map((one) => one.name)

describe('a file', () => {
  it('is a picture, a sound, a clip or something else, by its ending', () => {
    for (const [name, category] of [
      ['cover.PNG', 'image'],
      ['logo.svg', 'image'],
      ['scan.bmp', 'image'],
      ['mix.flac', 'audio'],
      ['take.opus', 'audio'],
      ['clip.mkv', 'video'],
      ['old.avi', 'video'],
      ['lyrics.txt', 'other'],
      ['subs.srt', 'other'],
      ['brief.pdf', 'other'],
      ['stems.zip', 'other'],
      ['README', 'other'],
    ] as const) {
      expect(categoryOf(name), name).toBe(category)
    }
  })

  it('attached carries what it is for, its size and when it changed', () => {
    const asset = {
      id: 'a1',
      kind: 'cover',
      path: 'C:/ws/media/a1.png',
      original_name: 'Final cover.png',
      size_bytes: 2048,
      modified_at: '2026-11-01T10:00:00Z',
    } as Asset

    expect(entryOfAsset(asset, 'a1')).toEqual({
      key: 'a1',
      name: 'Final cover.png',
      path: 'C:/ws/media/a1.png',
      category: 'image',
      place: 'cover',
      roles: ['cover'],
      size: 2048,
      modified: '2026-11-01T10:00:00Z',
    })
    // An older cover is a file like any other; a candidate says it is one.
    expect(entryOfAsset(asset, 'a2').roles).toEqual([])
    expect(entryOfAsset({ ...asset, kind: 'candidate' }, 'a2').roles).toEqual(['candidate'])
    // A kind invented later is filed with the rest.
    expect(entryOfAsset({ ...asset, kind: 'stem' }, undefined).place).toBe('other')
    // The category is read off the name the file came under: the stored
    // name is the workspace's own.
    expect(
      entryOfAsset({ ...asset, path: 'C:/ws/media/a1', original_name: 'mix.wav' }, undefined)
        .category,
    ).toBe('audio')
  })

  it('on disk is named by its last step and placed by the folder it is in', () => {
    const file: FolderFile = {
      path: 'C:/media/songs/x/scenes/take 2/v1.mp4',
      relative: 'scenes/take 2/v1.mp4',
      size: 10,
      modified: '2026-11-02T10:00:00Z',
    }
    const at = entryOfFolderFile(file, file.path)
    expect([at.name, at.place, at.category, at.roles]).toEqual([
      'v1.mp4',
      'scenes/take 2',
      'video',
      ['finished'],
    ])
    const top = entryOfFolderFile({ ...file, relative: 'final.mp4' }, undefined)
    expect([top.name, top.place, top.roles]).toEqual(['final.mp4', '', []])
  })
})

describe('the files arranged', () => {
  const files = [
    entry('take 10.mp4', { place: 'scenes', size: 30, modified: '2026-11-03T00:00:00Z' }),
    entry('take 2.mp4', { place: 'scenes', size: 50, modified: '2026-11-01T00:00:00Z' }),
    entry('Cover.png', { size: 20, modified: '2026-11-02T00:00:00Z' }),
    entry('mix.wav', { place: 'raw', size: 900 }),
    entry('notes.txt', { size: 1, modified: '2026-10-01T00:00:00Z' }),
  ]

  it('are gathered by what they are, in the order pictures, sound, clips, the rest', () => {
    const groups = arrange(files, look())
    expect(groups.map((group) => group.key)).toEqual(['image', 'audio', 'video', 'other'])
    // By name the way names are read: 2 before 10.
    expect(names(groups[2]!.entries)).toEqual(['take 2.mp4', 'take 10.mp4'])
  })

  it("are gathered by where they lie, the work's own folder first", () => {
    const groups = arrange(files, look({ group: 'place' }))
    expect(groups.map((group) => group.key)).toEqual(['', 'raw', 'scenes'])
    expect(names(groups[0]!.entries)).toEqual(['Cover.png', 'notes.txt'])
  })

  it('keep the order given for places, and show a place nobody ordered after it', () => {
    const attached = [
      entry('b.png', { place: 'frame' }),
      entry('a.png', { place: 'cover' }),
      entry('c.png', { place: 'something new' }),
    ]
    const groups = arrange(attached, look({ group: 'place' }), ['cover', 'candidate', 'frame'])
    expect(groups.map((group) => group.key)).toEqual(['cover', 'frame', 'something new'])
  })

  it('show only the ticked categories, and all of them when none is ticked', () => {
    expect(arrange(files, look({ only: ['video', 'audio'] })).map((group) => group.key)).toEqual([
      'audio',
      'video',
    ])
    expect(arrange(files, look({ only: [] })).flatMap((group) => group.entries)).toHaveLength(5)
    // A category with nothing in it leaves no heading behind.
    expect(arrange([entry('a.png')], look()).map((group) => group.key)).toEqual(['image'])
  })

  it('sort by size, by date and by category, either way', () => {
    const flat = (extra: Partial<FilesLook>) =>
      // One group, so the order read is the sort's alone.
      names(
        arrange(
          files.map((one) => ({ ...one, place: '' })),
          look({ group: 'place', ...extra }),
        )[0]!.entries,
      )

    expect(flat({ sort: 'size' })).toEqual([
      'notes.txt',
      'Cover.png',
      'take 10.mp4',
      'take 2.mp4',
      'mix.wav',
    ])
    expect(flat({ sort: 'size', descending: true })[0]).toBe('mix.wav')
    // A file whose time is not known stands last, whichever way the rest go.
    expect(flat({ sort: 'modified' })).toEqual([
      'notes.txt',
      'take 2.mp4',
      'Cover.png',
      'take 10.mp4',
      'mix.wav',
    ])
    expect(flat({ sort: 'modified', descending: true })).toEqual([
      'take 10.mp4',
      'Cover.png',
      'take 2.mp4',
      'notes.txt',
      'mix.wav',
    ])
    expect(flat({ sort: 'category' })).toEqual([
      'Cover.png',
      'mix.wav',
      'take 2.mp4',
      'take 10.mp4',
      'notes.txt',
    ])
    expect(flat({ sort: 'name', descending: true })[0]).toBe('take 10.mp4')
  })
})

describe('the look', () => {
  const store = (held: Record<string, string> = {}) => ({
    getItem: (key: string) => held[key] ?? null,
    setItem: (key: string, value: string) => {
      held[key] = value
    },
  })

  it('is kept and read back', () => {
    const held = store()
    const mine = look({
      view: 'table',
      group: 'place',
      sort: 'size',
      descending: true,
      only: ['video'],
    })
    saveLook(mine, held)
    expect(loadLook(held)).toEqual(mine)
  })

  it('falls back to the default for anything it cannot read', () => {
    expect(loadLook(store())).toEqual(DEFAULT_LOOK)
    expect(loadLook(store({ 'kilna.files.look': 'not json' }))).toEqual(DEFAULT_LOOK)
    expect(
      loadLook(
        store({
          'kilna.files.look': JSON.stringify({
            view: 'carousel',
            sort: 7,
            only: ['video', 'spam'],
          }),
        }),
      ),
    ).toEqual({ ...DEFAULT_LOOK, only: ['video'] })
  })
})
