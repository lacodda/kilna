import { useTranslation } from 'react-i18next'
import { NO_STAGE, stagesOf } from '@/lib/stages'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { CheckList } from '@/components/CheckList'
import { StageDial } from '@/components/StageDial'

interface Props {
  /** The ticked stops, as their percentages, and `NO_STAGE`. */
  chosen: readonly number[]
  onChange: (next: number[]) => void
}

/**
 * The stops of the stage dial as boxes to tick: what a list is narrowed to
 * by how far along its works are. The catalogue's column funnel and the
 * calendar's queue both hold this (`standsAt` is how either reads it).
 *
 * The dial the rows are drawn with, the stop's name, and the fraction it
 * stands for.
 *
 * The name was left out when this was written, on the argument that a funnel
 * of six words beside a column of six rings asks the eye to match one against
 * the other. What it actually produced was a funnel of bare percentages - a
 * scale nobody had been told the meaning of, since "60%" is not what the
 * card, the picker or the row call that stop. The dial carries the matching;
 * the word says which stop it is; the percentage stays for the person who
 * thinks in the number.
 *
 * "No stage" leads, because it is the answer to a different question from
 * the stops below it - not how far along, but never said - and it is the
 * entry the owner went looking for and could not find.
 */
export function StageChecks({ chosen, onChange }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  return (
    <CheckList
      options={[
        { value: NO_STAGE, label: t('stage.unset') },
        ...stagesOf(profile.config).map((stop) => ({
          value: stop.percent,
          label: t('stage.atPercent', { stage: sayLabel(stop.label), percent: stop.percent }),
          icon: <StageDial percent={stop.percent} stage={stop} size={16} className="shrink-0" />,
        })),
      ]}
      chosen={chosen}
      onChange={onChange}
    />
  )
}
