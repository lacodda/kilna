import type { RunEmission } from '@/lib/api'

/**
 * What a task is, as a key: this action, on this work.
 *
 * Built the same way here and in the backend, because both sides need the same
 * answer to the same question — is this button already working? The frontend
 * asks it to draw the button, the backend asks it to refuse a second run.
 */
export function taskKey(action: string, workId: string): string {
  return `${action}:${workId}`
}

/**
 * What a finished task should say, or null when nothing should be said.
 *
 * A task exists to be walked away from, so the ending has to come find the
 * person. Everything else stays quiet: a typed prompt was asked in a chat
 * someone is looking at, and a run that has only started has nothing to report.
 * A task stopped by hand is silent too — the person who stopped it knows.
 */
export function announcement(emission: RunEmission): 'done' | 'ended' | null {
  if (emission.task === undefined) return null

  switch (emission.event.kind) {
    case 'finished':
      return 'done'
    case 'failed':
      return 'ended'
    default:
      return null
  }
}

/** Whether this emission moves the set of tasks in flight. */
export function movesTaskList(emission: RunEmission): boolean {
  const { kind } = emission.event
  return kind === 'started' || kind === 'finished' || kind === 'failed' || kind === 'stopped'
}
