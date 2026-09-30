import { invoke } from '@tauri-apps/api/core'
import type {
  Asset,
  CanonLink,
  CanonLinkPatch,
  CanonProposal,
  CardFilter,
  CardSummary,
  CardView,
  ComposedTask,
  Dated,
  Fact,
  FactPatch,
  FactReview,
  Lens,
  NewAsset,
  NewCanonLink,
  NewFact,
  NewNote,
  Note,
  StartedTask,
} from '@/lib/api/types'

// The canon: cards, their facts and relations, their pictures, and the
// assistant's proposals for them (ADR 0043).

export const listCards = (filter?: CardFilter) => invoke<CardSummary[]>('list_cards', { filter })
export const readCard = (id: string) => invoke<CardView>('read_card', { id })
/** A card as a task's prompt receives it - the text the assistant is handed. */
export const cardAsSeen = (id: string, lens: Lens) => invoke<string>('card_as_seen', { id, lens })
export const canonTimeline = (noteId?: string) =>
  invoke<Dated[]>('canon_timeline', { noteId: noteId ?? null })
/** A template with its `[[card:id]]` references as a generator reads them. */
export const expandCardReferences = (text: string) =>
  invoke<string>('expand_card_references', { text })
export const createCard = (card: NewNote) => invoke<Note>('create_card', { card })
/** Write a card's description for a picture generator by hand; null clears it. */
export const describeCard = (id: string, text: string | null) =>
  invoke<Note>('describe_card', { id, text })
export const addFact = (fact: NewFact) => invoke<Fact>('add_fact', { fact })
export const updateFact = (id: string, patch: FactPatch) =>
  invoke<Fact>('update_fact', { id, patch })
export const retireFact = (id: string, reason: string) =>
  invoke<Fact>('retire_fact', { id, reason })
export const reorderFacts = (noteId: string, section: string, ids: string[]) =>
  invoke<void>('reorder_facts', { noteId, section, ids })
/** Returns the trash entry, which is what the undo on the toast restores. */
export const deleteFact = (id: string) => invoke<string>('delete_fact', { id })
export const relateCards = (link: NewCanonLink) => invoke<CanonLink>('relate_cards', { link })
export const updateRelation = (id: string, patch: CanonLinkPatch) =>
  invoke<CanonLink>('update_relation', { id, patch })
export const unrelateCards = (id: string) => invoke<void>('unrelate_cards', { id })
/** A picture pasted onto a card, or onto one of its facts. */
export const pasteCardPicture = (picture: NewAsset, bytes: number[], name: string) =>
  invoke<Asset>('paste_card_picture', { picture, bytes, name })
export const setPictureRole = (id: string, role: string) =>
  invoke<Asset>('set_picture_role', { id, role })
/** Every proposal for the canon nobody has answered, with the cards each touches. */
export const pendingCanonProposals = () => invoke<CanonProposal[]>('pending_canon_proposals')
/** A proposal for the canon read against the canon as it stands now. */
export const reviewCanonProposal = (messageId: string) =>
  invoke<FactReview[]>('review_canon_proposal', { messageId })
/** Gather a card's facts out of its note, or describe it for a generator. */
export const startCardTask = (id: string, action: string) =>
  invoke<StartedTask>('start_card_task', { id, action })
export const previewCardTask = (id: string, action: string) =>
  invoke<ComposedTask>('preview_card_task', { id, action })
