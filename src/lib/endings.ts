import type { TFunction } from 'i18next'
import type { Activity, RunEmission } from '@/lib/api/types'

/**
 * Runs that ended while nobody was looking, and how to say them (v0.107).
 *
 * In front of the window an ending is a toast, said as it happens. Out of
 * sight it becomes a notification, and forty of those for a batch of forty
 * would be a machine shouting. So endings out of sight are held: said at
 * once when nothing else is going - one task, the usual case, is told the
 * moment it ends - and otherwise gathered and said together, at the latest a
 * minute and a half after the first of them, and as soon as the last run of
 * the batch is over.
 */
export interface Ending {
  chatId: string
  /** The chat's name, when the window had it. */
  title: string | null
  /** An answer came, or the run stopped without one. */
  outcome: 'done' | 'ended'
  /** Started as a task - asked to be walked away from - or typed in a chat. */
  task: boolean
}

/** How long an ending may wait for others to join it. */
export const HOLD_MS = 90_000

/** The ending an event carries, or null for one that ends nothing - a
 *  stopped run included: the person who stopped it knows. */
export function endingOf(emission: RunEmission, title: string | null): Ending | null {
  const { kind } = emission.event
  if (kind !== 'finished' && kind !== 'failed') return null
  return {
    chatId: emission.chat_id,
    title,
    outcome: kind === 'finished' ? 'done' : 'ended',
    task: emission.task !== undefined,
  }
}

/** Whether nothing is going any more, so what is held can be said now. */
export function isIdle(activity: Activity | undefined): boolean {
  return activity !== undefined && activity.running === 0 && activity.queued === 0
}

/** How many names a gathered notification lists before "and N more". */
const NAMED = 3

/** The words for what was held: one ending as the toast would say it,
 *  several as a count with their names. */
export function sayEndings(
  endings: readonly Ending[],
  t: TFunction,
): { title: string; body?: string } {
  const only = endings[0]
  if (endings.length === 1 && only !== undefined) {
    const { title } = only
    if (only.outcome === 'ended') {
      return {
        title: title === null ? t('assistant.taskEnded') : t('assistant.taskEndedNamed', { title }),
      }
    }
    if (only.task) {
      return {
        title: title === null ? t('assistant.taskDone') : t('assistant.taskDoneNamed', { title }),
      }
    }
    return {
      title: title === null ? t('outside.answeredUntitled') : t('outside.answered', { title }),
    }
  }

  const failed = endings.filter((ending) => ending.outcome === 'ended').length
  const names = endings
    .map((ending) => ending.title)
    .filter((title): title is string => title !== null)
  const shown = names.slice(0, NAMED).join(', ')
  const rest = endings.length - Math.min(names.length, NAMED)
  const listed =
    shown === '' ? null : rest > 0 ? `${shown} ${t('outside.andMore', { count: rest })}` : shown
  const done = endings.length - failed
  // The heading is what came back; what did not is the line under it - or
  // the heading itself when nothing came back at all.
  const title =
    done > 0 ? t('outside.endedCount', { count: done }) : t('outside.someFailed', { count: failed })
  const lines = [
    listed,
    done > 0 && failed > 0 ? t('outside.someFailed', { count: failed }) : null,
  ].filter((line): line is string => line !== null)
  return { title, body: lines.length === 0 ? undefined : lines.join('\n') }
}
