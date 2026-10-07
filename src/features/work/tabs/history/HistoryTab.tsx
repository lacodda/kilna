import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import type { Moment } from '@/lib/api/types'
import { formatDay, formatTime } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { axisOf, hasTime, kindOf, MOMENT_KINDS, nearDays, type MomentKind } from '@/lib/timeline'
import { cn } from '@/lib/utils'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { EmptyState } from '@/components/ui/empty-state'
import { SectionLabel } from '@/components/ui/panel'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { useSayMoment } from '@/features/work/tabs/history/moments'

/** The dot a moment sits on, one tint per kind of thing. */
const DOT: Record<MomentKind | 'begun', string> = {
  versions: 'bg-accent',
  scores: 'bg-good',
  releases: 'bg-warn',
  journal: 'bg-faint',
  begun: 'bg-text',
}

/** What a moment is called among the others, for React to tell them apart. */
function keyOf(moment: Moment): string {
  switch (moment.type) {
    case 'begun':
      return 'begun'
    case 'made':
      return `made-${moment.work_id}`
    case 'entry':
      return `entry-${moment.entry.id}`
    default:
      return `${moment.type}-${moment.id}`
  }
}

/** The time of a moment, as a record says it: the hour for a timestamp, the
 *  day for a release still ahead, nothing for a day already headed. */
function timeOf(at: string, ahead: boolean): string | null {
  if (ahead) return formatDay(at)
  return hasTime(at) ? formatTime(at) : null
}

/**
 * A work's history on one axis (#78, ADR 0056): when it was begun, every
 * version and what it was written from, every score and what it read, what
 * was made from it and the day each went out, and the journal's lines no
 * row holds - latest first, on the days they fell on where the person is.
 *
 * Read off the rows, so a work brought in from elsewhere has its whole
 * history here, and a line of the journal swept a week after it was read
 * takes nothing with it that a row remembers. What is booked for a day
 * after today stands above today, under its own heading.
 */
export function HistoryTab({ workId }: { workId: string }) {
  const { t } = useTranslation()
  const moments = useQuery(queries.timeline(workId))
  const [shown, setShown] = useState<MomentKind[]>([...MOMENT_KINDS])
  const say = useSayMoment(workId)

  const counts = useMemo(() => {
    const out: Partial<Record<MomentKind, number>> = {}
    for (const moment of moments.data ?? []) {
      const kind = kindOf(moment)
      if (kind !== null) out[kind] = (out[kind] ?? 0) + 1
    }
    return out
  }, [moments.data])

  const { today, yesterday } = nearDays()
  const heading = (day: string) =>
    day === today
      ? t('timeline.today')
      : day === yesterday
        ? t('timeline.yesterday')
        : formatDay(day)

  const line = (moment: Moment, ahead: boolean) => {
    const said = say(moment)
    const time = timeOf(moment.at, ahead)
    const kind = kindOf(moment) ?? 'begun'
    return (
      <li
        key={keyOf(moment)}
        className="relative flex min-w-0 items-center gap-2 py-1.5 pr-3 pl-7 text-sm"
      >
        <span
          aria-hidden
          className={cn('absolute top-1/2 left-2 size-2 -translate-y-1/2 rounded-full', DOT[kind])}
        />
        {said.chip}
        {said.to === null ? (
          <span className="min-w-0 flex-1 truncate" title={said.text}>
            {said.text}
          </span>
        ) : (
          <Link
            to={said.to}
            title={said.text}
            className="min-w-0 flex-1 truncate text-text no-underline hover:underline"
          >
            {said.text}
          </Link>
        )}
        {moment.type === 'entry' && moment.entry.occurrences > 1 && (
          <span className="shrink-0 text-xs text-faint">
            {t('journal.repeated', { count: moment.entry.occurrences })}
          </span>
        )}
        {time !== null && (
          <time
            dateTime={moment.at}
            title={moment.at}
            className="shrink-0 font-mono text-xs text-faint tabular-nums"
          >
            {time}
          </time>
        )}
      </li>
    )
  }

  return (
    <Frame
      head={
        <>
          <SectionLabel>{t('journal.title')}</SectionLabel>
          <span className="text-xs text-faint">{t('timeline.hint')}</span>
          <ChipGroup
            multiple
            aria-label={t('timeline.filter')}
            value={shown}
            onValueChange={(next) => setShown(next as MomentKind[])}
            className="ml-auto"
          >
            {MOMENT_KINDS.map((kind) => (
              <Chip key={kind} value={kind} count={counts[kind] ?? 0}>
                {t(`timeline.filters.${kind}`)}
              </Chip>
            ))}
          </ChipGroup>
        </>
      }
    >
      <Loaded
        query={moments}
        fill
        skeleton={<SkeletonList rows={4} />}
        isEmpty={(data) => data.length === 0}
        emptyState={
          <EmptyState
            title={t('empty.historyTitle')}
            body={t('empty.historyBody')}
            className="flex-1"
          />
        }
      >
        {(data) => {
          const axis = axisOf(data, new Set(shown))
          return (
            <Pane label={t('journal.title')}>
              {/* One line down the whole axis, the dots on it. */}
              <div className="relative before:absolute before:inset-y-0 before:left-3 before:w-px before:bg-line">
                {axis.ahead.length > 0 && (
                  <section aria-label={t('timeline.ahead')}>
                    <h3 className="caption relative bg-raise py-1.5 pl-7">{t('timeline.ahead')}</h3>
                    <ul className="flex flex-col">
                      {axis.ahead.map((moment) => line(moment, true))}
                    </ul>
                  </section>
                )}
                {axis.days.map((day) => (
                  <section key={day.day} aria-label={heading(day.day)}>
                    <h3 className="caption relative bg-raise py-1.5 pl-7">{heading(day.day)}</h3>
                    <ul className="flex flex-col">
                      {day.moments.map((moment) => line(moment, false))}
                    </ul>
                  </section>
                ))}
              </div>
            </Pane>
          )
        }}
      </Loaded>
    </Frame>
  )
}
