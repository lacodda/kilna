import { useTranslation } from 'react-i18next'
import type { Work } from '@/lib/api/types'
import { stageAt } from '@/lib/stages'
import { useProfile } from '@/lib/useProfile'
import { useStage } from '@/lib/useStage'
import { StageStops } from '@/components/StageStops'
import { Widget } from '@/features/work/tabs/overview/Widget'

/**
 * How far along the work is, as its author judges it: the five stops in a
 * row, the one it stands at marked, and each a press away (FR-03).
 *
 * Set here, with the same five dials the header's picker opens: the stage has
 * no tab of its own to go to, so the widget is where it is changed. No word
 * and no percentage beside them - until v0.119 the widget wrote the stop's
 * name, its number, a meter of the same number and the name of the next
 * stop, four ways of saying one thing a dial already draws.
 */
export function StageWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const stage = useStage(work.id)
  const current = stageAt(profile.config, work.stage)

  return (
    <Widget caption={t('stage.label')}>
      <StageStops
        label={t('stage.label')}
        pick="one"
        on={[current.percent]}
        disabled={stage.isPending}
        onPick={(next) => {
          stage.mutate(next)
        }}
      />
    </Widget>
  )
}
