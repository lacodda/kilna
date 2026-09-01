import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { AlertTriangle, CalendarDays } from 'lucide-react'
import {
  calendar as fetchCalendar,
  catalogue,
  dismissedFindings,
  type ScoredWork,
} from '@/lib/api'
import { coverFor } from '@/lib/cover'
import { isQuiet, summarise, type Decision } from '@/lib/dashboard'
import { findings, visible } from '@/lib/findings'
import { today } from '@/lib/month'
import { keys } from '@/lib/query'
import { missing } from '@/lib/readiness'
import { labelOf, useProfile } from '@/lib/useProfile'
import { ReadyMarks } from '@/components/calendar/ReadyMarks'
import { Badge } from '@/components/ui/badge'
import { EmptyState } from '@/components/ui/EmptyState'
import { FocusBoard } from '@/components/FocusBoard'
import { Panel, SectionLabel } from '@/components/ui/panel'
import { SkeletonList } from '@/components/ui/Skeleton'

interface Props {
  onSelect: (workId: string, tab?: string) => void
}

/**
 * The first screen: what needs a decision, what goes out this week, what to
 * work on next.
 *
 * It asks no questions of its own — the catalogue's rows and the calendar's
 * slots are the same two queries the other screens make, read a third way. A
 * dashboard with a query of its own would be a second place for "is this
 * ready" to be decided, and the two would drift apart.
 *
 * Everything here is a way into something else. Nothing on this screen changes
 * anything: it is a place to look before deciding where to go.
 */
