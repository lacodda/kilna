import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { ScoredWork } from '@/lib/api/types'
import { subjectOf } from '@/lib/dashboard'
import { queries } from '@/lib/query/queries'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { Skeleton } from '@/components/ui/skeleton'
import { Loaded } from '@/components/Loaded'
import { Widget } from '@/features/dashboard/Widget'

interface Props {
  works: readonly ScoredWork[]
  onSelect: (workId: string, tab?: string) => void
}

/**
 * What the assistant is doing right now, and how much is waiting its turn.
 *
 * The queue rather than `activeTasks`: that list folds the waiting into the
 * running, which is the right answer for a button ("already asked for?") and
 * the wrong one here - a task still in the queue drawn with a moving bar would
 * claim work that has not started. Kept current by the run events' bridge, as
 * the queue banner is.
 *
 * The bars do not fill. A run is a conversation with a model, and how much of
 * it is left is known to nobody - dowel's Progress says exactly that when it
 * is given no value, and a number here would be invented.
 */
export function RunningWidget({ works, onSelect }: Props) {
  const { t } = useTranslation()
  const queue = useQuery({ ...queries.taskQueue(), staleTime: 0 })

  return (
    <Widget title={t('dashboard.running')}>
      <Loaded query={queue} plain skeleton={<Skeleton className="h-4 w-full" />}>
        {({ running, waiting }) => (
          <>
            {running.length === 0 ? (
              <p className="text-sm text-faint">{t('dashboard.idle')}</p>
            ) : (
              <ul className="flex flex-col gap-2.5">
                {running.map((key) => (
                  <Task key={key} taskKey={key} works={works} onSelect={onSelect} />
                ))}
              </ul>
            )}
            {waiting.length > 0 && (
              <p className="text-xs text-faint">
                {t('assistant.queueWaiting', { count: waiting.length })}
              </p>
            )}
          </>
        )}
      </Loaded>
    </Widget>
  )
}

/** One task in flight: the action, the work it is about, a bar that moves. */
function Task({ taskKey, works, onSelect }: { taskKey: string } & Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const subject = subjectOf(taskKey)
  const action = labelOf(profile.config.prompts, subject.action)
  const work = works.find((row) => row.work_id === subject.workId)

  return (
    <li className="flex min-w-0 flex-col gap-1.5">
      {/* The answer lands in the work's chat, so that is where the line goes.
          A task about a comment or a screenshot has no work to open, and
          says only what it is doing. */}
      {work === undefined ? (
        <span className="truncate text-sm font-semibold">{action}</span>
      ) : (
        <Button
          variant="link"
          onClick={() => onSelect(work.work_id, 'assistant')}
          className="min-w-0 justify-start truncate text-sm font-semibold text-text"
        >
          {action} · {work.title}
        </Button>
      )}
      <Progress size="sm" tone="warn" label={t('assistant.actionWorking', { label: action })} />
    </li>
  )
}
