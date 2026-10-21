import type {
  Board,
  BoardCard,
  BoardView,
  CardKind,
  CardRequest,
  LibraryEntry,
  NoteKind,
  ProfileConfig,
  PromptTemplate,
  WorkedIn,
} from '@/lib/api/types'

/**
 * The boards of a work's conception (v0.99, ADR 0065), as the window reads
 * them: which cards a filter shows, how a board groups its cards under
 * topics, what a task about a board is called and what it asks for.
 */

/** The most cards one run is asked for - the backend's `answer::MOST`. */
export const MOST_CARDS = 6

/** How many a board asks for when the person has not said - `ON_ASK`. */
export const CARDS_ON_ASK = 4

/** The board's filters, in the order the cover's board has them. */
export const CARD_FILTERS = ['all', 'star', 'mine', 'rejected'] as const
export type CardFilter = (typeof CARD_FILTERS)[number]

/** Whether a card shows under a filter. A turned-down card stays under
 *  "all", dimmed: it is still an example of what is not wanted. */
export function shows(card: BoardCard, filter: CardFilter): boolean {
  switch (filter) {
    case 'all':
      return true
    case 'star':
      return card.verdict === 'star'
    case 'mine':
      return card.source === 'own' || card.source === 'refined'
    case 'rejected':
      return card.verdict === 'rejected'
  }
}

/** How many cards each filter shows. */
export function counts(cards: readonly BoardCard[]): Record<CardFilter, number> {
  const out = { all: 0, star: 0, mine: 0, rejected: 0 }
  for (const card of cards) {
    for (const filter of CARD_FILTERS) if (shows(card, filter)) out[filter] += 1
  }
  return out
}

/** One group of a board: a topic and its cards, in the order they stand. */
export interface Topic {
  /** The topic's word; empty for the cards under none. */
  name: string
  cards: BoardCard[]
}

/**
 * A board's cards under their topics: the topics in the order their first
 * cards stand, the cards in order within each - the backend's arrangement,
 * read as groups, the same order the order and the export read. A board
 * with no topics at all is one group with no name.
 */
export function topicsOf(cards: readonly BoardCard[]): Topic[] {
  const sorted = [...cards].sort((a, b) => a.position - b.position)
  const groups: Topic[] = []
  for (const card of sorted) {
    const found = groups.find((group) => group.name === card.topic)
    if (found === undefined) groups.push({ name: card.topic, cards: [card] })
    else found.cards.push(card)
  }
  return groups
}

/** The kind of card a card is, on its board. */
export function kindOf(board: Board, card: Pick<BoardCard, 'kind'>): CardKind | undefined {
  return board.cards.find((kind) => kind.key === card.kind)
}

/** What a card is called: the entry of the library it links to, or the
 *  first of its kind's fields that says anything. */
export function headlineOf(board: Board, card: BoardCard): string {
  if (card.entry != null) return entryHeadline(card.entry)
  const keys = kindOf(board, card)?.fields.map((field) => field.key) ?? Object.keys(card.fields)
  for (const key of keys) {
    const text = card.fields[key]?.trim()
    if (text !== undefined && text !== '') return text
  }
  return ''
}

/** An entry of the library as a list reads it: the author, the title, the
 *  year - the backend's `Entry::headline`, in the window's quotes. */
export function entryHeadline(entry: Pick<LibraryEntry, 'title' | 'author' | 'year'>): string {
  const parts: string[] = []
  if (entry.author != null && entry.author.trim() !== '') parts.push(entry.author.trim())
  parts.push(`«${entry.title.trim()}»`)
  if (entry.year != null && entry.year.trim() !== '') parts.push(entry.year.trim())
  return parts.join(', ')
}

/** The note kind that is the library, when the profile has one. */
export function libraryOf(config: ProfileConfig): NoteKind | undefined {
  return config.note_kinds.find((kind) => kind.library === true)
}

/** The medium of an entry, by its key, on the library's kind. */
export function mediumOf(library: NoteKind | undefined, key: string | null | undefined) {
  if (key == null) return undefined
  return library?.media?.find((medium) => medium.key === key)
}

/** Whether a work of the entry's medium is ever quoted: a song's lyrics are
 *  not. An entry with no medium, or one the library no longer names, is. */
export function quoted(library: NoteKind | undefined, entry: LibraryEntry | null): boolean {
  return mediumOf(library, entry?.medium)?.quoted !== false
}

/** The chips of a field - one to a line or parted by semicolons - the
 *  backend's `library::chips`. */
