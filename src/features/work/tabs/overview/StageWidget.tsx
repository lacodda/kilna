import { useTranslation } from 'react-i18next'
import type { Stage, Work } from '@/lib/api/types'
import { nextStage } from '@/lib/overview'
import { stageAt } from '@/lib/stages'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Progress } from '@/components/ui/progress'
import { StagePicker } from '@/components/StagePicker'
import { Invite, useLook, Widget } from '@/features/work/tabs/overview/Widget'

/**
 * How far along the work is, as its author judges it: the widget catalogue's
 * gauge - the stop it has reached, the share of the way, and the next stop.
 *
 * Set here, with the same dial the header carries: the stage has no tab of
 * its own to go to, so the widget is where it is changed.
 */
export function StageWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { presentation } = useLook()

  const current = stageAt(profile.config, work.stage)
  const next = nextStage(profile.config, work.stage)
  const picker = <StagePicker workId={work.id} percent={work.stage} />

  if (current === undefined || work.stage === null) {
    return (
      <Widget caption={t('stage.label')} tone="empty" actions={picker}>
        <Invite>{t('overview.noStage')}</Invite>
      </Widget>
    )
  }

  const percent = work.stage
  const meter = (
    <Progress
      size="md"
      tone={toneOf(current)}
      value={percent}
      label={t('stage.atPercent', { percent, stage: sayLabel(current.label) })}
    />
  )

  if (presentation !== 'card') {
    return (
      <Widget caption={t('stage.label')} actions={picker}>
        <div className="flex min-w-0 items-center gap-3">
          <b className="shrink-0 text-base font-semibold">{sayLabel(current.label)}</b>
          <span className="max-w-65 min-w-0 flex-1">{meter}</span>
          <span className="shrink-0 font-mono text-xs text-faint tabular-nums">{percent}%</span>
        </div>
      </Widget>
    )
  }

  return (
    <Widget
      caption={t('stage.label')}
      // Pulled into the caption's line, so the dial does not make this
      // caption taller than its neighbours'.
      aside={<StagePicker workId={work.id} percent={work.stage} className="-my-1.5" />}
    >
      <div className="flex min-w-0 items-baseline gap-2">
        <b className="truncate text-lg font-semibold">{sayLabel(current.label)}</b>
        <span className="font-mono text-xs text-faint tabular-nums">{percent}%</span>
      </div>
      {meter}
      <span className="text-xs text-faint">
        {next === undefined
          ? t('overview.lastStage')
          : t('overview.nextStage', { stage: sayLabel(next.label) })}
      </span>
    </Widget>
  )
}

/** The meter in the stop's own colour, where the craft gave it one the meter
 *  can wear; a plain stop is drawn in the accent. */
function toneOf(stage: Stage): 'accent' | 'good' | 'warn' | 'bad' {
  const colour = stage.colour
  return colour === 'good' || colour === 'warn' || colour === 'bad' ? colour : 'accent'
}
