import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { RunEmission } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { useRunEvent } from '@/lib/runEvents'

/**
 * The keys of the tasks in flight, kept current as runs start and end.
 *
 * Asked of the backend rather than kept by a component: a run started before
 * the screen was opened still owns its button, and a component's own state
 * cannot know that. The list is refreshed by the run events' bridge; `onEnded`
 * hears every run that finishes or fails — the moment an answer to show has
 * arrived.
 */
export function useRunningTasks(onEnded?: (emission: RunEmission) => void): Set<string> {
  const running = useQuery({ ...queries.activeTasks(), staleTime: 0 })

  useRunEvent((payload) => {
    if (payload.event.kind === 'finished' || payload.event.kind === 'failed') onEnded?.(payload)
  })

  return useMemo(() => new Set(running.data ?? []), [running.data])
}