export function DashboardView({ onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  const works = useQuery({ queryKey: keys.catalogue, queryFn: catalogue })
  const slots = useQuery({ queryKey: keys.calendar, queryFn: fetchCalendar })
  // Read here as well as in the board: the quiet state below has to know
  // whether anything is standing, and a finding the person has already
  // answered must not keep the screen from saying it is quiet.
  const dismissals = useQuery({ queryKey: keys.dismissals, queryFn: dismissedFindings })

  if (works.isPending || slots.isPending) return <SkeletonList rows={6} />

  if (works.isError || slots.isError || works.data === undefined || slots.data === undefined) {
    return (
      <p role="alert" className="text-sm text-bad">
        {t('toast.loadFailed')}
      </p>
    )
  }

  // Read once per render against the user's own day: the backend knows only
  // UTC, and after sunset at a negative offset that is already tomorrow.
  const summary = summarise(works.data, slots.data, today())

  // A finding can outlive every section above — a scored, booked work whose
  // draft moved after the score, dated beyond the week, shows nowhere else.
  // Claiming nothing is waiting while one stands would be a lie the screen
  // tells confidently, which is worse than a busy screen.
  const standing = visible(
    findings(works.data, slots.data, profile.config, today()),
    dismissals.data ?? [],
  )

  // Quiet is now shown *above* the board rather than instead of it. Returning
  // early took the board away with the sections, and the board is the one part
  // of this screen that is the person's own: their lines and their way back to
  // what they put away would have vanished on the morning everything was in
  // order — exactly the morning they are worth reading.
  const quiet = isQuiet(summary) && standing.length === 0

  return (
    <div className="flex flex-col gap-6">
      {quiet && <EmptyState title={t('dashboard.quietTitle')} body={t('dashboard.quietBody')} />}

      {summary.decisions.length > 0 && (
        <section className="flex flex-col gap-2">
          <SectionLabel>
            <AlertTriangle aria-hidden className="size-3.5" />
            {t('dashboard.decisions')}
            <span className="font-mono text-[11px] normal-case tracking-normal text-dim">
              {t('dashboard.decisionsCount', { count: summary.decisions.length })}
            </span>
          </SectionLabel>
          <Panel className="divide-y divide-line">
            {summary.decisions.map((decision) => (
              <DecisionRow key={decision.release.id} decision={decision} onSelect={onSelect} />
            ))}
          </Panel>
        </section>
      )}

      {summary.week.length > 0 && (
        <section className="flex flex-col gap-2">
          <SectionLabel>
            <CalendarDays aria-hidden className="size-3.5" />
            {t('dashboard.week')}
            <span className="font-mono text-[11px] normal-case tracking-normal text-dim">
              {t('dashboard.weekCount', { count: summary.week.length })}
            </span>
          </SectionLabel>
          <Panel className="divide-y divide-line">
            {summary.week.map((entry) => (
              <WeekRow key={entry.release.id} entry={entry} onSelect={onSelect} />
            ))}
          </Panel>
        </section>
      )}

      {summary.shortlist.length > 0 && (
        <section className="flex flex-col gap-2">
          <SectionLabel>{t('dashboard.shortlist')}</SectionLabel>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
            {summary.shortlist.map((work) => (
              <CoverCard key={work.work_id} work={work} onSelect={onSelect} />
            ))}
          </div>
        </section>
      )}

      {summary.unscored.length > 0 && (
        <section className="flex flex-col gap-2">
          <SectionLabel>{t('dashboard.unscored')}</SectionLabel>
          <div className="flex flex-wrap gap-1.5">
            {summary.unscored.map((work) => (
              <button
                key={work.work_id}
                type="button"
                onClick={() => onSelect(work.work_id, 'score')}
                className="cursor-pointer rounded-full border border-line px-2.5 py-0.5 text-[11.5px] text-dim transition-colors hover:border-line-2 hover:text-text"
              >
                {work.title}
              </button>
            ))}
          </div>
        </section>
      )}

      {/* Last, and deliberately so: the sections above are about today, these
          are standing complaints. The two kinds the dashboard already draws as
          sections of its own are skipped rather than said twice. */}
      <FocusBoard
        works={works.data}
        calendar={slots.data}
        skip={['unscored', 'ready-unscheduled']}
        onSelect={onSelect}
      />
    </div>
  )
}

/** A release whose date is close and whose work is not ready for it. */
function DecisionRow({
  decision,
  onSelect,
}: {
  decision: Decision
  onSelect: (workId: string, tab?: string) => void
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { release, daysLeft } = decision

  const gaps = missing(release.readiness).map((gap) =>
    gap === 'score' ? t('calendar.missingScore') : labelOf(profile.config.version_roles, gap),
  )

  return (
    <button
      type="button"
      // Straight to the tab that closes the gap: a missing score opens Score, a
      // missing draft opens the versions. Being told what is wrong and left on
      // the same screen is half an answer.
      onClick={() => onSelect(release.work_id, release.readiness.scored ? undefined : 'score')}
      className="flex w-full cursor-pointer items-center gap-3 px-3 py-2.5 text-left transition-colors first:rounded-t-2xl last:rounded-b-2xl hover:bg-soft"
    >
      <span
        aria-hidden
        className="size-8 shrink-0 rounded-lg"
        style={{ background: coverFor(release.work_id) }}
      />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-semibold">{release.work_title}</span>
        <span className="block truncate text-[11.5px] text-faint">{gaps.join(', ')}</span>
      </span>
      <ReadyMarks readiness={release.readiness} released={false} daysLeft={daysLeft} />
      <Badge variant={daysLeft < 0 ? 'bad' : daysLeft <= 2 ? 'warn' : 'soft'}>
        {when(t, daysLeft)}
      </Badge>
    </button>
  )
}

/** Anything holding a slot inside the week, ready or not. */
function WeekRow({
  entry,
  onSelect,
}: {
  entry: Decision
  onSelect: (workId: string, tab?: string) => void
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { release, daysLeft } = entry

  return (
    <button
      type="button"
      onClick={() => onSelect(release.work_id)}
      className="flex w-full cursor-pointer items-center gap-3 px-3 py-2.5 text-left transition-colors first:rounded-t-2xl last:rounded-b-2xl hover:bg-soft"
    >
      <span className="w-20 shrink-0 font-mono text-[11.5px] text-faint">
        {when(t, daysLeft)}
      </span>
      <span
        aria-hidden
        className="size-8 shrink-0 rounded-lg"
        style={{ background: coverFor(release.work_id) }}
      />
      <span className="min-w-0 flex-1 truncate text-sm font-semibold">{release.work_title}</span>
      <Badge variant="soft">{labelOf(profile.config.release_kinds, release.kind)}</Badge>
      <ReadyMarks readiness={release.readiness} released={false} daysLeft={daysLeft} />
    </button>
  )
}

/** Scored work with nowhere to go yet — the next thing worth picking up. */
function CoverCard({
  work,
  onSelect,
}: {
  work: ScoredWork
  onSelect: (workId: string, tab?: string) => void
}) {
  const profile = useProfile()

  return (
    <button
      type="button"
      onClick={() => onSelect(work.work_id)}
      className="cursor-pointer overflow-hidden rounded-2xl border border-line bg-raise text-left transition-transform hover:-translate-y-0.5 hover:border-line-2"
    >
      <span aria-hidden className="block h-20" style={{ background: coverFor(work.work_id) }} />
      <span className="block px-3 py-2">
        <span className="block truncate text-[12.5px] font-semibold">{work.title}</span>
        <span className="block font-mono text-[11px] text-faint">
          {work.tier === null ? '' : `${labelOf(profile.config.tiers, work.tier)} · `}
          {work.total?.toFixed(1) ?? ''}
        </span>
      </span>
    </button>
  )
}

/** "today", "in 3 days", "2 days late" — a distance, never a raw date. */
function when(t: (key: string, options?: Record<string, unknown>) => string, daysLeft: number) {
  if (daysLeft < 0) return t('dashboard.late', { count: -daysLeft })
  if (daysLeft === 0) return t('dashboard.today')
  return t('dashboard.inDays', { count: daysLeft })
}
