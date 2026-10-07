import { countChanges, diffLines } from '@/lib/diff'
import { sectionLines } from '@/lib/sections'

/**
 * What a version's row in the list says about its text: how long it is, in
 * the units a writer counts in, and how far it moved from the version it is
 * compared with by default (`lib/history`'s `predecessor`).
 */
export interface VersionStats {
  /** Words of the text itself - a part's header names it, and is not sung. */
  words: number
  /** Lines with words on them: blank lines and part headers are not lines a
   *  writer counts. */
  lines: number
  /** Lines in and out since the predecessor - the comparison's own count,
   *  so the row and the column beside the text never disagree. `null` with
   *  nothing to compare against. */
  change: { added: number; removed: number } | null
}

/** A word: letters or digits, with an apostrophe or a hyphen inside it. */
const WORD = /[\p{L}\p{N}]+(?:['’-][\p{L}\p{N}]+)*/gu

export function statsOf(body: string, previous: string | null): VersionStats {
  const headers = new Set(sectionLines(body))
  let words = 0
  let lines = 0
  body.split('\n').forEach((line, index) => {
    if (headers.has(index)) return
    const found = line.match(WORD)?.length ?? 0
    if (found === 0) return
    words += found
    lines += 1
  })
  return {
    words,
    lines,
    change: previous === null ? null : countChanges(diffLines(previous, body)),
  }
}
