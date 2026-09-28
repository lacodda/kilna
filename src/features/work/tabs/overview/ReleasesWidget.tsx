import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { ScheduledRelease, Work } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { releaseTone, type Tone } from '@/lib/overview'
import { queries } from '@/lib/query/queries'
import { labelOf, useProfile, vocabularyOf } from '@/lib/useProfile'
import { StatusDot } from '@/components/ui/status-dot'
import { Loaded } from '@/components/Loaded'
import {
  Invite,
  InviteLink,
  useLook,
  Widget,
  WidgetSkeleton,
} from '@/features/work/tabs/overview/Widget'

/** How many releases a widget lists before it says how many more there are. */
const ROWS = { s: 3, m: 6, l: 6 } as const

/**
 * Where the work goes out, and where each of those stands: the widget
 * catalogue's list - a dot, the kind of release, the day.
 *
 * The dot is the state (`releaseTone`) and the words beside it say the same,
 * so the colour is never the only thing saying it.
 */
export function ReleasesWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const { presentation, size } = useLook()
  const releases = useQuery(queries.releasesForWork(work.id))

  const to = `/works/${work.id}/releases`
  const empty = releases.data?.length === 0

  return (
    <Widget
      caption={t('card.tab.releases')}
      to={to}
      tone={empty ? 'empty' : 'plain'}
      actions={empty ? <InviteLink to={to}>{t('releases.add')}</InviteLink> : undefined}
    >
      <Loaded query={releases} plain skeleton={<WidgetSkeleton />}>
        {(rows) => {
          if (rows.length === 0) return <Invite>{t('overview.noReleases')}</Invite>

          // Across a row of the board they stand side by side, as the mockup's
          // bands and sheet draw them; in a card, one under another.
          if (presentation !== 'card') {
            return (
              <ul className="flex flex-wrap items-center gap-x-4.5 gap-y-1">
                {rows.map((release) => (
                  <ReleaseLine key={release.id} release={release} kind={work.kind} across />
                ))}
              </ul>
            )
          }

          const shown = rows.slice(0, ROWS[size])
          const more = rows.length - shown.length
          return (
            <>
              <ul className="flex flex-col">
                {shown.map((release) => (
                  <ReleaseLine key={release.id} release={release} kind={work.kind} />
                ))}
              </ul>
              {more > 0 && (
                <span className="text-xs text-faint">{t('overview.more', { count: more })}</span>
              )}
            </>
          )
        }}
      </Loaded>
    </Widget>
  )
}

/** The state words the dot stands for, for a reader who does not see it. */
const SAID: Record<Tone, string> = {
  good: 'calendar.released',
  info: 'calendar.ready',
  warn: 'overview.releaseWaits',
  neutral: 'overview.releaseWaits',
}

function ReleaseLine({
  release,
  kind,
  across = false,
}: {
  release: ScheduledRelease
  kind: string
  across?: boolean
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  const kinds = vocabularyOf(profile.config, kind).release_kinds
  const tone = releaseTone(release)

  const when =
    release.status === 'released'
      ? t('releases.releasedOn', {
          date: release.released_at === null ? '' : formatDay(release.released_at),
        })
      : release.scheduled_at === null
        ? t('releases.unscheduled')
        : formatDay(release.scheduled_at)

  return (
    <li
      className={
        across
          ? 'flex items-center gap-1.75 text-sm'
          : 'flex min-w-0 items-center gap-2 border-b border-line py-1 text-sm last:border-b-0'
      }
    >
      <StatusDot status={tone} size="sm" label={t(SAID[tone])} />
      <span className="min-w-0 truncate">{labelOf(kinds, release.kind)}</span>
      <time className="ml-auto shrink-0 font-mono text-xs text-faint tabular-nums">{when}</time>
    </li>
  )
}
