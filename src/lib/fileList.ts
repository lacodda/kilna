import type { Asset, FolderFile } from '@/lib/api/types'
import { extensionOf } from '@/lib/media'

/*
 * A work's files as a list a person can look through (FR-14, v0.121): what
 * each file is, where it lies, and the ways the Files tab arranges them.
 *
 * Two kinds of file meet here - one copied into the workspace and attached to
 * the work, one found in the work's folder on disk - and both are drawn the
 * same way, so they are one shape: an entry. What tells them apart is where
 * they lie (`place`), not how they look.
 *
 * Until v0.121 the tab drew tiles and nothing else, grouped by what a file was
 * for. That was right for three pictures and wrong for a folder of renders:
 * forty takes are found by their date or their size, in a list, not by
 * looking at forty thumbnails of the same shot.
 */

/** What a file is, by its ending: the four a person sorts files into. */
export const CATEGORIES = ['image', 'audio', 'video', 'other'] as const
export type FileCategory = (typeof CATEGORIES)[number]

const IMAGES = ['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif', 'svg', 'bmp']
const AUDIO = ['mp3', 'wav', 'flac', 'm4a', 'aac', 'ogg', 'oga', 'opus']
const VIDEO = ['mp4', 'webm', 'mov', 'm4v', 'mkv', 'avi']

/** The category of a path: everything that is not a picture, a sound or a
 *  clip - a text, subtitles, a PDF, an archive - is "other". */
export function categoryOf(path: string): FileCategory {
  const ending = extensionOf(path)
  if (IMAGES.includes(ending)) return 'image'
  if (AUDIO.includes(ending)) return 'audio'
  if (VIDEO.includes(ending)) return 'video'
  return 'other'
}

/** What a file is to its work, beyond being one of its files. */
export type FileRole = 'cover' | 'candidate' | 'finished'

/** One file, as the tab draws it. */
export interface FileEntry {
  /** Unique among the entries drawn together. */
  key: string
  name: string
  path: string
  category: FileCategory
  /** Where it lies: what an attached file is for (`cover`, `frame`, ...,
   *  `other`), or the folder a file on disk is in, `''` for the work's own. */
  place: string
  roles: FileRole[]
  size: number
  /** When it last changed, as RFC 3339, when the system says. */
  modified?: string
}

/** The kinds of attached file the tab knows, in the order the work happens
 *  in: what it wears, what was tried for that, what a video is built from. */
export const ATTACHED_PLACES = ['cover', 'candidate', 'frame', 'video', 'other'] as const

/** An attached file as an entry. `coverId` is the asset that is the cover
 *  now - the newest of the covers; an older one is a file like any other. */
export function entryOfAsset(asset: Asset, coverId: string | undefined): FileEntry {
  const known = (ATTACHED_PLACES as readonly string[]).includes(asset.kind)
  const roles: FileRole[] = []
  if (asset.id === coverId) roles.push('cover')
  if (asset.kind === 'candidate') roles.push('candidate')
  return {
    key: asset.id,
    name: asset.original_name ?? asset.label ?? asset.id,
    path: asset.path,
    category: categoryOf(asset.original_name ?? asset.path),
    place: known ? asset.kind : 'other',
    roles,
    size: asset.size_bytes,
    modified: asset.modified_at,
  }
}

/** A file of the work's folder as an entry. `finishedPath` is the file the
 *  work is finished as, when there is one. */
export function entryOfFolderFile(file: FolderFile, finishedPath: string | undefined): FileEntry {
  const cut = file.relative.lastIndexOf('/')
  return {
    key: file.path,
    name: cut === -1 ? file.relative : file.relative.slice(cut + 1),
    path: file.path,
    category: categoryOf(file.path),
    place: cut === -1 ? '' : file.relative.slice(0, cut),
    roles: file.path === finishedPath ? ['finished'] : [],
    size: file.size,
    modified: file.modified,
  }
}

