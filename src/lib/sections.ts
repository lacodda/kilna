/**
 * The lines of a text that name a part of it rather than being part of it.
 *
 * A lyric is written in sections - `[Verse 1]`, `[Chorus: louder, doubled]`,
 * `[Bridge]` - and the headers are how it is read: the eye jumps from one to
 * the next to find the chorus, and a generator reads them as instructions
 * rather than as words to sing. Drawn in the text's own colour they are one
 * more line among forty, so the reading view picks them out in the accent,
 * the way the mockup's `.lyrics .sec` does.
 *
 * A header is a whole line in square brackets. A bracket inside a line
 * ("I said [quietly] no") is part of the line and stays as it is.
 */

/** One bracketed name on a line of its own, with space around it allowed. */
const HEADER = /^\s*\[[^\]\n]+\]\s*$/

/** Zero-based indexes of the lines that are section headers, in order. */
export function sectionLines(text: string): number[] {
  const found: number[] = []
  text.split('\n').forEach((line, index) => {
    if (HEADER.test(line)) found.push(index)
  })
  return found
}
