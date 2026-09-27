import { createContext, useCallback, useContext, useEffect, useRef, type ReactNode } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useQueryClient, type QueryClient } from '@tanstack/react-query'
import type { Run, RunEmission, TaskQueue } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { withEvent } from '@/lib/runs'
import { movesTaskList } from '@/lib/tasks'

type Listener = (emission: RunEmission) => void

const RunEvents = createContext<((listener: Listener) => () => void) | null>(null)

/**
 * What the cache learns from one event of a run, wherever the window is.
 *
 * Every event lands on its run in the chat's list, so a run reads the same
 * whether its events arrived live or were replayed from storage. A run
 * starting or ending moves the lists of what is running, queued and waiting;
 * a run ending wrote an answer, so the chat's transcript and the chat lists'
 * captions and prices moved with it.
 */
function settle(client: QueryClient, emission: RunEmission) {
  client.setQueryData<Run[]>(keys.runs(emission.chat_id), (previous) =>
    previous?.map((run) => (run.id === emission.run_id ? withEvent(run, emission.event) : run)),
  )
  if (movesTaskList(emission)) {
    for (const key of [keys.activeRuns, keys.activeTasks, keys.taskQueue, keys.waitingChats]) {
      void client.invalidateQueries({ queryKey: key })
    }
  }
  if (emission.event.kind === 'finished' || emission.event.kind === 'failed') {
    void client.invalidateQueries({ queryKey: keys.transcript(emission.chat_id) })
    void client.invalidateQueries({ queryKey: keys.allChats })
  }
}

/**
 * The one ear on the assistant's runs.
 *
 * The backend says what a run is doing on `assistant:run`, and what the queue
 * holds on `assistant:queue`. Until v0.77 seven components listened for
 * themselves, each refreshing its own corner of the cache - the same event
 * handled seven ways, and a screen that forgot to listen showed a stale list.
 * Now the cache is kept here, once, for every screen; a component that must
 * *do* something when a run ends - say so, open an answer - asks
 * `useRunEvent` instead of listening.
 */
export function RunEventsBridge({ children }: { children: ReactNode }) {
  const client = useQueryClient()
  const listeners = useRef(new Set<Listener>())

  useEffect(() => {
    const runs = listen<RunEmission>('assistant:run', ({ payload }) => {
      settle(client, payload)
      for (const listener of listeners.current) listener(payload)
    })
    const queue = listen<TaskQueue>('assistant:queue', ({ payload }) => {
      client.setQueryData(keys.taskQueue, payload)
    })
    return () => {
      void runs.then((unlisten) => unlisten())
      void queue.then((unlisten) => unlisten())
    }
  }, [client])

  const subscribe = useCallback((listener: Listener) => {
    listeners.current.add(listener)
    return () => {
      listeners.current.delete(listener)
    }
  }, [])

  return <RunEvents value={subscribe}>{children}</RunEvents>
}

/**
 * Hear every event of every run, after the cache has taken it in. The handler
 * may change between renders; the latest one is called.
 */
export function useRunEvent(listener: Listener): void {
  const subscribe = useContext(RunEvents)
  const latest = useRef(listener)
  useEffect(() => {
    latest.current = listener
  })
  useEffect(() => subscribe?.((emission) => latest.current(emission)), [subscribe])
}
