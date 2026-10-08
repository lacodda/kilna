/*
 * What a stored file is to look at: a picture, a clip, or something else.
 *
 * Decided by the file's ending, the one fact every stored file has. The Files
 * tab drew every file as an `<img>` - a scene's clips live among the work's
 * files, and each showed as a broken picture - and the frame viewer did the
 * same to a clip opened from the clips strip. One answer, used by both.
 *
 * The lists are the same the backend holds a strip to (`scene_frame::PICTURES`
 * and `CLIPS`), and the pickers offer.
 */

/** The endings a picture arrives as. */
export const PICTURES = ['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif']

/** The endings a clip arrives as. */
export const CLIPS = ['mp4', 'webm', 'mov', 'm4v']

/** The endings a sound arrives as - a mix, a stem, a take (v0.93). What the
 *  window's player can open: the formats WebView2 and WebKit both decode,
 *  FLAC and Opus among them. */
const SOUNDS = ['mp3', 'wav', 'flac', 'm4a', 'aac', 'ogg', 'oga', 'opus']

export type MediaKind = 'picture' | 'clip' | 'sound' | 'other'

/** A path's ending, lower-case and without the dot; empty when it has none. */
export function extensionOf(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? ''
  const dot = name.lastIndexOf('.')
  return dot <= 0 ? '' : name.slice(dot + 1).toLowerCase()
}

export function mediaKindOf(path: string): MediaKind {
  const ending = extensionOf(path)
  if (PICTURES.includes(ending)) return 'picture'
  if (CLIPS.includes(ending)) return 'clip'
  if (SOUNDS.includes(ending)) return 'sound'
  return 'other'
}

/** The name a path ends in: what a file is called on screen. */
export function nameOf(path: string): string {
  return path.split(/[\\/]/).pop() ?? path
}
