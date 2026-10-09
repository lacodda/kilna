import { useEffect, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { startLabTask } from '@/lib/api/trials'
import { stopTask } from '@/lib/api/ideas'
import type { PromptTemplate, TrialBoard, TrialRequest, Work } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { aroundOfLabTask, boardOfLabTask, labActionOf, labTaskKey, totalOf } from '@/lib/trials'
import { useProfile } from '@/lib/useProfile'
import { useRunningTasks } from '@/lib/useRunningTasks'

// What a board remembers between its tab being left and opened again, for as
// long as the window lives: how many trials each run was asked for, and the
// trials that arrived while the person was looking. A restart breaks every
// run in flight, so none of it is stored.
const asked = new Map<string, number>()
const arrived = new Map<string, Set<string>>()
const NONE: ReadonlySet<string> = new Set()

/** A run writing trials for this board now. */
export interface LabRun {
  key: string
  /** The trial it varies or fixes, when it is about one. */
  around: string | null
  /** How many trials it writes, when this window asked for them. */
  coming: number
}

export interface LabTask {
  /** The profile's action that proposes trials for this kind, if any. */
  action: PromptTemplate | undefined
  /** The runs writing for this board now. */
  runs: LabRun[]
  /** Trials that arrived while the board was watched: they wear a badge. */
  fresh: ReadonlySet<string>
  ask: (request: TrialRequest) => void
  asking: boolean
  /** A run of this board was started elsewhere - from the preview. */
  started: (request: TrialRequest) => void
  stop: (key: string) => void
}

/**
 * Asking an experiment's board for trials, and watching them land (v0.95,
 * ADR 0061) - the way a cover's board asks for ideas.
 *
 * A board may have several runs at once: a sweep of the field, and the fix
 * of a trial heard meanwhile. Each runs under a key that says what it asked,
 * so the board reads its runs off the running list by the key's prefix, and
 * a trial shows its own run - a variation being written - by the key's
 * `around`.
 */
export function useLabTask(work: Work): LabTask {
  const { config } = useProfile()
  const client = useQueryClient()
  const action = labActionOf(config, work.kind)
  const [, setTick] = useState(0)

  const running = useRunningTasks((emission) => {
    if (boardOfLabTask(emission.task) !== work.id) return
    const before = new Set(
      (client.getQueryData<TrialBoard>(keys.trialBoard(work.id))?.trials ?? []).map(
        (card) => card.trial.id,
      ),
    )
    void client.invalidateQueries({ queryKey: keys.trialBoard(work.id) }).then(() => {
      const after = client.getQueryData<TrialBoard>(keys.trialBoard(work.id))
      const added = (after?.trials ?? [])
        .map((card) => card.trial.id)
        .filter((id) => !before.has(id))
      if (added.length > 0) {
        arrived.set(work.id, new Set([...(arrived.get(work.id) ?? []), ...added]))
        setTick((tick) => tick + 1)
      }
    })
    for (const other of [keys.pendingProposals, keys.journal]) {
      void client.invalidateQueries({ queryKey: other })
    }
  })

  const prefix = action === undefined ? null : `${action.key}:lab:${work.id}:`
  const runs: LabRun[] =
    prefix === null
      ? []
      : [...running]
          .filter((key) => key.startsWith(prefix))
          .map((key) => ({ key, around: aroundOfLabTask(key), coming: asked.get(key) ?? 1 }))

  // A run started here is remembered by its key, so its rows are drawn for
  // as long as it goes; forgotten once it is not running.
  useEffect(() => {
    for (const key of [...asked.keys()]) {
      if (key.startsWith(`${action?.key ?? ''}:lab:${work.id}:`) && !running.has(key)) {
        asked.delete(key)
      }
    }
  }, [running, action, work.id])

  const start = useAppMutation({
    mutationFn: (request: TrialRequest) => {
      if (action === undefined) return Promise.reject(new Error('no action'))
      return startLabTask(work.id, action.key, request)
    },
    failure: 'trials.askFailed',
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: (started, request) => {
      asked.set(started.taskKey, totalOf(request))
    },
  })

  const stopping = useAppMutation({
    mutationFn: (key: string) => stopTask(key),
    failure: 'trials.stopFailed',
    refresh: [keys.activeTasks],
  })

  return {
    action,
    runs,
    fresh: arrived.get(work.id) ?? NONE,
    ask: (request) => start.mutate(request),
    asking: start.isPending,
    started: (request) => {
      if (action !== undefined)
        asked.set(labTaskKey(action.key, work.id, request), totalOf(request))
    },
    stop: (key) => stopping.mutate(key),
  }
}
