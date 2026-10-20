import { useEffect, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { startBoardTask } from '@/lib/api/boards'
import { stopTask } from '@/lib/api/ideas'
import type { Board, BoardView, CardRequest, PromptTemplate, Work } from '@/lib/api/types'
import { boardActionOf, boardOfTask, boardTaskKey, CARDS_ON_ASK, totalOf } from '@/lib/boards'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useProfile } from '@/lib/useProfile'
import { useRunningTasks } from '@/lib/useRunningTasks'

// What a board remembers between its tab being left and opened again, for as
// long as the window lives - how many cards the last ask was for, when this
// window first saw the run going, the cards that arrived while it was looked
// at. None of it is a fact about the workspace: a restart breaks every run in
// flight, so none of it is stored (the cover's board keeps the same).
const asked = new Map<string, number>()
const seen = new Map<string, number>()
const arrived = new Map<string, Set<string>>()
const NONE: ReadonlySet<string> = new Set()

export interface BoardTask {
  /** The profile's action that proposes cards for this board, if any. */
  action: PromptTemplate | undefined
  /** A run is writing cards for this board now. */
  writing: boolean
  /** How many cards that run is writing: the skeletons to draw. */
  coming: number
  /** When this window first saw the run going, for the clock. */
  since: number | null
  /** Cards that arrived while the board was watched: they wear a badge. */
  fresh: ReadonlySet<string>
  ask: (request: CardRequest) => void
  asking: boolean
  /** A run of this board was started elsewhere - from the preview. */
  started: (request: CardRequest) => void
  stop: () => void
}

/**
 * Asking a board of the conception for cards, and watching the answer land
 * (v0.99, ADR 0065) - what `useCoverIdeas` is for a cover's board.
 *
 * Running is read off the backend's list of tasks, by the board's key with or
 * without one of the person's own cards to work out; when a run of this board
 * ends its cards are already on it - the backend put them there as the answer
 * came - so the board is asked again, and the cards that were not on it
 * before wear a badge.
 */
export function useBoardTask(work: Work, board: Board): BoardTask {
  const { config } = useProfile()
  const client = useQueryClient()
  const action = boardActionOf(config, work.kind, board.key)
  const slot = `${work.id}:${board.key}`
  const keysOf =
    action === undefined
      ? []
      : [true, false].map((refines) => boardTaskKey(action.key, work.id, board.key, refines))
  const views = useQuery(queries.boards(work.id))
  const [, setTick] = useState(0)

  const running = useRunningTasks((emission) => {
    const about = boardOfTask(emission.task)
    if (about === null || about.workId !== work.id || about.board !== board.key) return
    const cardsOf = (list: BoardView[] | undefined) =>
      (list ?? []).find((view) => view.board === board.key)?.cards ?? []
    const before = new Set(cardsOf(views.data).map((card) => card.id))
    void client.invalidateQueries({ queryKey: keys.boards(work.id) }).then(() => {
      const after = cardsOf(client.getQueryData<BoardView[]>(keys.boards(work.id)))
      const added = after.map((card) => card.id).filter((id) => !before.has(id))
      if (added.length > 0) {
        arrived.set(slot, new Set([...(arrived.get(slot) ?? []), ...added]))
        setTick((tick) => tick + 1)
      }
    })
    for (const other of [keys.pendingProposals, keys.journal, keys.work(work.id)]) {
      void client.invalidateQueries({ queryKey: other })
    }
  })
  const writing = keysOf.some((key) => running.has(key))

  // The clock counts from the first moment this window saw the run; it ticks
  // while the run goes, and forgets the moment when it ends.
  useEffect(() => {
    if (!writing) {
      seen.delete(slot)
      return undefined
    }
    if (!seen.has(slot)) seen.set(slot, Date.now())
    const timer = window.setInterval(() => setTick((tick) => tick + 1), 1000)
    return () => window.clearInterval(timer)
  }, [slot, writing])

  /** What a started run will write, for the skeletons and the clock. */
  const note = (request: CardRequest) => {
    asked.set(slot, totalOf(request))
    if (!seen.has(slot)) seen.set(slot, Date.now())
  }

  const start = useAppMutation({
    mutationFn: (request: CardRequest) => {
      if (action === undefined) return Promise.reject(new Error('no action'))
      return startBoardTask(work.id, action.key, board.key, request)
    },
    failure: 'board.askFailed',
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: (_, request) => note(request),
  })

  const stopping = useAppMutation({
    mutationFn: async () => {
      for (const key of keysOf) if (running.has(key)) await stopTask(key)
    },
    failure: 'board.stopFailed',
    refresh: [keys.activeTasks],
  })

  return {
    action,
    writing,
    coming: asked.get(slot) ?? CARDS_ON_ASK,
    since: seen.get(slot) ?? null,
    fresh: arrived.get(slot) ?? NONE,
    ask: (request) => start.mutate(request),
    asking: start.isPending,
    started: note,
    stop: () => stopping.mutate(undefined),
  }
}
