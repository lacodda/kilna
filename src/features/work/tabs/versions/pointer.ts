/*
 * Where a pointer landed in a marked textarea, in characters.
 *
 * A click in a textarea places the caret between two characters - the
 * boundary nearest the pointer - so the caret alone does not say which letter
 * was clicked: the right half of an "и" and the left half of the "я" after it
 * leave the caret in the same place. The stress gesture (ADR 0053) has to
 * know the letter, or "линия" gets its stress one vowel off.
 *
 * The textarea draws no boxes for its letters, but its mirror does
 * (`MarkedTextarea`): the same text, line by line, at the same metrics,
 * underneath. The letter after the caret is measured there, and the pointer
 * is on it or on the one before.
 */

/** The text node and offset inside the mirror where `index` of the text is. */
function mirrored(mirror: Element, text: string, index: number): [Node, number] | null {
  const line = text.slice(0, index).split('\n').length - 1
  const column = index - (text.lastIndexOf('\n', index - 1) + 1)
  const row = mirror.querySelector(`[data-line="${line}"]`)
  if (row === null) return null
  const walker = document.createTreeWalker(row, NodeFilter.SHOW_TEXT)
  let left = column
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    const length = node.textContent?.length ?? 0
    if (left < length) return [node, left]
    left -= length
  }
  return null
}

/**
 * The index of the character under the pointer, given where the click put the
 * caret - or null when the window cannot measure (no layout, as in a test),
 * and the caller falls back to reading the caret alone.
 */
export function charUnderPointer(
  field: HTMLTextAreaElement,
  text: string,
  caret: number,
  x: number,
  y: number,
): number | null {
  // The caret at the end of a line: nothing after it on that line, so the
  // pointer was past the line's last letter.
  if (caret >= text.length || text.charAt(caret) === '\n') return caret - 1
  const mirror = field.parentElement?.querySelector('[data-mirror]')
  if (mirror == null) return null
  const at = mirrored(mirror, text, caret)
  if (at === null) return null
  const range = document.createRange()
  range.setStart(at[0], at[1])
  range.setEnd(at[0], at[1] + 1)
  if (typeof range.getBoundingClientRect !== 'function') return null
  const box = range.getBoundingClientRect()
  if (box.width === 0 && box.height === 0) return null
  // The letter after the caret wrapped onto the next line: the click was at
  // the end of this one.
  if (y < box.top - box.height / 2) return caret - 1
  return x >= box.left ? caret : caret - 1
}

/** Bring the mirror's line holding `index` into view - the textarea never
 *  scrolls itself; the box around it does, and a caret placed off screen by
 *  a press elsewhere would be a caret nobody sees. */
export function revealIndex(field: HTMLTextAreaElement, text: string, index: number): void {
  const line = text.slice(0, index).split('\n').length - 1
  const row = field.parentElement?.querySelector(`[data-mirror] [data-line="${line}"]`)
  row?.scrollIntoView({ block: 'nearest' })
}
