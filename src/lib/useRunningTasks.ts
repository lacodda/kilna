import { useEffect, useMemo } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { activeTasks, type RunEmission } from '@/lib/api'
import { keys } from '@/lib/query'
import { movesTaskList } from '@/lib/tasks'

/**
 * The keys of the tasks in flight, kept current as runs start and end.
 *
 * Asked of the backend rather than kept by a component: a run started before
 * the screen was opened still owns its button, and a component's own state
 * cannot know that. `onEnded` hears every run that finishes or fails — the
 * moment an answer to show has arrived.
 */
export function useRunningTasks(onEnded?: (emission: RunEmission) => void): Set<string> {
  const client = useQueryClient()
  const running = useQuery({
    queryKey: keys.activeTasks,
    queryFn: activeTasks,
    staleTime: 0,
  })

  useEffect(() => {
    const subscription = listen<RunEmission>('assistant:run', ({ payload }) => {
      if (movesTaskList(payload)) void client.invalidateQueries({ queryKey: keys.activeTasks })
      if (payload.event.kind === 'finished' || payload.event.kind === 'failed') onEnded?.(payload)
    })
    return () => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    }
  }, [client, onEnded])

  return useMemo(() => new Set(running.data ?? []), [running.data])
}
