import { invoke } from '@tauri-apps/api/core'
import type {
  ComposedTask,
  CoverBoard,
  CoverIdea,
  IdeaRequest,
  IdeaVerdict,
  StartedTask,
  Work,
} from '@/lib/api/types'

// A publication's board of ideas for its cover (v0.89, ADR 0050).

/** The ideas on a publication's board, and its neighbours' covers. */
export const coverBoard = (id: string) => invoke<CoverBoard>('cover_board', { id })
/** The person's own idea, in their words, on the board. */
export const addOwnIdea = (workId: string, words: string) =>
  invoke<CoverIdea>('add_own_idea', { workId, words })
/** Star an idea, turn it down, or take the verdict back with `null`. */
export const judgeIdea = (id: string, verdict: IdeaVerdict | null) =>
  invoke<CoverIdea>('judge_idea', { id, verdict })
/** A neighbour's cover, copied onto the board with the verdict. */
export const judgeSiblingCover = (workId: string, siblingId: string, verdict: IdeaVerdict) =>
  invoke<CoverIdea>('judge_sibling_cover', { workId, siblingId, verdict })
/** The cover becomes what the idea decides; logged as an edit of the work. */
export const takeIdea = (id: string) => invoke<Work>('take_idea', { id })
export const takeSiblingCover = (workId: string, siblingId: string) =>
  invoke<Work>('take_sibling_cover', { workId, siblingId })
/** Off the board, into the trash. */
export const deleteIdea = (id: string) => invoke<string>('delete_idea', { id })

/** Ask for ideas in the background; they land on the board as they come. */
export const startCoverTask = (id: string, action: string, request: IdeaRequest) =>
  invoke<StartedTask>('start_cover_task', { id, action, request })
export const previewCoverTask = (id: string, action: string, request: IdeaRequest) =>
  invoke<ComposedTask>('preview_cover_task', { id, action, request })
/** Stop a task by its key, whoever started it. Says whether one was going. */
export const stopTask = (key: string) => invoke<boolean>('stop_task', { key })