/** How the files are drawn: the four ways a file manager offers. */
export const VIEWS = ['large', 'tiles', 'list', 'table'] as const
export type FileView = (typeof VIEWS)[number]

/** What the files are gathered by: what they are, or where they lie. */
export type GroupBy = 'type' | 'place'

export const SORTS = ['name', 'modified', 'size', 'category'] as const
export type SortBy = (typeof SORTS)[number]

/** How a person has the Files tab arranged. Theirs, not the work's: kept on
 *  this machine and the same on every card. */
export interface FilesLook {
  view: FileView
  group: GroupBy
  sort: SortBy
  descending: boolean
  /** The categories shown; none ticked means all of them. */
  only: FileCategory[]
}

export const DEFAULT_LOOK: FilesLook = {
  view: 'tiles',
  group: 'type',
  sort: 'name',
  descending: false,
  only: [],
}

const STORED = 'kilna.files.look'

/** The look kept on this machine, with anything unreadable put back to the
 *  default - a key from an older release, a hand edit. */
export function loadLook(storage: Pick<Storage, 'getItem'> = localStorage): FilesLook {
  let kept: Partial<Record<keyof FilesLook, unknown>> = {}
  try {
    const raw = storage.getItem(STORED)
    const read: unknown = raw === null ? {} : JSON.parse(raw)
    if (typeof read === 'object' && read !== null) kept = read as typeof kept
  } catch {
    // Not JSON: the default.
  }
  const oneOf = <T extends string>(all: readonly T[], value: unknown, otherwise: T): T =>
    all.includes(value as T) ? (value as T) : otherwise
  return {
    view: oneOf(VIEWS, kept.view, DEFAULT_LOOK.view),
    group: oneOf(['type', 'place'] as const, kept.group, DEFAULT_LOOK.group),
    sort: oneOf(SORTS, kept.sort, DEFAULT_LOOK.sort),
    descending: kept.descending === true,
    only: Array.isArray(kept.only)
      ? CATEGORIES.filter((category) => (kept.only as unknown[]).includes(category))
      : [],
  }
}

export function saveLook(look: FilesLook, storage: Pick<Storage, 'setItem'> = localStorage): void {
  try {
    storage.setItem(STORED, JSON.stringify(look))
  } catch {
    // A full or closed store: the look holds for this sitting.
  }
}

/** Files gathered under one heading. */
export interface FileGroup {
  /** A category, or a place. */
  key: string
  by: GroupBy
  entries: FileEntry[]
}

/**
 * The entries as the look arranges them: narrowed to the ticked categories,
 * gathered by type or by place, each group sorted.
 *
 * By type the groups stand in the order of `CATEGORIES`. By place they stand
 * in `places` when it is given - what attached files are for has an order of
 * its own - and otherwise with the work's own folder first and the folders
 * under it by name. A group with nothing in it is not there at all: an empty
 * heading is furniture.
 */
export function arrange(
  entries: readonly FileEntry[],
  look: FilesLook,
  places?: readonly string[],
): FileGroup[] {
  const shown =
    look.only.length === 0 ? entries : entries.filter((one) => look.only.includes(one.category))
  const gathered = new Map<string, FileEntry[]>()
  for (const entry of shown) {
    const key = look.group === 'type' ? entry.category : entry.place
    const group = gathered.get(key)
    if (group) group.push(entry)
    else gathered.set(key, [entry])
  }

  const order: string[] =
    look.group === 'type'
      ? [...CATEGORIES]
      : places !== undefined
        ? [...places]
        : [...gathered.keys()].sort((a, b) => (a === '' ? -1 : b === '' ? 1 : a.localeCompare(b)))
  // A place nobody named an order for still shows, after the rest.
  for (const key of gathered.keys()) if (!order.includes(key)) order.push(key)

  return order.flatMap((key) => {
    const group = gathered.get(key)
    return group === undefined
      ? []
      : [{ key, by: look.group, entries: sorted(group, look.sort, look.descending) }]
  })
}

