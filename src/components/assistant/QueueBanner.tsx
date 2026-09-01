import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Layers } from 'lucide-react'
import { clearTaskQueue, taskQueue, type RunEmission, type TaskQueue } from '@/lib/api'
import { keys } from '@/lib/query'
import { movesTaskList } from '@/lib/tasks'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'

/**
 * How much of a batch is still to come.
 *
 * A batch is asked for in one click and then takes an hour: three runs at a
 * time, the rest waiting. Without this the application looks idle between
 * answers, and the honest question — "is it still going, and how much is
 * left?" — has nowhere to be asked.
 *
 * Shown from every screen for the same reason the waiting banner is: the batch
 * was started to be walked away from. It disappears by itself when the last
 * task is picked up, because at that point nothing is waiting and the runs
 * still going speak for themselves in the panel.
 */
export function QueueBanner() {
  const { t } = useTranslation()
  const client = useQueryClient()

  const queue = useQuery({
    queryKey: keys.taskQueue,
    queryFn: taskQueue,
    staleTime: 0,
  })

  // Two sources, because the queue moves for two reasons: a run ending frees a
  // slot, and the backend says so directly when a batch is started or cleared.
  useEffect(() => {
    const runs = listen<RunEmission>('assistant:run', ({ payload }) => {
      if (movesTaskList(payload)) {
        void client.invalidateQueries({ queryKey: keys.taskQueue })
      }
    })
    const queued = listen<TaskQueue>('assistant:queue', ({ payload }) => {
      client.setQueryData(keys.taskQueue, payload)
    })
    return () => {
      void runs.then((unlisten) => {
        unlisten()
      })
      void queued.then((unlisten) => {
        unlisten()
      })
    }
  }, [client])

  const drop = useMutation({
    mutationFn: clearTaskQueue,
    onSuccess: (dropped) => {
      void client.invalidateQueries({ queryKey: keys.taskQueue })
      void client.invalidateQueries({ queryKey: keys.activeTasks })
      say.info(t('assistant.queueDropped', { count: dropped }))
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  const waiting = queue.data?.waiting.length ?? 0
  // Nothing waiting is not a quiet state worth drawing: the runs still going
  // are visible in the panel, and a bar saying "0 left" is furniture.
  if (waiting === 0) return null

  return (
    <div
      role="status"
      className="flex items-center gap-3 rounded-[10px] border border-line bg-raise px-3 py-2 text-sm"
    >
      <Layers size={16} className="shrink-0 text-dim" aria-hidden />
      <span>{t('assistant.queueWaiting', { count: waiting })}</span>
      <Button
        variant="ghost"
        size="sm"
        className="ml-auto"
        disabled={drop.isPending}
        onClick={() => {
          drop.mutate()
        }}
      >
        {t('assistant.queueDrop')}
      </Button>
    </div>
  )
}
