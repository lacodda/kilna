import type { LineMark } from '@/components/ui/marked-text'
import { sectionLines } from '@/lib/sections'
import { cn } from '@/lib/utils'

/**
 * How a version's text is set, wherever it is on screen: read, typed into,
 * standing beside another as the older side of a comparison, or written into
 * the form for a new version.
 *
 * One class list for all of them, because the editor draws its marks on a
 * mirror of the text and the mirror has to land on the same letters - and
 * because a text that changed size between reading it and starting to type
 * would move the line the eye was on. Plain bodies are the mockup's `.lyrics`:
 * monospace at the scale's base size, on a line height of 1.75 so a verse
 * reads as lines rather than as a paragraph. A markdown body is prose, and
 * takes the renderer's own type.
 *
 * `selectable`, because the registry copies grant nothing of their own and a
 * version is someone's writing: it has to be copyable wherever it shows.
 */
export function textMetrics(markdown: boolean): string {
  return cn(
    'selectable px-5.5 py-4 text-base',
    markdown ? 'leading-relaxed' : 'font-mono leading-[1.75]',
  )
}

/** How many tints a repeated word may be drawn in before they cycle. */
export const REPEAT_TINTS = 6

/**
 * A line new since the version the text is compared with by default (#24):
 * a thin bar in the left margin, the way an editor marks a changed line in
 * its gutter. Quieter than the comparison's tint, because it is always on -
 * every draft shows what it changed without a press - and drawn in the
 * padding, so it sits beside the letters rather than under them.
 */
export const CHANGED_LINE =
  'relative before:absolute before:inset-y-0 before:-left-3 before:w-0.5 before:rounded-full before:bg-good'

/** The mockup's `.lyrics .sec`: a section header in the accent. */
const SECTION = 'text-accent-2'

/**
 * The line marks of a plain text with its section headers picked out.
 *
 * One class per line is what the marked text draws, so a header that is also
 * a line new since the other version keeps both: the tint of the change and
 * the ink of the header.
 */
export function withSections(text: string, marks: readonly LineMark[]): LineMark[] {
  const byLine = new Map<number, string | undefined>()
  for (const line of sectionLines(text)) byLine.set(line, SECTION)
  for (const mark of marks) byLine.set(mark.line, cn(byLine.get(mark.line), mark.className))
  return [...byLine].map(([line, className]) => ({ line, className }))
}
