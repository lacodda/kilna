import type { Composition, ProfileConfig, StyleBrick, StyleType, TextCheck } from '@/lib/api/types'

/*
 * The window's side of the dictionary of phrases (v0.94): which texts are
 * written out of it, how the dictionary is split into its halves, and where a
 * phrase goes when it is put into a text by hand. Pure, so it is tested
 * rather than discovered by clicking.
 */

/** The composition a version of `role` of a work of `kind` is written by. */
export function compositionFor(
  config: ProfileConfig,
  kind: string | undefined,
  role: string,
): Composition | undefined {
  if (kind === undefined) return undefined
  return (config.compose ?? []).find(
    (composition) => composition.role === role && composition.kinds.includes(kind),
  )
}

/** The key of the half of the dictionary that is not a composition's. */
export const PICTURE = 'picture'

/** One half of the dictionary: the picture's, or a composition's. */
export interface Half {
  key: string
  /** Null for the picture's half, which the window names itself. */
  composition: Composition | null
  types: StyleType[]
}

/**
 * The dictionary split the way it is used: the types a picture is built from,
 * then one half per composition - its types in the order they are picked.
 * A type of phrase in no composition is nowhere a text is written from; the
 * profile refuses one, and here it would stand with the pictures.
 */
export function halvesOf(config: ProfileConfig, types: readonly StyleType[]): Half[] {
  const compositions = config.compose ?? []
  const composed = new Set(compositions.flatMap((c) => c.parts.map((part) => part.type)))
  const halves: Half[] = [
    { key: PICTURE, composition: null, types: types.filter((type) => !composed.has(type.key)) },
  ]
  for (const composition of compositions) {
    const own = composition.parts
      .map((part) => types.find((type) => type.key === part.type))
      .filter((type): type is StyleType => type !== undefined)
    halves.push({ key: composition.key, composition, types: own })
  }
  return halves.filter((half) => half.types.length > 0)
}

/** The half a type stands in. */
export function halfOfType(halves: readonly Half[], typeKey: string | undefined): Half | undefined {
  if (typeKey === undefined) return halves[0]
  return halves.find((half) => half.types.some((type) => type.key === typeKey))
}

/**
 * A text with `phrase` put in at the caret, as a tag of a tag line: a
 * separator before it unless the caret is at the start of the text or a line
 * or just after one, and after it unless the text goes on with one. The caret
 * lands after the phrase.
 */
export function insertPhrase(
  text: string,
  caret: number,
  phrase: string,
  separator = ', ',
): { text: string; caret: number } {
  const at = Math.max(0, Math.min(caret, text.length))
  const before = text.slice(0, at)
  const after = text.slice(at)
  const mark = separator.trim()
  const lead = before.trimEnd()
  const needsBefore = lead !== '' && !lead.endsWith('\n') && !lead.endsWith(mark)
  const trail = after.trimStart()
  const needsAfter = trail !== '' && !trail.startsWith(mark) && !trail.startsWith('\n')
  const head =
    lead === '' || before.endsWith('\n') ? before : needsBefore ? `${lead}${separator}` : `${lead} `
  const inserted = `${head}${phrase}`
  const tail = needsAfter ? `${separator}${trail}` : after
  return { text: `${inserted}${tail}`, caret: inserted.length }
}

/** The bricks of a composition's types, house ones first within a type,
 *  then by name - the order a picker offers them in. */
export function offeredBricks(
  bricks: readonly StyleBrick[],
  typeKey: string,
  house: ReadonlySet<string>,
): StyleBrick[] {
  return bricks
    .filter((brick) => brick.type_key === typeKey && brick.status === 'ready')
    .sort(
      (a, b) =>
        Number(house.has(b.id)) - Number(house.has(a.id)) ||
        (a.description ?? a.name).localeCompare(b.description ?? b.name),
    )
}

/** Whether a brick answers a search typed in any language: its phrase, its
 *  name, or what it means. */
export function brickMatches(brick: StyleBrick, query: string): boolean {
  const q = query.trim().toLowerCase()
  if (q === '') return true
  const explained =
    brick.explanation === null
      ? []
      : typeof brick.explanation === 'string'
        ? [brick.explanation]
        : Object.values(brick.explanation)
  return [brick.name, brick.description ?? '', ...explained].some((text) =>
    text.toLowerCase().includes(q),
  )
}

/** The class a mark of a phrase carries, with its index: what the text is
 *  hovered by (`phraseAt`). */
export const PHRASE_MARK = 'phrase-mark'

/** The phrase a hovered element of a read text is a mark of, if any. */
export function phraseAt(target: EventTarget | null, check: TextCheck | undefined): number | null {
  if (check === undefined || !(target instanceof Element)) return null
  const mark = target.closest('mark')
  if (mark === null) return null
  for (const name of mark.classList) {
    const index = /^phrase-(\d+)$/.exec(name)?.[1]
    if (index !== undefined) return Number(index)
  }
  return null
}
