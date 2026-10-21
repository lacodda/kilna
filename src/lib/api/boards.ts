import { invoke } from '@tauri-apps/api/core'
import type {
  BoardCard,
  BoardView,
  CardPatch,
  CardRequest,
  CardVerdict,
  ComposedTask,
  Note,
  StartedTask,
  WorkedIn,
} from '@/lib/api/types'

// The boards of a work's conception (v0.99, ADR 0065).

/** Every board a work lays out, with the cards on each. */
export const workBoards = (id: string) => invoke<BoardView[]>('work_boards', { id })
/** The person's own card, in their words: its headline. */
export const addOwnCard = (workId: string, board: string, words: string, kind?: string) =>
  invoke<BoardCard>('add_own_card', { workId, board, kind: kind ?? null, words })
/** Change what a card says, or its kind. */
export const updateCard = (id: string, patch: CardPatch) =>
  invoke<BoardCard>('update_card', { id, patch })
/** Star a card, turn it down, or take the verdict back with `null`. */
export const judgeCard = (id: string, verdict: CardVerdict | null) =>
  invoke<BoardCard>('judge_card', { id, verdict })
/** Stand a card under a topic, at a place among that topic's cards. */
export const moveCard = (id: string, topic: string, index: number) =>
  invoke<void>('move_card', { id, topic, index })
/** Rename a topic, or merge it into another by that one's name. */
export const retopicCards = (workId: string, board: string, from: string, to: string) =>
  invoke<void>('retopic_cards', { workId, board, from, to })
/** Off the board, into the trash. */
export const deleteCard = (id: string) => invoke<string>('delete_card', { id })
/** The whole order a writer works from, in markdown. */
export const conceptionOrder = (id: string) => invoke<string>('conception_order', { id })
/** A chip of a card kept as a phrase of its work (v0.100, ADR 0066). */
export const chipToPhrase = (cardId: string, chip: string) =>
  invoke<Note>('chip_to_phrase', { cardId, chip })
/** The works an entry of the library has worked in. */
export const entryWorkedIn = (id: string) => invoke<WorkedIn[]>('entry_worked_in', { id })

/** Ask a board for cards in the background; they land on it as they come. */
export const startBoardTask = (id: string, action: string, board: string, request: CardRequest) =>
  invoke<StartedTask>('start_board_task', { id, action, board, request })
export const previewBoardTask = (id: string, action: string, request: CardRequest) =>
  invoke<ComposedTask>('preview_board_task', { id, action, request })