/** By name the way a person reads names - `take 2` before `take 10`. */
const byName = (a: FileEntry, b: FileEntry) =>
  a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })

function sorted(entries: FileEntry[], sort: SortBy, descending: boolean): FileEntry[] {
  const compare = (a: FileEntry, b: FileEntry): number => {
    switch (sort) {
      case 'name':
        return byName(a, b)
      case 'size':
        return a.size - b.size || byName(a, b)
      case 'category':
        return (
          CATEGORIES.indexOf(a.category) - CATEGORIES.indexOf(b.category) ||
          extensionOf(a.name).localeCompare(extensionOf(b.name)) ||
          byName(a, b)
        )
      case 'modified': {
        // A file whose time is not known stands last either way: it is not
        // older than the rest, it is unknown.
        if (a.modified === undefined || b.modified === undefined) {
          return a.modified === b.modified ? byName(a, b) : a.modified === undefined ? 1 : -1
        }
        return a.modified.localeCompare(b.modified) || byName(a, b)
      }
    }
  }
  const unknownLast = (a: FileEntry, b: FileEntry) =>
    sort === 'modified' && (a.modified === undefined) !== (b.modified === undefined)
  return [...entries].sort((a, b) => {
    const order = compare(a, b)
    return descending && !unknownLast(a, b) ? -order : order
  })
}

/** What looking at a file inside kilna shows (FR-14): the picture, the
 *  clip, the text, the document - or, for anything else, what is known of
 *  it. A sound is not looked at: it plays in the card's player. */
export type Sight = 'image' | 'video' | 'text' | 'pdf' | 'sound' | 'other'

/** The endings read inside the window as text - the list the backend holds
 *  the reading to (`folder::TEXTS`). */
const TEXTS = ['txt', 'md', 'srt', 'vtt', 'lrc', 'json', 'csv', 'log', 'yaml', 'yml']

/** What the window itself can draw of the pictures and clips: a file filed
 *  as one by its ending and not among these is shown as what is known. */
const DRAWN = [
  'png',
  'jpg',
  'jpeg',
  'webp',
  'gif',
  'avif',
  'svg',
  'bmp',
  'mp4',
  'webm',
  'mov',
  'm4v',
]

export function sightOf(entry: Pick<FileEntry, 'name' | 'category'>): Sight {
  const ending = extensionOf(entry.name)
  if (entry.category === 'audio') return 'sound'
  if (entry.category === 'image') return DRAWN.includes(ending) ? 'image' : 'other'
  if (entry.category === 'video') return DRAWN.includes(ending) ? 'video' : 'other'
  if (ending === 'pdf') return 'pdf'
  return TEXTS.includes(ending) ? 'text' : 'other'
}

/** The endings the system may be asked to open a file of: what the backend
 *  will open (`folder::OPENABLE`) - a picture, a clip, a sound, a plain
 *  document, never a program. */
const OPENABLE = [
  ...['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif'],
  ...['mp4', 'webm', 'mov', 'm4v'],
  ...['mp3', 'wav', 'flac', 'm4a', 'aac', 'ogg', 'oga', 'opus'],
  ...['pdf', 'txt', 'md'],
]

export function opensInSystem(name: string): boolean {
  return OPENABLE.includes(extensionOf(name))
}

/** The entry `step` places from `key` among those that are looked at - the
 *  sounds left out - going round at either end. */
export function neighbour(
  entries: readonly FileEntry[],
  key: string,
  step: 1 | -1,
): FileEntry | undefined {
  const seen = entries.filter((one) => sightOf(one) !== 'sound')
  const at = seen.findIndex((one) => one.key === key)
  if (at === -1 || seen.length < 2) return undefined
  return seen[(at + step + seen.length) % seen.length]
}
