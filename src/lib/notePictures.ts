import { attachAsset, fileSrc, pasteAsset } from '@/lib/api/assets'
import type { Asset } from '@/lib/api/types'
import { nameOf, PICTURES } from '@/lib/media'

/*
 * Pictures written into a note's body (v0.93, ADR 0058).
 *
 * A pasted picture is copied into the workspace like any other file (ADR
 * 0027), hung on its note as `inline`, and named in the text by the file it
 * was stored as: `![](media/<id>.png)` - relative to the workspace, so the
 * text reads the same in an export that carries `media/` beside it.
 *
 * The functions below are the two halves of the shared editor's picture host
 * (`@lacodda/scheda-editor`'s `AssetHost`): `pastePicture` writes a pasted
 * picture and `pictureLine` gives the link to insert, `urlOf` turns a link
 * back into something the page can load. A textarea calls them today; the
 * day the notes move onto that editor, they are its host as they stand.
 */

/** How a body names a file of the workspace, before its stored name. */
export const BODY_PREFIX = 'media/'

/** The link a body writes for a picture of the workspace. */
function linkOf(asset: Pick<Asset, 'path'>): string {
  return `${BODY_PREFIX}${nameOf(asset.path)}`
}

/** The stored name a link names, when it is one of the workspace's own: a
 *  bare name after `media/`, nothing that steps out of the folder. */
export function storedNameOf(link: string): string | null {
  if (!link.startsWith(BODY_PREFIX)) return null
  const name = decodeURIComponent(link.slice(BODY_PREFIX.length))
  if (name === '' || name === '.' || name === '..' || /[\\/]/.test(name)) return null
  return name
}

/** A URL the page can load for a picture a body links to, or null when the
 *  link is not one of the workspace's files. */
export function urlOf(directory: string, link: string): string | null {
  const name = storedNameOf(link)
  if (name === null) return null
  const separator = directory.includes('\\') ? '\\' : '/'
  const base = directory.endsWith(separator) ? directory : `${directory}${separator}`
  return fileSrc(`${base}${name}`)
}

/** A picture written as a line of markdown: on a line of its own, so it
 *  stands as a block in the text rather than inside a sentence. */
export function pictureLine(asset: Asset): string {
  const name = asset.original_name ?? ''
  const stem = name.replace(/\.[^.]+$/, '')
  // What a clipboard calls every picture says nothing about this one.
  const alt = stem.toLowerCase() === 'image' ? '' : stem.replace(/[[\]]/g, '')
  return `![${alt}](${linkOf(asset)})`
}

/** Write a pasted picture onto a note and answer with the asset. */
export function pastePicture(noteId: string, file: File): Promise<Asset> {
  return file.arrayBuffer().then((buffer) =>
    pasteAsset(new Uint8Array(buffer), file.name === '' ? 'pasted.png' : file.name, {
      note_id: noteId,
      kind: 'inline',
    }),
  )
}

/** Copy a picture from disk onto a note - one dropped on the editor. */
export function attachPicture(noteId: string, path: string): Promise<Asset> {
  return attachAsset(path, { note_id: noteId, kind: 'inline' })
}

/** Whether a path names a picture a note can show. */
export function isPicturePath(path: string): boolean {
  const name = nameOf(path)
  const dot = name.lastIndexOf('.')
  return dot > 0 && PICTURES.includes(name.slice(dot + 1).toLowerCase())
}

/** Put `inserted` into `text` at `at`, on a line of its own. */
export function insertAt(text: string, at: number, inserted: string): string {
  const cut = Math.max(0, Math.min(at, text.length))
  const before = text.slice(0, cut)
  const after = text.slice(cut)
  const lead = before === '' || before.endsWith('\n') ? '' : '\n'
  const tail = after === '' || after.startsWith('\n') ? '' : '\n'
  return `${before}${lead}${inserted}${tail}${after}`
}
