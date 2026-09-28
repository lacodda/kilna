import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { AlertTriangle, CalendarDays } from 'lucide-react'
import { dismissFinding, restoreFinding } from '@/lib/api/focus'
import type { Dismissal, ScheduledRelease, ScoredWork } from '@/lib/api/types'
import { aside, decide, isQuiet, summarise } from '@/lib/dashboard'
import { dismissalKey, findings, visible, type Finding } from '@/lib/findings'
import { today } from '@/lib/month'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Panel, SectionLabel } from '@/components/ui/panel'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Pane, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { DecisionCard } from '@/features/dashboard/DecisionCard'
import { FindingsWidget } from '@/features/dashboard/FindingsWidget'
import { NotesWidget } from '@/features/dashboard/NotesWidget'
import { RunningWidget } from '@/features/dashboard/RunningWidget'
import { SinceWidget } from '@/features/dashboard/SinceWidget'
import { SlotCard } from '@/features/dashboard/SlotCard'
import { WeekRow } from '@/features/dashboard/WeekRow'
import { WorksWidget } from '@/features/dashboard/WorksWidget'

interface Props {
  onSelect: (workId: string, tab?: string) => void
}

/**
 * The first screen: what needs a decision, what goes out this week, and what
 * the workspace is doing - in two columns that each scroll on their own.
 *
 * The lead column is the day's business: decisions as cards with the one move
 * that settles each, the week's slots, the work nothing has judged, and the
 * nearest slots. The side column is the state of things: the catalogue in
 * figures, the assistant's running tasks, what the workspace noticed, the
 * person's own lines, and what happened since they last looked.
 *
 * It asks no questions of its own about the work — the catalogue's rows and
 * the calendar's slots are the same two queries the other screens make, read
 * a third way. A dashboard with a query of its own would be a second place for
 * "is this ready" to be decided, and the two would drift apart.
 *
 * Everything here is a way into something else. Nothing on this screen changes
 * the work: it is a place to look before deciding where to go.
 */
export function DashboardView({ onSelect }: Props) {
  const works = useQuery(queries.catalogue())
  const slots = useQuery(queries.calendar())
  // Awaited with the other two: the decisions are drawn from findings, and a
  // card the person has already put away must not flash up while their
  // answer is on its way.
  const dismissals = useQuery(queries.dismissals())

  const skeleton = <SkeletonList rows={6} />

  // The screen holds the window's height; each column scrolls under it.
  // Three reads, each with its own way back when it fails.
  return (
    <Frame>
      <Loaded query={works} fill skeleton={skeleton}>
        {(catalogue) => (
          <Loaded query={slots} fill skeleton={skeleton}>
            {(calendar) => (
              <Loaded query={dismissals} fill skeleton={skeleton}>
                {(dismissed) => (
                  <Summary
                    works={catalogue}
                    slots={calendar}
                    dismissed={dismissed}
                    onSelect={onSelect}
                  />
                )}
              </Loaded>
            )}
          </Loaded>
        )}
      </Loaded>
    </Frame>
  )
}

