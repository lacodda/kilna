import { useEffect, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { startCoverTask, stopTask } from '@/lib/api/ideas'
import type { CoverBoard, IdeaRequest, PromptTemplate, Work } from '@/lib/api/types'
import { boardOfTask, coverActionOf, coverTaskKey, ideasOnMake, totalOf } from '@/lib/ideas'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useProfile } from '@/lib/useProfile'
import { useRunningTasks } from '@/lib/useRunningTasks'

// What a board remembers between its tab being left and opened again, for
// as long as the window lives: how many ideas the last ask on each board was
// for, when this window first saw each board's run going, and the ideas that
// arrived while the person was looking. None of it is a fact about the
// workspace - a restart breaks every run in flight - so none of it is stored.
const asked = new Map<string, number>()
const seen = new Map<string, number>()
const arrived = new Map<string, Set<string>>()
const NONE: ReadonlySet<string> = new Set()

/** The moment a board's run was first seen going, kept for its clock. */
function stamp(key: string) {
  if (!seen.has(key)) seen.set(key, Date.now())
}

export interface CoverIdeas {
  /** The profile's action that proposes ideas for this kind, if any. */
  action: PromptTemplate | undefined
  /** A run is writing ideas for this board now. */
  writing: boolean
  /** How many ideas that run is writing: the skeletons to draw. */
  coming: number
  /** When this window first saw the run going, for the clock. */
  since: number | null
  /** Ideas that arrived while the board was watched: they wear a badge. */
  fresh: ReadonlySet<string>
  ask: (request: IdeaRequest) => void
  asking: boolean
  /** A run of this board was started elsewhere - from the preview. */
  started: (request: IdeaRequest) => void
  stop: () => void
}

/**
 * Asking a publication's board for ideas, and watching the answer land
 * (v0.89, ADR 0050).
 *
 * The running list is the backend's, as every task button reads it: a run
 * that "Make…" started before this tab was opened shows as running here. When
 * a run of this board ends, its ideas are already on the board - the backend
 * put them there as the answer came - so the board is asked again, and the
 * ideas that were not on it before wear a badge.
 */
export function useCoverIdeas(work: Work): CoverIdeas {
  const { config } = useProfile()
  const client = useQueryClient()
  const action = coverActionOf(config, work.kind)
  const key = action === undefined ? null : coverTaskKey(action.key, work.id)
  const board = useQuery(queries.coverBoard(work.id))
  const [, setTick] = useState(0)

  const running = useRunningTasks((emission) => {
    if (boardOfTask(emission.task) !== work.id) return
    const before = new Set((board.data?.ideas ?? []).map((card) => card.idea.id))
    void client.invalidateQueries({ queryKey: keys.coverBoard(work.id) }).then(() => {
      const after = client.getQueryData<CoverBoard>(keys.coverBoard(work.id))
      const added = (after?.ideas ?? []).map((card) => card.idea.id).filter((id) => !before.has(id))
      if (added.length > 0) {
        arrived.set(work.id, new Set([...(arrived.get(work.id) ?? []), ...added]))
        setTick((tick) => tick + 1)
      }
    })
    for (const other of [keys.pendingProposals, keys.journal]) {
      void client.invalidateQueries({ queryKey: other })
    }
  })
  const writing = key !== null && running.has(key)

  // The clock counts from the first moment this window saw the run, which is
  // the moment it was asked for when it was asked for here; it ticks while
  // the run goes, and forgets the moment when it ends.
  useEffect(() => {
    if (key === null) return undefined
    if (!writing) {
      seen.delete(key)
      return undefined
    }
    stamp(key)
    const timer = window.setInterval(() => setTick((tick) => tick + 1), 1000)
    return () => window.clearInterval(timer)
  }, [key, writing])

  const start = useAppMutation({
    mutationFn: (request: IdeaRequest) => {
      if (action === undefined) return Promise.reject(new Error('no action'))
      return startCoverTask(work.id, action.key, request)
    },
    failure: 'ideas.askFailed',
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: (_, request) => note(request),
  })

  /** What a started run will write, for the skeletons and the clock. */
  function note(request: IdeaRequest) {
    asked.set(work.id, totalOf(request))
    if (key !== null) stamp(key)
  }

  const stopping = useAppMutation({
    mutationFn: () => (key === null ? Promise.resolve(false) : stopTask(key)),
    failure: 'ideas.stopFailed',
    refresh: [keys.activeTasks],
  })

  return {
    action,
    writing,
    coming: asked.get(work.id) ?? (ideasOnMake(config) || 1),
    since: key === null ? null : (seen.get(key) ?? null),
    fresh: arrived.get(work.id) ?? NONE,
    ask: (request) => start.mutate(request),
    asking: start.isPending,
    started: note,
    stop: () => stopping.mutate(),
  }
}

/** The run's time so far, as the board's line says it: `0:12`. */
export function clockOf(since: number, now: number = Date.now()): string {
  const seconds = Math.max(0, Math.floor((now - since) / 1000))
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
}
