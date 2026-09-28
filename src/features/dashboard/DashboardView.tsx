import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { AlertTriangle, CalendarDays } from 'lucide-react'
import type { ScheduledRelease, ScoredWork } from '@/lib/api/types'
import { coverImageFor } from '@/lib/cover'
import { useCovers } from '@/lib/useCovers'
import { isQuiet, summarise, type Decision } from '@/lib/dashboard'
import { findings, visible } from '@/lib/findings'
import { today } from '@/lib/month'
import { queries } from '@/lib/query/queries'
import { missing } from '@/lib/readiness'
import { allOf, labelOf, useProfile } from '@/lib/useProfile'
import { StageDial } from '@/components/StageDial'
import { stageAt } from '@/lib/stages'
import { formatNumber } from '@/lib/format'
import { ReadyMarks } from '@/components/ReadyMarks'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { RowButton } from '@/components/ui/list-row'
import { EmptyState } from '@/components/ui/empty-state'
import { Panel, SectionLabel } from '@/components/ui/panel'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { FocusBoard } from '@/features/dashboard/FocusBoard'

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
  const works = useQuery(queries.catalogue())
  const slots = useQuery(queries.calendar())

  // The screen holds the window's height; what is on it scrolls under it.
  // Two reads, each with its own way back when it fails.
  return (
    <Frame>
      <Loaded query={works} fill skeleton={<SkeletonList rows={6} />}>
        {(catalogue) => (
          <Loaded query={slots} fill skeleton={<SkeletonList rows={6} />}>
            {(calendar) => <Summary works={catalogue} slots={calendar} onSelect={onSelect} />}
          </Loaded>
        )}
      </Loaded>
    </Frame>
  )
}

