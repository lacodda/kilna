import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  calendar as fetchCalendar,
  markReleased,
  releaseQueue,
  scheduleRelease,
  unscheduleRelease,
  type ScheduledRelease,
} from '@/lib/api'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'
import { cn } from '@/lib/utils'

interface Props {
  onSelect: (workId: string) => void
}

// The queue feeds the calendar: strongest first on the left, dated slots on the
// right. Claiming a taken slot is the one place the app pushes back.
export function CalendarView({ onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  const [slots, setSlots] = useState<ScheduledRelease[]>([])
  const [queued, setQueued] = useState<ScheduledRelease[]>([])
  const [picked, setPicked] = useState<string | null>(null)
  const [slot, setSlot] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [revision, setRevision] = useState(0)

  const refresh = () => setRevision((current) => current + 1)

  useEffect(() => {
    Promise.all([fetchCalendar(), releaseQueue()])
      .then(([calendar, queue]) => {
        setSlots(calendar)
        setQueued(queue)
        setError(null)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }, [revision])

  const claim = () => {
    if (picked === null || slot === '') return

    scheduleRelease(picked, slot)
      .then((result) => {
        setError(null)
        setNotice(
          result.displaced === null
            ? null
            : t('calendar.displaced', {
                title: result.displaced.title ?? result.displaced.work_id,
              }),
        )
        setPicked(null)
        refresh()
      })
      .catch((cause: unknown) => {
        setNotice(null)
        setError(String(cause))
      })
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[22rem_1fr]">
      <section className="flex flex-col gap-3">
        <h3 className="text-sm font-semibold">{t('calendar.queue')}</h3>
        <p className="text-xs text-neutral-500">{t('calendar.queueHint')}</p>

        {queued.length === 0 ? (
          <p className="py-6 text-sm text-neutral-500">{t('calendar.queueEmpty')}</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {queued.map((entry) => (
              <li key={entry.id}>
                <button
                  type="button"
                  onClick={() => setPicked(entry.id === picked ? null : entry.id)}
                  className={cn(
                    'flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors',
                    entry.id === picked
                      ? 'bg-kiln-500/10 text-kiln-700 dark:text-kiln-200'
                      : 'hover:bg-neutral-100 dark:hover:bg-neutral-800',
                  )}
                >
                  <span className="flex-1 truncate font-medium">{entry.work_title}</span>
                  <span className="text-xs text-neutral-500">
                    {labelOf(profile.config.release_kinds, entry.kind)}
                  </span>
                  <span className="w-10 text-right tabular-nums">
                    {entry.total?.toFixed(0) ?? '—'}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}

        {picked !== null && (
          <form
            className="flex gap-2"
            onSubmit={(event) => {
              event.preventDefault()
              claim()
            }}
          >
            <Input
              type="date"
              value={slot}
              onChange={(event) => setSlot(event.target.value)}
              aria-label={t('calendar.slotDate')}
            />
            <Button type="submit" variant="primary" disabled={slot === ''}>
              {t('calendar.claim')}
            </Button>
          </form>
        )}

        {error !== null && (
          <p role="alert" className="text-sm text-red-600">
            {error}
          </p>
        )}
        {notice !== null && (
          <p className="rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:bg-amber-950/50 dark:text-amber-200">
            {notice}
          </p>
        )}
      </section>

      <section className="flex flex-col gap-3">
        <h3 className="text-sm font-semibold">{t('calendar.title')}</h3>

        {slots.length === 0 ? (
          <p className="py-6 text-sm text-neutral-500">{t('calendar.empty')}</p>
        ) : (
          <ul className="flex flex-col gap-2">
            {slots.map((entry) => (
              <li
                key={entry.id}
                className="flex flex-wrap items-center gap-3 rounded-md border border-neutral-200 px-3 py-2 dark:border-neutral-700"
              >
                <span className="w-24 font-mono text-sm tabular-nums">{entry.scheduled_at}</span>
                <button
                  type="button"
                  className="font-medium hover:underline"
                  onClick={() => onSelect(entry.work_id)}
                >
                  {entry.work_title}
                </button>
                <span className="rounded bg-neutral-100 px-1.5 py-0.5 text-xs dark:bg-neutral-800">
                  {labelOf(profile.config.release_kinds, entry.kind)}
                </span>
                {entry.total !== null && (
                  <span className="text-xs text-neutral-500 tabular-nums">
                    {entry.total.toFixed(1)}
                  </span>
                )}

                {entry.status === 'released' ? (
                  <span className="ml-auto text-xs text-emerald-600">
                    {t('calendar.released')}
                    {entry.url !== null && (
                      <a
                        href={entry.url}
                        target="_blank"
                        rel="noreferrer"
                        className="ml-2 underline"
                      >
                        {t('calendar.link')}
                      </a>
                    )}
                  </span>
                ) : (
                  <span className="ml-auto flex gap-1">
                    <Button
                      size="sm"
                      onClick={() => {
                        const url = window.prompt(t('calendar.urlPrompt')) ?? undefined
                        markReleased(entry.id, url)
                          .then(refresh)
                          .catch((cause: unknown) => setError(String(cause)))
                      }}
                    >
                      {t('calendar.markReleased')}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      title={t('calendar.unschedule')}
                      onClick={() => {
                        unscheduleRelease(entry.id)
                          .then(refresh)
                          .catch((cause: unknown) => setError(String(cause)))
                      }}
                    >
                      ↩
                    </Button>
                  </span>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  )
}
