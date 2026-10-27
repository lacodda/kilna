import { useTranslation } from 'react-i18next'
import type { ImportProblem, ImportReason, Placement } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { Button } from '@/components/ui/button'

interface Props {
  placements: Placement[]
  /** The rows of an imported plan that were left out, and why (v0.104). */
  problems?: ImportProblem[]
  onCancel: () => void
  onBook: () => void
  booking: boolean
}

/**
 * The auto-layout's plan, waiting for an answer: what would land where, said
 * in one line and drawn as ghosts in the month.
 *
 * Under the screen's head rather than in it: the head is what the month
 * shows and what can be done to it, and this is a question that outranks all
 * of that while it is up. Booking applies exactly the previewed array - the
 * backend refuses it whole if the calendar moved in between.
 */
export function LayoutPreview({ placements, problems = [], onCancel, onBook, booking }: Props) {
  const { t } = useTranslation()
  // Said the way the rest of the window says a day, not as the ISO the plan
  // carries. The plan is in date order, so its ends are its first and last.
  const first = placements[0]?.date
  const last = placements[placements.length - 1]?.date

  return (
    <div className="flex w-full flex-wrap items-center gap-3 rounded-md border border-accent bg-accent-soft px-3 py-2 text-sm">
      <span className="flex-1">
        {placements.length === 0
          ? t('calendar.files.nothingPlaced')
          : t('calendar.layoutPreview', {
              count: placements.length,
              from: first === undefined ? '' : formatDay(first),
              to: last === undefined ? '' : formatDay(last),
            })}
      </span>
      <Button size="sm" onClick={onCancel}>
        {t('dialog.cancel')}
      </Button>
      <Button
        size="sm"
        variant="primary"
        disabled={booking || placements.length === 0}
        onClick={onBook}
      >
        {t('calendar.layoutApply')}
      </Button>
      {problems.length > 0 && (
        <details className="w-full text-xs text-dim">
          <summary className="cursor-pointer">
            {t('calendar.files.leftOut', { count: problems.length })}
          </summary>
          <ul className="mt-1 flex flex-col gap-0.5">
            {problems.map((problem) => (
              <li key={problem.line} className="flex min-w-0 gap-2">
                <span className="w-10 shrink-0 font-mono text-faint">{problem.line}</span>
                <span className="min-w-0 truncate font-mono">{problem.text}</span>
                <span className="shrink-0">{t(REASONS[problem.reason])}</span>
              </li>
            ))}
          </ul>
        </details>
      )}
    </div>
  )
}

/** The words for why a row of an imported plan was left out. */
const REASONS: Record<ImportReason, string> = {
  badDate: 'calendar.files.reason.badDate',
  noWork: 'calendar.files.reason.noWork',
  ambiguousWork: 'calendar.files.reason.ambiguousWork',
  nothingWaiting: 'calendar.files.reason.nothingWaiting',
  ambiguousDoor: 'calendar.files.reason.ambiguousDoor',
  takenDay: 'calendar.files.reason.takenDay',
  closedDay: 'calendar.files.reason.closedDay',
  repeated: 'calendar.files.reason.repeated',
}
