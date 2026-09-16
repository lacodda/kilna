import type { Scene, SceneFrame } from '@/lib/api'
import { chosenFrame, chosenVideo } from '@/lib/scenes'
import { formatSeconds } from '@/lib/timecode'

/*
 * The list a board is cut from.
 *
 * What an editing program needs from kilna is not a project file - it is the
 * answer to "which file, in what order, for how long". A project file would
 * be a better answer for exactly one program and a worse one for every other,
 * and it would tie a board to a version of software the board has no opinion
 * about. Text is what every one of them reads, what a person can check with
 * their eyes before trusting it, and what pastes into a note, a message or a
 * terminal unchanged (decision of the owner, 2026-09-16).
 *
 * A scene takes one block: its number and timing on the first line, then the
 * paths, one per line, indented under it. Two lines rather than columns
 * because a path is long and a column of them wraps into porridge; indented
 * rather than flat because the eye needs to see which scene a file belongs to
 * without counting.
 *
 * A scene with nothing chosen still gets its block, saying so. Silence would
 * be the worse answer: the list is also how a person finds out what is still
 * missing, and a gap that prints nothing reads as a gap that is not there.
 */

/** What is written for a scene with no still or no clip chosen. */
const MISSING = '—'

export interface MontageOptions {
  /** How a missing file is written. The screen's word, so it can be
   * translated; the default is the dash the board uses elsewhere. */
  missing?: string
}

/** One scene's lines, given its material. */
function block(
  scene: Scene,
  frames: SceneFrame[] | undefined,
  missing: string,
): string {
  const number = String(scene.position).padStart(2, '0')
  const from = formatSeconds(scene.starts_at ?? null)
  const to = formatSeconds(scene.ends_at ?? null)
  // An untimed scene says so rather than printing an empty span: a board is
  // timed late, and a dash is a question, while `–` alone is a typo.
  const when = from === '' && to === '' ? missing : `${from}–${to}`
  const still = chosenFrame(frames)?.path ?? missing
  const clip = chosenVideo(frames)?.path ?? missing
  const indent = ' '.repeat(number.length + 2 + when.length + 2)
  return `${number}  ${when}  ${still}\n${indent}${clip}`
}

/**
 * The whole board as the text that goes to the clipboard and into the `.txt`.
 *
 * Scenes arrive in the order the board shows them; this does not sort them
 * again, because the board's order IS the cut and a second opinion about it
 * here would be a second truth.
 */
export function montageList(
  scenes: Scene[],
  framesByScene: Map<string, SceneFrame[]>,
  options: MontageOptions = {},
): string {
  const missing = options.missing ?? MISSING
  return scenes.map((scene) => block(scene, framesByScene.get(scene.id), missing)).join('\n')
}

/** What the saved file is called: the work's name, so a folder of them is
 * readable, with the unsafe characters taken out rather than encoded. */
export function montageFileName(title: string): string {
  const safe = title
    .replace(/[\\:*?"<>|/]/g, '')
    .trim()
    .replace(/\s+/g, ' ')
  return `${safe === '' ? 'montage' : safe} — montage.txt`
}