function Summary({
  works,
  slots,
  onSelect,
}: {
  works: ScoredWork[]
  slots: ScheduledRelease[]
  onSelect: Props['onSelect']
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  // Read here as well as in the board: the quiet state below has to know
  // whether anything is standing, and a finding the person has already
  // answered must not keep the screen from saying it is quiet.
  const dismissals = useQuery(queries.dismissals())

  // Read once per render against the user's own day: the backend knows only
  // UTC, and after sunset at a negative offset that is already tomorrow.
  const summary = summarise(works, slots, today())

  // A finding can outlive every section above — a scored, booked work whose
  // draft moved after the score, dated beyond the week, shows nowhere else.
  // Claiming nothing is waiting while one stands would be a lie the screen
  // tells confidently, which is worse than a busy screen.
  const standing = visible(findings(works, slots, profile.config, today()), dismissals.data ?? [])

  // Quiet is now shown *above* the board rather than instead of it. Returning
  // early took the board away with the sections, and the board is the one part
  // of this screen that is the person's own: their lines and their way back to
  // what they put away would have vanished on the morning everything was in
  // order — exactly the morning they are worth reading.
  const quiet = isQuiet(summary) && standing.length === 0

  return (
    <Scroll label={t('nav.dashboard')} contentClassName="flex flex-col gap-6">
      {quiet && <EmptyState title={t('dashboard.quietTitle')} body={t('dashboard.quietBody')} />}

      {summary.decisions.length > 0 && (
        <section className="flex flex-col gap-2">
          <SectionLabel>
            <AlertTriangle aria-hidden className="size-3.5" />
            {t('dashboard.decisions')}
            <span className="font-mono text-xs normal-case tracking-normal text-dim">
              {t('dashboard.decisionsCount', { count: summary.decisions.length })}
            </span>
          </SectionLabel>
          {/* `overflow-hidden`: a row rounds its hover to its own corner rather
              than the panel's, and at the panel's corners that tint would
              bulge past the curve. The panel clips it, for both lists here. */}
          <Panel className="divide-y divide-line overflow-hidden">
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
            <span className="font-mono text-xs normal-case tracking-normal text-dim">
              {t('dashboard.weekCount', { count: summary.week.length })}
            </span>
          </SectionLabel>
          <Panel className="divide-y divide-line overflow-hidden">
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
              <Button
                key={work.work_id}
                variant="ghost"
                size="xs"
                onClick={() => onSelect(work.work_id, 'score')}
              >
                {work.title}
              </Button>
            ))}
          </div>
        </section>
      )}

      {/* Last, and deliberately so: the sections above are about today, these
          are standing complaints. The two kinds the dashboard already draws as
          sections of its own are skipped rather than said twice. */}
      <FocusBoard
        works={works}
        calendar={slots}
        skip={['unscored', 'ready-unscheduled']}
        onSelect={onSelect}
      />
    </Scroll>
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
  const covers = useCovers()
  const { t } = useTranslation()
  const profile = useProfile()
  const { release, daysLeft } = decision

  const gaps = missing(release.readiness).map((gap) =>
    gap === 'score'
      ? t('calendar.missingScore')
      : labelOf(allOf(profile.config, 'version_roles'), gap),
  )

  return (
    <RowButton
      // Straight to the tab that closes the gap: a missing score opens Score, a
      // missing draft opens the versions. Being told what is wrong and left on
      // the same screen is half an answer.
      onClick={() => onSelect(release.work_id, release.readiness.scored ? undefined : 'score')}
      start={
        <span
          aria-hidden
          className="size-8 shrink-0 rounded-lg"
          style={{ background: coverImageFor(release.work_id, covers.get(release.work_id)) }}
        />
      }
      description={gaps.join(', ')}
      end={
        <>
          {/* Beside the readiness marks, which answer a different question:
              those say whether the release could go out, this says how
              finished the work behind it is. */}
          {release.work_stage !== null && (
            <StageDial
              percent={release.work_stage}
              stage={stageAt(profile.config, release.work_stage)}
            />
          )}
          <ReadyMarks readiness={release.readiness} released={false} daysLeft={daysLeft} />
          <Badge variant={daysLeft < 0 ? 'bad' : daysLeft <= 2 ? 'warn' : 'soft'}>
            {when(t, daysLeft)}
          </Badge>
        </>
      }
    >
      {release.work_title}
    </RowButton>
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
  const covers = useCovers()
  const { t } = useTranslation()
  const profile = useProfile()
  const { release, daysLeft } = entry

  return (
    <RowButton
      onClick={() => onSelect(release.work_id)}
      start={
        <span className="flex items-center gap-2.5">
          <span className="w-20 shrink-0 font-mono text-xs text-faint">{when(t, daysLeft)}</span>
          <span
            aria-hidden
            className="size-8 shrink-0 rounded-lg"
            style={{ background: coverImageFor(release.work_id, covers.get(release.work_id)) }}
          />
        </span>
      }
      end={
        <>
          {/* What it is and what goes out — a video's YouTube release and a
              song's clip are told apart by the first word, once the profile
              has more than one kind of work. */}
          <Badge variant="soft">
            {profile.config.work_kinds.length > 1
              ? `${labelOf(profile.config.work_kinds, release.work_kind)} · ${labelOf(allOf(profile.config, 'release_kinds'), release.kind)}`
              : labelOf(allOf(profile.config, 'release_kinds'), release.kind)}
          </Badge>
          {/* How finished the work is, beside how ready the release is: the
              marks say whether it could go out, the dial says whether it is
              done. */}
          {release.work_stage !== null && (
            <StageDial
              percent={release.work_stage}
              stage={stageAt(profile.config, release.work_stage)}
            />
          )}
          <ReadyMarks readiness={release.readiness} released={false} daysLeft={daysLeft} />
        </>
      }
    >
      {release.work_title}
    </RowButton>
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
  const covers = useCovers()
  const profile = useProfile()

  return (
    // A card, not a row: a cover over two lines, which no row's slots hold.
    // So the quiet button with no size of its own, laid out as a column; its
    // border and hover are the card's.
    <Button
      variant="ghost"
      size={null}
      onClick={() => onSelect(work.work_id)}
      className="flex-col items-stretch gap-0 overflow-hidden text-left"
    >
      <span
        aria-hidden
        className="block h-20"
        style={{ background: coverImageFor(work.work_id, covers.get(work.work_id)) }}
      />
      <span className="block px-3 py-2">
        <span className="block truncate text-sm font-semibold">{work.title}</span>
        <span className="block font-mono text-xs text-faint">
          {work.tier === null ? '' : `${labelOf(allOf(profile.config, 'tiers'), work.tier)} · `}
          {work.total === null ? '' : formatNumber(work.total)}
        </span>
      </span>
    </Button>
  )
}

/** "today", "in 3 days", "2 days late" — a distance, never a raw date. */
function when(t: (key: string, options?: Record<string, unknown>) => string, daysLeft: number) {
  if (daysLeft < 0) return t('dashboard.late', { count: -daysLeft })
  if (daysLeft === 0) return t('dashboard.today')
  return t('dashboard.inDays', { count: daysLeft })
}
