import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { Check, Copy } from 'lucide-react'
import type { StudioItem } from '@/lib/api/types'
import { openWorkFolder } from '@/lib/api/folders'
import { isVideoLink, studioLink, studioPack } from '@/lib/api/studio'
import { formatWeekday } from '@/lib/format'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel } from '@/lib/useProfile'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

interface Props {
  /** The works whose releases are put out. */
  workIds: readonly string[]
  onClose: () => void
}

/** The door the pack puts out on. */
const DOOR = 'youtube'

/**
 * A batch put out on YouTube by hand (v0.159, FR-22 22.1 p.1): no API - its
 * quota is gone - and no robot driving the browser - that is how a channel
 * gets banned. One release a step: its video and cover shown in the folder
 * to drag into YouTube Studio, every field a press from the clipboard and
 * ticked once copied, the day and time in the channel's zone - said when
 * summer time makes its offset another than today's - and the link the
 * platform gave back, which marks the release out on its day or keeps it
 * booked before, and goes on to the next.
 */
export function StudioPack({ workIds, onClose }: Props) {
  const { t } = useTranslation()
  const pack = useQuery({
    queryKey: ['studio', 'pack', ...workIds],
    queryFn: () => studioPack(workIds, DOOR),
  })
  const [step, setStep] = useState(0)
  const [kept, setKept] = useState(0)

  const items = pack.data?.items ?? []
  const skipped = pack.data?.skipped ?? []
  const item = items[step]

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
      title={
        item === undefined
          ? t('studio.title')
          : t('studio.titleStep', { step: step + 1, of: items.length })
      }
      size="lg"
      dismiss={item === undefined && pack.data !== undefined ? 'close' : 'cancel'}
    >
      {pack.data === undefined ? null : item === undefined ? (
        <div className="flex flex-col gap-2 text-sm">
          <p>{items.length === 0 ? t('studio.nothing') : t('studio.done', { count: kept })}</p>
          {skipped.length > 0 && (
            <ul className="flex flex-col gap-0.5 text-xs text-dim">
              {skipped.map((one) => (
                <li key={one.work_id}>
                  <b className="font-semibold text-text">{one.title}</b>
                  {' · '}
                  {t(`studio.skip.${one.reason}`)}
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : (
        <Step
          key={item.release.id}
          item={item}
          onNext={(linked) => {
            if (linked) setKept((before) => before + 1)
            setStep((before) => before + 1)
          }}
        />
      )}
    </Dialog>
  )
}

/** One release of the pack. */
function Step({ item, onNext }: { item: StudioItem; onNext: (linked: boolean) => void }) {
  const { t, i18n } = useTranslation()
  const [copied, setCopied] = useState<string[]>([])
  const [link, setLink] = useState(item.release.url ?? '')

  const copy = (key: string, text: string) => {
    navigator.clipboard.writeText(text).then(
      () => setCopied((before) => (before.includes(key) ? before : [...before, key])),
      (cause: unknown) => say.failed(cause),
    )
  }
  const reveal = (path: string) => {
    revealItemInDir(path).catch((cause: unknown) => say.failed(cause))
  }

  const keep = useAppMutation({
    mutationFn: () => studioLink(item.release.id, link),
    failure: 'studio.linkFailed',
    refresh: [...refresh.works, keys.calendar],
    onSuccess: (done) => {
      say.ok(done.released ? t('studio.released') : t('studio.booked'))
      onNext(true)
    },
  })

  const summer = item.offset !== null && item.offset_now !== null && item.offset !== item.offset_now
  const rows: { key: string; label: string; value: string }[] = [
    // The day as YouTube Studio's own field takes it typed: in the
    // language the window is in, not ISO.
    ...(item.date === null
      ? []
      : [{ key: 'date', label: t('studio.date'), value: studioDate(item.date, i18n.language) }]),
    ...(item.time === null ? [] : [{ key: 'time', label: t('studio.time'), value: item.time }]),
    ...item.fields.map((field) => ({
      key: field.key,
      label: sayLabel(field.label),
      value: field.value,
    })),
  ]

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-0.5">
        <b className="text-sm font-semibold">{item.work_title}</b>
        <span className="text-xs text-dim">
          {[
            item.date === null ? t('studio.noDate') : formatWeekday(item.date),
            item.time,
            item.time_zone === null
              ? null
              : `${item.time_zone}${item.offset === null ? '' : ` (${item.offset})`}`,
          ]
            .filter((part) => part !== null)
            .join(' · ')}
        </span>
        {summer && (
          <span className="text-xs text-warn">
            {t('studio.summer', { then: item.offset, now: item.offset_now })}
          </span>
        )}
      </div>

      <div className="flex flex-wrap gap-1.5">
        {item.video !== null && (
          <Button size="sm" onClick={() => reveal(item.video!)}>
            {t('studio.showVideo')}
          </Button>
        )}
        {item.cover !== null && (
          <Button size="sm" variant="ghost" onClick={() => reveal(item.cover!)}>
            {t('studio.showCover')}
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          onClick={() => void openWorkFolder(item.release.work_id).catch(say.failed)}
        >
          {t('studio.openFolder')}
        </Button>
      </div>

      <ul className="flex flex-col gap-1">
        {rows.map((row) => {
          const done = copied.includes(row.key)
          return (
            <li key={row.key} className="flex items-center gap-2 text-sm">
              <span className="w-32 shrink-0 text-dim">{row.label}</span>
              <span className="min-w-0 flex-1 truncate" title={row.value}>
                {row.value === '' ? (
                  <span className="text-faint">{t('studio.empty')}</span>
                ) : (
                  row.value
                )}
              </span>
              <Button
                size="sm"
                variant={done ? 'soft' : 'ghost'}
                disabled={row.value === ''}
                aria-label={t('studio.copyField', { label: row.label })}
                onClick={() => copy(row.key, row.value)}
              >
                {done ? <Check aria-hidden /> : <Copy aria-hidden />}
                {done ? t('studio.copied') : t('studio.copy')}
              </Button>
            </li>
          )
        })}
      </ul>

      <div className="flex flex-col gap-1.5 border-t border-line pt-3">
        <span className="text-sm text-dim">{t('studio.linkLabel')}</span>
        <div className="flex gap-1.5">
          <Input
            aria-label={t('studio.linkLabel')}
            placeholder="https://youtu.be/…"
            value={link}
            onChange={(event) => setLink(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' && isVideoLink(link)) keep.mutate()
            }}
          />
          <Button
            variant="primary"
            disabled={!isVideoLink(link) || keep.isPending}
            onClick={() => keep.mutate()}
          >
            {t('studio.keepAndNext')}
          </Button>
        </div>
        {link.trim() !== '' && !isVideoLink(link) && (
          <span className="text-xs text-bad">{t('studio.notALink')}</span>
        )}
        <div>
          <Button size="sm" variant="ghost" onClick={() => onNext(false)}>
            {t('studio.skipStep')}
          </Button>
        </div>
      </div>
    </div>
  )
}

/** A day as a date field types it: `1 дек. 2026 г.`, `Dec 1, 2026`. */
function studioDate(date: string, language: string): string {
  const at = new Date(`${date}T00:00:00Z`)
  if (Number.isNaN(at.getTime())) return date
  return new Intl.DateTimeFormat(language, {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    timeZone: 'UTC',
  }).format(at)
}