function Summary({
  works,
  slots,
  dismissed,
  onSelect,
}: {
  works: ScoredWork[]
  slots: ScheduledRelease[]
  dismissed: Dismissal[]
  onSelect: Props['onSelect']
}) {
  const { t } = useTranslation()
  const profile = useProfile()

  const hide = useAppMutation({
    mutationFn: (finding: Finding) => dismissFinding(dismissalKey(finding)),
    refresh: refresh.focus,
  })

  const unhide = useAppMutation({
    mutationFn: (row: Dismissal) =>
      restoreFinding({ kind: row.kind, work_id: row.work_id, complaint: row.complaint }),
    refresh: refresh.focus,
  })

  // Read once per render against the user's own day: the backend knows only
  // UTC, and after sunset at a negative offset that is already tomorrow.
  const day = today()
  const summary = summarise(works, slots, day)
  const standing = visible(findings(works, slots, profile.config, day), dismissed)
  const decisions = decide(summary, standing, works)
  const beside = aside(standing, decisions)

  // A finding can outlive every section of the lead column — a scored, booked
  // work whose draft moved after the score, dated beyond the week, shows only
  // beside it. Claiming nothing is waiting while one stands would be a lie the
  // screen tells confidently, which is worse than a busy screen.
  const leadIsEmpty = isQuiet(summary) && decisions.length === 0

  return (
    <div className="grid min-h-0 min-w-0 flex-1 grid-cols-[minmax(0,1.5fr)_minmax(0,1fr)] grid-rows-1 gap-2.5">
      <Pane label={t('dashboard.lead')} bodyClassName="flex flex-col gap-4 p-3">
        {leadIsEmpty &&
          (standing.length === 0 ? (
            <EmptyState title={t('dashboard.quietTitle')} body={t('dashboard.quietBody')} />
          ) : (
            <EmptyState
              plain
              title={t('dashboard.noDecisionsTitle')}
              body={t('dashboard.noDecisionsBody')}
            />
          ))}

        {decisions.length > 0 && (
          <section className="flex flex-col gap-2">
            <SectionLabel>
              <AlertTriangle aria-hidden className="size-3.5" />
              {t('dashboard.decisions')}
              <span className="font-mono text-xs tracking-normal normal-case text-dim">
                {t('dashboard.decisionsCount', { count: decisions.length })}
              </span>
            </SectionLabel>
            <div className="grid grid-cols-[repeat(auto-fill,minmax(210px,1fr))] gap-2.5">
              {decisions.map((decision) => (
                <DecisionCard
                  key={decision.key}
                  decision={decision}
                  onSelect={onSelect}
                  dismissing={hide.isPending}
                  onDismiss={
                    decision.kind === 'finding' ? () => hide.mutate(decision.finding) : undefined
                  }
                />
              ))}
            </div>
          </section>
        )}

        {summary.week.length > 0 && (
          <section className="flex flex-col gap-2">
            <SectionLabel>
              <CalendarDays aria-hidden className="size-3.5" />
              {t('dashboard.week')}
              <span className="font-mono text-xs tracking-normal normal-case text-dim">
                {t('dashboard.weekCount', { count: summary.week.length })}
              </span>
            </SectionLabel>
            {/* `overflow-hidden`: the rows' hover tint stops at the panel's
                rounded corners rather than bulging past them. */}
            <Panel className="divide-y divide-line overflow-hidden">
              {summary.week.map((entry) => (
                <WeekRow key={entry.release.id} entry={entry} onSelect={onSelect} />
              ))}
            </Panel>
          </section>
        )}

        {summary.unscored.length > 0 && (
          <section className="flex flex-col gap-2">
            <SectionLabel>{t('dashboard.unscored')}</SectionLabel>
            <div className="flex flex-wrap gap-1.5">
              {summary.unscored.map((work) => (
                // Dashed, the mockup's `.chip.dash`: a work waiting for the
                // judgement that would give it a place in the lists above.
                <Button
                  key={work.work_id}
                  variant="ghost"
                  size="xs"
                  className="rounded-full border-dashed"
                  onClick={() => onSelect(work.work_id, 'score')}
                >
                  {work.title}
                </Button>
              ))}
            </div>
          </section>
        )}

        {summary.nearest.length > 0 && (
          <section className="flex flex-col gap-2">
            <SectionLabel>{t('dashboard.shortlist')}</SectionLabel>
            <div className="grid grid-cols-[repeat(auto-fill,minmax(210px,1fr))] gap-2.5">
              {summary.nearest.map((slot) => (
                <SlotCard key={slot.release.id} slot={slot} onSelect={onSelect} />
              ))}
            </div>
          </section>
        )}
      </Pane>

      <Scroll label={t('dashboard.side')} contentClassName="flex flex-col gap-2.5">
        <WorksWidget works={works} />
        <RunningWidget works={works} onSelect={onSelect} />
        <FindingsWidget
          findings={beside}
          putAway={dismissed}
          onSelect={onSelect}
          onDismiss={(finding) => hide.mutate(finding)}
          onRestore={(row) => unhide.mutate(row)}
          dismissing={hide.isPending}
        />
        <NotesWidget onSelect={onSelect} />
        <SinceWidget />
      </Scroll>
    </div>
  )
}
