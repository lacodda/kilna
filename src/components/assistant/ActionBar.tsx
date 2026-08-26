import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { activeTasks, startTask, type RunEmission } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { movesTaskList, taskKey } from '@/lib/tasks'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'

interface Props {
  workId: string
}

/**
 * The profile's actions, started from the work rather than from the panel.
 *
 * The same actions live in the assistant tab, where clicking one fills the
 * composer so the prompt can be read before it is paid for. This bar is the
 * other way of using them: hands on the work, wanting the thing done, not
 * wanting to move. A click starts a run and says where it went — nothing to
 * watch, nothing to wait for.
 *
 * The answer lands in a chat of its own, never in whatever conversation
 * happened to be open. Two reasons: a task dropped into a live thread inherits
 * that thread's session as context, and it buries the answer in someone else's
 * subject.
 */
export function ActionBar({ workId }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()

  // Which actions are already going. Asked of the backend rather than kept
  // here: a run started before this card was opened still owns its button, and
  // a component's own state cannot know that.
  const running = useQuery({
    queryKey: keys.activeTasks,
    queryFn: activeTasks,
    staleTime: 0,
  })

  // A task ends without anyone watching this bar, and its button has to come
  // back by itself. Only run boundaries move the list.
  useEffect(() => {
    const subscription = listen<RunEmission>('assistant:run', ({ payload }) => {
      if (movesTaskList(payload)) {
        void client.invalidateQueries({ queryKey: keys.activeTasks })
      }
    })
    return () => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    }
  }, [client])

  const start = useMutation({
    mutationFn: (action: string) => startTask(workId, action),
    onSuccess: (started) => {
      void client.invalidateQueries({ queryKey: keys.activeTasks })
      void client.invalidateQueries({ queryKey: keys.allChats })
      say.info(t('assistant.taskStarted', { title: started.title }))
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  const actions = profile.config.prompts
  if (actions.length === 0) return null

  const busy = new Set(running.data ?? [])
  // The action whose start has not come back yet. Held only for that moment:
  // once the backend answers, the list of running tasks is what the buttons
  // read, and this goes back to null whether the start succeeded or failed.
  const pending = start.isPending ? start.variables : null

  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-sm font-semibold">{t('assistant.actions')}</h3>

      <div className="flex flex-wrap gap-1.5">
        {actions.map((action) => {
          // The key the backend refuses duplicates by, built the same way on
          // both sides. Only this button's own task disables it: three runs
          // may go at once, and greying out the whole row because one action
          // was clicked would say otherwise.
          //
          // The list is the single source of that answer — a click that is
          // still in flight is covered by `pending` rather than by a
          // second piece of state that would have to be cleared by hand.
          const working = busy.has(taskKey(action.key, workId)) || pending === action.key

          return (
            <Button
              key={action.key}
              size="sm"
              title={action.description}
              disabled={working}
              onClick={() => {
                start.mutate(action.key)
              }}
            >
              {working ? t('assistant.actionWorking', { label: action.label }) : action.label}
            </Button>
          )
        })}
      </div>
    </section>
  )
}
