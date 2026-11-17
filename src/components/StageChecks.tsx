import { useTranslation } from 'react-i18next'
import { stopOf } from '@/lib/stages'
import { StageStops } from '@/components/StageStops'

interface Props {
  /** The ticked stops, as their percentages. */
  chosen: readonly number[]
  onChange: (next: number[]) => void
}

/**
 * The five stops as dials to tick: what a list is narrowed to by how far
 * along its works are. The catalogue's column funnel and the calendar's queue
 * both hold this (`standsAt` is how either reads it).
 *
 * A row of five dials and no words (FR-03). Until v0.119 it was a list of
 * boxes, each with its stop's name and percentage, led by "Stage not set" -
 * the entry for works nobody had judged. Those works stand at the first stop
 * now, and that is the dial that finds them.
 *
 * What a saved view ticked on the older scale is read as the stop it lands
 * on, the old "not set" among them.
 */
export function StageChecks({ chosen, onChange }: Props) {
  const { t } = useTranslation()
  const ticked = [...new Set(chosen.map(stopOf))]
  return (
    <StageStops
      label={t('stage.label')}
      pick="any"
      on={ticked}
      onPick={(percent) => {
        onChange(
          ticked.includes(percent)
            ? ticked.filter((one) => one !== percent)
            : [...ticked, percent].sort((a, b) => a - b),
        )
      }}
      className="px-1 py-0.5"
    />
  )
}
