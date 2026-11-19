import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Download } from 'lucide-react'
import type { ReceiverFound } from '@/lib/api/types'
import { disarmReceiver, onReceiverEnded, onReceiverFound, takeReceived } from '@/lib/api/receiver'
import { formatTime } from '@/lib/format'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'

/**
 * The armed receiver, from every screen (v0.156, FR-21 21.2): while a work
 * waits for the file a generator is about to save, the bar says for which
 * and what, until when, and puts it down on a press. When the file arrives
 * the bar says so and moves it in on a press - or says it was moved in, when
 * the setting takes what arrives without asking.
 *
 * Above every screen, as the waiting banner is: the generator is in another
 * window, and the person comes back to whichever screen they left.
 */
export function ReceiverBanner() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const armed = useQuery({ ...queries.armedReceiver(), staleTime: 0 })
  const [found, setFound] = useState<ReceiverFound | null>(null)

  useEffect(() => {
    const stops: (() => void)[] = []
    void onReceiverFound((arrived) => {
      void client.invalidateQueries({ queryKey: keys.armedReceiver })
      if (arrived.taken !== null) {
        say.ok(t('receiver.taken', { name: arrived.name, title: arrived.title }))
        for (const key of refresh.work) void client.invalidateQueries({ queryKey: key })
        return
      }
      setFound(arrived)
    }).then((stop) => stops.push(stop))
    void onReceiverEnded(() => {
      void client.invalidateQueries({ queryKey: keys.armedReceiver })
      say.info(t('receiver.expired'))
    }).then((stop) => stops.push(stop))
    return () => {
      for (const stop of stops) stop()
    }
  }, [client, t])

  const cancel = useAppMutation({
    mutationFn: disarmReceiver,
    refresh: [keys.armedReceiver],
  })
  const take = useAppMutation({
    mutationFn: (arrived: ReceiverFound) =>
      takeReceived(arrived.work_id, arrived.role, arrived.path),
    failure: 'receiver.takeFailed',
    refresh: refresh.work,
    onSuccess: (_, arrived) => {
      say.ok(t('receiver.taken', { name: arrived.name, title: arrived.title }))
      setFound(null)
    },
  })

  if (found !== null) {
    return (
      <div
        role="status"
        className="flex items-center gap-3 rounded-md border border-accent bg-raise px-3 py-2 text-sm"
      >
        <Download size={16} className="shrink-0 text-accent" aria-hidden />
        <span className="min-w-0 truncate">
          {t('receiver.arrived', {
            name: found.name,
            title: found.title,
            what: t(`receiver.role.${found.role}`),
          })}
        </span>
        <span className="ml-auto flex shrink-0 gap-1.5">
          <Button variant="ghost" size="sm" onClick={() => setFound(null)}>
            {t('receiver.notThis')}
          </Button>
          <Button
            variant="primary"
            size="sm"
            disabled={take.isPending}
            onClick={() => take.mutate(found)}
          >
            {t('receiver.take')}
          </Button>
        </span>
      </div>
    )
  }

  const waiting = armed.data
  if (waiting === undefined || waiting === null) return null
  return (
    <div
      role="status"
      className="flex items-center gap-3 rounded-md border border-line bg-raise px-3 py-2 text-sm"
    >
      <Download size={16} className="shrink-0 text-dim" aria-hidden />
      <span className="min-w-0 truncate">
        {t('receiver.waiting', {
          what: t(`receiver.role.${waiting.role}`),
          title: waiting.title,
          until: formatTime(waiting.until),
        })}
      </span>
      <Button
        variant="ghost"
        size="sm"
        className="ml-auto shrink-0"
        disabled={cancel.isPending}
        onClick={() => cancel.mutate()}
      >
        {t('receiver.cancel')}
      </Button>
    </div>
  )
}
