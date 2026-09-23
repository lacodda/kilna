import type { Note } from '@/lib/api'

/**
 * What a note is called in a list: its title, or its first line without the
 * markdown that dresses it. Also what a work made from an untitled idea is
 * offered as a name.
 */
export function titleOf(note: Pick<Note, 'title' | 'body'>): string {
  const title = note.title?.trim()
  if (title !== undefined && title !== '') return title
  for (const raw of note.body.split('\n')) {
    const line = raw
      .trim()
      .replace(/^(?:[#>*+-]+|\d+[.)])\s*/, '')
      .replace(/^\[[ xX]\]\s*/, '')
      .trim()
    if (line !== '') return line.length > 80 ? `${line.slice(0, 79)}…` : line
  }
  return ''
}

/** Tags as typed into one field: split on commas, trimmed, empties and
 *  repeats dropped, the first spelling kept. */
export function tagsFrom(text: string): string[] {
  const seen = new Set<string>()
  const out: string[] = []
  for (const raw of text.split(',')) {
    const tag = raw.trim()
    if (tag === '' || seen.has(tag.toLowerCase())) continue
    seen.add(tag.toLowerCase())
    out.push(tag)
  }
  return out
}
