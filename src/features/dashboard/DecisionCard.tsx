import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import type { Decision } from '@/lib/dashboard'
import { coverImageFor } from '@/lib/cover'
import { formatNumber } from '@/lib/format'
import { stageAt } from '@/lib/stages'
import { useCovers } from '@/lib/useCovers'
import { allOf, labelOf, useProfile } from '@/lib/useProfile'
import { StageDial } from '@/components/StageDial'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Panel } from '@/components/ui/panel'
import { when } from '@/features/dashboard/when'

interface Props {
  decision: Decision
  onSelect: (workId: string, tab?: string) => void
  /** Put a complaint away. Only a decision drawn from a finding has one to put. */
  onDismiss?: () => void
  dismissing?: boolean
}

/**
 * Something only the person can settle: the work, why it is here in one
 * sentence, and the one button that goes and does it.
 *
 * A card, not a row. The row it replaced said what was wrong and left the
 * person to guess what to press - the whole row opened the work, on a tab
 * that was not always the one the gap was on. Here the button is named for
 * the move (open, score, re-score, schedule) and lands on the tab it is made
 * on.
 */
export function DecisionCard({ decision, onSelect, onDismiss, dismissing = false }: Props) {
  const { t } = useTranslation()
  const covers = useCovers()
  const profile = useProfile()

  const reason = (() => {
    if (decision.kind === 'release') {
      const gaps = decision.gaps.map((gap) =>
        gap === 'score'
          ? t('calendar.missingScore')
          : labelOf(allOf(profile.config, 'version_roles'), gap),
      )
      return t('dashboard.reason.missing', { gaps: gaps.join(', ') })
    }
    // Both findings a card is drawn from are about scored work, so the score
    // is there to quote; the short complaint stands in should it ever not be.
    if (decision.total === null) return t(`findings.kindShort.${decision.finding.kind}`)
    const total = formatNumber(decision.total)
    return decision.move === 'rescore'
      ? t('dashboard.reason.rescore', { total })
      : t('dashboard.reason.schedule', { total })
  })()

  return (
    <Panel className="group relative flex min-w-0 flex-col overflow-hidden">
      <span
        aria-hidden
        className="block h-10 shrink-0"
        style={{ background: coverImageFor(decision.workId, covers.get(decision.workId)) }}
      />

      <div className="flex min-w-0 flex-col gap-1.5 px-3 pt-2 pb-3">
        <h3 className="truncate text-sm font-semibold">{decision.title}</h3>
        <p className="text-sm leading-relaxed text-dim">{reason}</p>
        <div className="flex flex-wrap items-center gap-1.5">
          {decision.kind === 'release' && (
            <>
              <Badge
                variant={
                  decision.slot.daysLeft < 0 ? 'bad' : decision.slot.daysLeft <= 2 ? 'warn' : 'soft'
                }
              >
                {when(t, decision.slot.daysLeft)}
              </Badge>
              {/* How finished the work behind it is, beside how close the
                  date is: the reason says what the release lacks, the dial
                  says how far the work has come. */}
              {decision.slot.release.work_stage !== null && (
                <StageDial
                  percent={decision.slot.release.work_stage}
                  stage={stageAt(profile.config, decision.slot.release.work_stage)}
                />
              )}
            </>
          )}
          {/* The accent is for what has a date or a score going stale under
              it; a work waiting for a slot can wait a day, and its button
              says so by being quieter - the mockup's plain `.ghost`. */}
          <Button
            size="xs"
            variant={decision.move === 'schedule' ? 'ghost' : 'soft'}
            onClick={() => onSelect(decision.workId, decision.tab)}
          >
            {t(`dashboard.move.${decision.move}`)}
          </Button>
        </div>
      </div>

      {/* Over the cover, where the card's own words are not: shown on hover
          and on focus, so the keyboard reaches it too. Only a finding has
          one - a release leaves this list when it is ready or moved, and
          there is nothing to put away. */}
      {onDismiss !== undefined && (
        <div className="absolute top-1.5 right-1.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
          <Button
            variant="icon"
            size="icon-xs"
            className="bg-raise/80"
            aria-label={t('findings.dismiss')}
            title={t('findings.dismissHint')}
            disabled={dismissing}
            onClick={onDismiss}
          >
            <X aria-hidden />
          </Button>
        </div>
      )}
    </Panel>
  )
}
