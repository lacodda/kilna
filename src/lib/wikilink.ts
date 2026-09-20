/**
 * `[[work:id]]` and `[[version:id]]` in a body, read as links.
 *
 * A body is written in one place and read in five — a card, a note, a diff, an
 * assistant's reply, an export. A person writing "see [[work:abc]]" means the
 * same thing in all of them, so the meaning is parsed here, once, and each
 * reader decides what to draw.
 *
 * What is *not* here is any knowledge of routes or of the DOM: this says what
 * a link means, the renderer says what it looks like. That is what lets the
 * same rule be tested against strings rather than against a rendered page.
 */

/** What a wiki link points at. Unknown kinds are not links. */
export type LinkTarget = 'work' | 'version'

const TARGETS: readonly LinkTarget[] = ['work', 'version']

export interface WikiLink {
  target: LinkTarget
  id: string
  /** The text to draw, when the link carried one after a `|`. */
  label?: string
  /** Where it sat in the source, so a renderer can splice around it. */
  start: number
  end: number
}

/*
 * Inside the brackets: a kind, a colon, an id, and optionally `|` and a label.
 *
 * The id is deliberately narrow — the characters a uuid and a short key are
 * made of — so that `[[see: the note about x]]` stays prose. A person who
 * writes a sentence in double brackets has not written a link, and turning it
 * into a dead one would be worse than leaving it alone.
 *
 * No newline inside: a link that swallowed a paragraph break would eat the
 * text after it whenever someone left a bracket unclosed.
 */
const PATTERN = /\[\[([a-z]+):([A-Za-z0-9_-]+)(\|([^\]\n]*))?\]\]/g

/** Every link in `body`, in the order it reads. */
export function wikiLinks(body: string): WikiLink[] {
  const found: WikiLink[] = []
  // A fresh regex per call: a module-level `g` regex carries `lastIndex`
  // between calls, which makes the second reader of the same body see half
  // the links. Cheap, and the alternative is a bug that only shows up when
  // two components render at once.
  const pattern = new RegExp(PATTERN.source, 'g')
  let match: RegExpExecArray | null
  while ((match = pattern.exec(body)) !== null) {
    const whole = match[0]
    const kind = match[1]
    const id = match[2]
    const label = match[4]
    if (kind === undefined || id === undefined) continue
    if (!TARGETS.includes(kind as LinkTarget)) continue
    const trimmed = label?.trim()
    found.push({
      target: kind as LinkTarget,
      id,
      label: trimmed === undefined || trimmed === '' ? undefined : trimmed,
      start: match.index,
      end: match.index + whole.length,
    })
  }
  return found
}

/**
 * The body with every link replaced by what `draw` returns for it.
 *
 * The text between links is passed through `text` untouched, which is how a
 * caller escapes it: this function never decides what is safe, because what is
 * safe depends on where the result is going.
 */
export function replaceWikiLinks(
  body: string,
  draw: (link: WikiLink) => string,
  text: (between: string) => string = (between) => between,
): string {
  const links = wikiLinks(body)
  if (links.length === 0) return text(body)

  let out = ''
  let at = 0
  for (const link of links) {
    out += text(body.slice(at, link.start))
    out += draw(link)
    at = link.end
  }
  out += text(body.slice(at))
  return out
}

/**
 * Where a link goes in the window.
 *
 * A work is its own address. A version is not: the card is addressed by the
 * *work*, so a version link needs the work it belongs to, which only a lookup
 * knows. Passing it in rather than guessing is the difference between a link
 * that opens the right card and one that opens a card that does not exist —
 * and a caller that has not looked it up yet gets `undefined`, which is what
 * makes an unresolved link render as plain text instead of a dead one.
 */
export function hrefOf(
  link: Pick<WikiLink, 'target' | 'id'>,
  workOfVersion?: (versionId: string) => string | undefined,
): string | undefined {
  if (link.target === 'work') return `/works/${link.id}`
  const workId = workOfVersion?.(link.id)
  return workId === undefined ? undefined : `/works/${workId}/versions`
}
