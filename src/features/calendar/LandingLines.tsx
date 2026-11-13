import { useTranslation } from 'react-i18next'
import type { SlotPreview, SlotVerdict } from '@/lib/api/types'
import { landingWarning } from '@/lib/repeats'

/**
 * How a day's verdict reads while something is dragged over it.
 *
 * Neither is a refusal - a day holds as many releases as are put on it - so
 * neither is red. Something already there is information; a date settled by
 * hand is worth a warmer look before adding beside it. Written as a record
 * over the verdicts, so a verdict the backend adds is a type error here until
 * it has a tone.
 */
const VERDICT_TONE: Record<SlotVerdict, string> = {
  // Never drawn: an empty day says nothing while something is dragged over it.
  empty: '',
  taken: 'text-dim',
  pinned: 'text-warn',
}

/** Whether a day has anything to say about what would land on it. */
export function landingSays(verdict: SlotPreview | null): verdict is SlotPreview {
  if (verdict === null) return false
  return (
    verdict.closed !== null ||
    verdict.verdict !== 'empty' ||
    landingWarning(verdict.repeats) !== null
  )
}

/**
 * What landing on a day would mean, a line each: that the day is closed, who
 * is there, and what the release would repeat there (ADR 0054) - the repeat
 * last, nearest the foot, in the guard's red: it is the one that may change
 * where the release goes. Spans rather than paragraphs, so they can stand in
 * the one paragraph the month lays over the foot of a day.
 */
export function LandingLines({ verdict }: { verdict: SlotPreview }) {
  const { t } = useTranslation()
  const repeats = landingWarning(verdict.repeats)
  return (
    <>
      {verdict.closed !== null && (
        <span className="block text-warn">
          {t('timeOff.landsOnClosed', { title: verdict.closed })}
        </span>
      )}
      {verdict.verdict !== 'empty' && (
        <span className={`block ${VERDICT_TONE[verdict.verdict]}`}>
          {t(`calendar.preview.${verdict.verdict}`, { title: verdict.holder_title ?? '' })}
        </span>
      )}
      {repeats !== null && (
        <span role="status" className="block text-bad">
          {repeats}
        </span>
      )}
    </>
  )
}
