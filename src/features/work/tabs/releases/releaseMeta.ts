import { useQueryClient } from '@tanstack/react-query'
import type { PromptTemplate } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { releaseActionOf, useProfile } from '@/lib/useProfile'
import { useRunningTasks } from '@/lib/useRunningTasks'

/** The key a release's meta task runs under - the backend's `release_key`,
 *  built the same way so the button and the refusal of a second run agree. */
function releaseTaskKey(action: string, releaseId: string): string {
  return `${action}:release:${releaseId}`
}

/** The release a task key is about, when it is this action's task on one. */
function releaseOfTask(key: string | undefined, action: string): string | null {
  const prefix = `${action}:release:`
  return key?.startsWith(prefix) === true ? key.slice(prefix.length) : null
}

interface ReleaseMeta {
  /** The profile's action that writes a release's meta for this kind, if any. */
  action: PromptTemplate | undefined
  /** Whether that action is writing this release's meta right now. */
  writing: (releaseId: string) => boolean
}

/**
 * The release meta action of a kind of work, and which releases it is
 * writing now (v0.86).
 *
 * The running list is the backend's, as every task button reads it - a run
 * started from "Make…" before this screen opened still shows as running. When
 * one of its runs ends, the answer has landed: the empty fields are filled
 * and the started ones wait as a proposal, so what shows either is asked
 * again. `onEnded` hears which release, and whether the run finished rather
 * than failed.
 */
export function useReleaseMeta(
  kind: string | undefined,
  onEnded?: (releaseId: string, finished: boolean) => void,
): ReleaseMeta {
  const { config } = useProfile()
  const client = useQueryClient()
  const action = releaseActionOf(config, kind)

  const running = useRunningTasks((emission) => {
    if (action === undefined) return
    const releaseId = releaseOfTask(emission.task, action.key)
    if (releaseId === null) return
    // The fields and the proposals live under the releases' prefix; the
    // bell counts what waits, and the fill wrote a line in the journal.
    for (const key of [...refresh.release, keys.pendingProposals, keys.journal]) {
      void client.invalidateQueries({ queryKey: key })
    }
    onEnded?.(releaseId, emission.event.kind === 'finished')
  })

  return {
    action,
    writing: (releaseId) =>
      action !== undefined && running.has(releaseTaskKey(action.key, releaseId)),
  }
}
