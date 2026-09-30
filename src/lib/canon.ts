import type {
  CanonSection,
  CardSummary,
  Fact,
  FactStatus,
  Layer,
  Lens,
  NoteKind,
  ProfileConfig,
  ReadFact,
} from '@/lib/api/types'

/*
 * The canon as the window reads it (ADR 0043): which kinds of note are cards,
 * how the list of cards is grouped, and what a task may see of a fact.
 *
 * What a task may see is never decided here. The backend says it, fact by
 * fact, in `ReadFact.lenses` - the same function every prompt reads through -
 * and this only asks the answer. A second copy of the rule in the window
 * would be the one place the screen could show what the generator is not
 * given.
 */

/** The three layers, in the order the canon tells them. */
export const LAYERS: readonly Layer[] = ['public', 'internal', 'inWorks']

/** The states a person can put a fact in by hand; `retired` asks for a reason
 *  and is its own gesture. */
export const SETTLED: readonly FactStatus[] = ['canon', 'open', 'draft']

/** The tasks a card is read for, and "everything", the screen's own view. */
export type LensChoice = 'all' | Lens
export const LENS_CHOICES: readonly LensChoice[] = ['all', 'cover', 'work', 'public']

/** Every kind of note the profile names. */
function noteKindsOf(config: ProfileConfig): NoteKind[] {
  return config.note_kinds ?? []
}

/** The kinds of card: the note kinds the profile gives sections, root first. */
export function cardKindsOf(config: ProfileConfig): string[] {
  const kinds = noteKindsOf(config).filter((kind) => (kind.sections ?? []).length > 0)
  return [...kinds.filter((k) => k.root === true), ...kinds.filter((k) => k.root !== true)].map(
    (kind) => kind.key,
  )
}

/** The kind of card `key` is, when it is one. */
export function cardKindOf(config: ProfileConfig, key: string): NoteKind | undefined {
  return noteKindsOf(config).find((kind) => kind.key === key && (kind.sections ?? []).length > 0)
}

/** The kinds a plain note can take: those with no sections. */
export function plainNoteKindsOf(config: ProfileConfig): NoteKind[] {
  return noteKindsOf(config).filter((kind) => (kind.sections ?? []).length === 0)
}

/** Whether the craft keeps a canon at all: one kind of card is enough. */
export function hasCanon(config: ProfileConfig): boolean {
  return cardKindsOf(config).length > 0
}

/** The sections of a kind that hold facts written by hand. */
export function writableSections(kind: NoteKind): CanonSection[] {
  return (kind.sections ?? []).filter(
    (section) => section.shape !== 'relations' && section.shape !== 'appearances',
  )
}

/** A group of the list: a kind of card, or the heroes who live at one work. */
export interface CardGroup {
  /** The kind's key, or `heroes`. */
  key: string
  kind?: NoteKind
  cards: CardSummary[]
}

/**
 * The list of cards, grouped as the mockup groups it: the root, then kind by
 * kind in the profile's order, and last the heroes of single works - a card
 * that lives at a song is read apart from the world's standing cast until it
 * is raised to the channel. A group with nothing in it is not drawn.
 */
export function groupCards(cards: readonly CardSummary[], config: ProfileConfig): CardGroup[] {
  const groups: CardGroup[] = []
  for (const key of cardKindsOf(config)) {
    const kind = cardKindOf(config, key)
    const own = cards.filter((card) => card.kind === key && card.work_id === null)
    if (own.length > 0) groups.push({ key, kind, cards: own })
  }
  // A card of a kind the profile no longer names still reads, under its key.
  const known = new Set(cardKindsOf(config))
  const strays = cards.filter((card) => !known.has(card.kind) && card.work_id === null)
  if (strays.length > 0) groups.push({ key: 'other', cards: strays })
  const heroes = cards.filter((card) => card.work_id !== null)
  if (heroes.length > 0) groups.push({ key: 'heroes', cards: heroes })
  return groups
}

/** The facts of one section of a card, in the order the person set. */
export function factsIn(facts: readonly ReadFact[], section: string): ReadFact[] {
  return facts
    .filter((read) => read.fact.section === section)
    .sort((a, b) => a.fact.position - b.fact.position)
}

/** Whether the screen, looking through `lens`, shows a fact at full strength. */
export function seenThrough(lenses: readonly Lens[], lens: LensChoice): boolean {
  return lens === 'all' || lenses.includes(lens)
}

/** When a fact happened in the world, as it is told. */
export function whenOf(fact: Pick<Fact, 'when'>): string | null {
  const when = fact.when
  if (when === null || when === undefined) return null
  return when.label ?? when.sort ?? null
}

/** The letter an avatar shows when a card has no portrait. */
export function initialOf(title: string | null | undefined): string {
  const words = (title ?? '').replace(/[«»"“”]/g, '').trim()
  return words === '' ? '?' : [...words][0]!.toUpperCase()
}