export function chipsOf(text: string | undefined): string[] {
  const out: string[] = []
  for (const raw of (text ?? '').split(/[\n;]/)) {
    const chip = raw
      .trim()
      .replace(/[.,]+$/, '')
      .trim()
    if (chip === '' || out.some((kept) => kept.toLowerCase() === chip.toLowerCase())) continue
    out.push(chip)
  }
  return out
}

/** Whether a text is a link a person can open: where a quote was checked. */
export function isLink(text: string | undefined): boolean {
  const trimmed = (text ?? '').trim()
  return /^https?:\/\/\S+$/.test(trimmed) && trimmed.length > 'https://'.length
}

/** How a board is grouped: by the topics the person gave it, or by what its
 *  cards are - the work of culture's medium, or the kind of card. */
export type Grouping = 'topics' | 'kinds'

/** A board's cards grouped by what they are (v0.100): a card of an entry
 *  of the library under its medium's word, any other under its kind's -
 *  the groups in the order their first cards stand, as the topics are. */
export function kindsOf(
  board: Board,
  library: NoteKind | undefined,
  cards: readonly BoardCard[],
  say: (label: Board['label']) => string,
): Topic[] {
  const sorted = [...cards].sort((a, b) => a.position - b.position)
  const groups: Topic[] = []
  for (const card of sorted) {
    const medium = mediumOf(library, card.entry?.medium)
    const kind = kindOf(board, card)
    const name =
      medium !== undefined ? say(medium.label) : kind !== undefined ? say(kind.label) : card.kind
    const found = groups.find((group) => group.name === name)
    if (found === undefined) groups.push({ name, cards: [card] })
    else found.cards.push(card)
  }
  return groups
}

/** The works an entry on this board has worked in besides this one. */
export function elsewhereOf(view: BoardView | undefined, card: BoardCard): WorkedIn[] {
  if (card.note_id == null) return []
  return view?.elsewhere.find((one) => one.note_id === card.note_id)?.works ?? []
}

/** The profile's action that proposes cards for this board on this kind. */
export function boardActionOf(
  config: ProfileConfig,
  kind: string | undefined,
  board: string,
): PromptTemplate | undefined {
  return config.prompts.find(
    (prompt) =>
      prompt.scope === 'board' &&
      prompt.produces?.trim() === `cards:${board}` &&
      ((prompt.kinds ?? []).length === 0 || (kind !== undefined && prompt.kinds!.includes(kind))),
  )
}

/** The key a board's task runs under - the backend's `board_key`. */
export function boardTaskKey(
  action: string,
  workId: string,
  board: string,
  refines: boolean,
): string {
  return `${action}:board:${workId}:${refines ? 'own' : '-'}:${board}`
}

/** The work and the board a task key is about, when it is a board's. */
export function boardOfTask(key: string | undefined): { workId: string; board: string } | null {
  if (key === undefined) return null
  const parts = key.split(':')
  if (parts[1] !== 'board' || parts.length < 5) return null
  const workId = parts[2]!
  const board = parts.slice(4).join(':')
  return workId === '' || board === '' ? null : { workId, board }
}

/** What the composer asks for: `count` of the assistant's own, the person's
 *  words worked out when they asked for that and wrote any, where to dig, and
 *  whether to go further the way of what is starred. */
export function requestOf(
  count: number,
  words: string,
  refine: boolean,
  direction: string,
  more: boolean,
): CardRequest {
  const own = words.trim()
  const heading = direction.trim()
  return {
    count,
    refine: refine && own !== '' ? own : null,
    direction: heading === '' ? null : heading,
    more,
  }
}

/** How many cards a request makes - the skeletons drawn while it runs. */
export function totalOf(request: CardRequest): number {
  return request.count + (request.refine !== null && request.refine.trim() !== '' ? 1 : 0)
}

/** Where a drop lands, read off what the pointer was let go over: a card
 *  stands before the card it is dropped on, at the end of the topic it is
 *  dropped beside. Null for a drop outside the board. */
export function dropOf(target: Element | null): { topic: string; index: number } | null {
  const onCard = target?.closest<HTMLElement>('[data-card-slot]')
  if (onCard !== null && onCard !== undefined) {
    return {
      topic: onCard.dataset.cardTopic ?? '',
      index: Number(onCard.dataset.cardSlot ?? 0),
    }
  }
  const onTopic = target?.closest<HTMLElement>('[data-topic-drop]')
  if (onTopic !== null && onTopic !== undefined) {
    return { topic: onTopic.dataset.topicDrop ?? '', index: Number.MAX_SAFE_INTEGER }
  }
  return null
}
