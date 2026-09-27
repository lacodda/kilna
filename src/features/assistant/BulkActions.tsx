import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { actionsOfScope } from '@/lib/actions'
import { startTasks } from '@/lib/api/assistant'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'

interface Props {
  /** The works ticked right now. */
  workIds: readonly string[]
  /** Called once the batch is away, so the catalogue can drop its selection. */
  onStarted: () => void
}

/**
 * The profile's actions, run against everything ticked.
 *
 * The same actions a card offers, asked of many works at once. Nothing new
 * happens per work — each one gets exactly the task a click on its own card
 * would have made, which is why the answers land in the same place and can be
 * applied the same way.
 *
 * Only three runs may be alive at once, so a batch larger than that queues. The
 * one number worth reporting is how much of it is waiting: three started and
 * thirty-seven queued is a different afternoon from forty started.
 */
export function BulkActions({ workIds, onStarted }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  // Without the CLI there is nothing to ask. Asked here rather than assumed
  // from the panel: the catalogue can be used with the panel never opened.
  const status = useQuery({ ...queries.assistantStatus(), staleTime: 60_000 })

  const start = useAppMutation({
    mutationFn: (action: string) => startTasks(workIds, action),
    refresh: [keys.activeTasks, keys.taskQueue, keys.allChats],
    onSuccess: (batch) => {
      // Every work was already going or could not start: saying "0 started"
      // and clearing the ticks would look like it worked.
      if (batch.started + batch.queued === 0) {
        say.info(t('assistant.batchNothing'))
        return
      }

      say.ok(
        batch.queued > 0
          ? t('assistant.batchQueued', { started: batch.started, queued: batch.queued })
          : t('assistant.batchStarted', { count: batch.started }),
      )
      onStarted()
    },
  })

  // Work actions only: several works at once is several work tasks, and an
  // action about a scene, a style or a comment has none of those to be about.
  const actions = actionsOfScope(profile.config.prompts, 'work')
  if (actions.length === 0 || status.data?.available !== true) return null

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <span className="text-xs text-dim">{t('assistant.askFor')}</span>
      {actions.map((action) => (
        <Button
          key={action.key}
          size="sm"
          title={sayLabel(action.description)}
          disabled={start.isPending}
          onClick={() => {
            start.mutate(action.key)
          }}
        >
          {sayLabel(action.label)}
        </Button>
      ))}
    </div>
  )
}
