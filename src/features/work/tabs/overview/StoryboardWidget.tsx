import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Work } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { durationOf, framesByScene } from '@/lib/scenes'
import { checkStoryboard } from '@/lib/storyboard'
import { useProfile, vocabularyOf } from '@/lib/useProfile'
import { Chip } from '@/components/ui/chip'
import { Loaded } from '@/components/Loaded'
import { MiniBars } from '@/features/work/tabs/overview/MiniBars'
import {
  Invite,
  InviteLink,
  useLook,
  Widget,
  WidgetSkeleton,
} from '@/features/work/tabs/overview/Widget'

/**
 * How far the storyboard has come: how many scenes, how many of them are
 * written, drawn and filmed, and whether anything is still owed - the Scenes
 * tab's own reckoning (`checkStoryboard`), read over the same rows and
 * pictures so the two cannot disagree.
 *
 * Only on a kind that has a storyboard: a song has none, and its overview has
 * no widget for one.
 */
export function StoryboardWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { presentation } = useLook()
  const blocks = vocabularyOf(profile.config, work.kind).scene_blocks

  const scenes = useQuery(queries.scenes(work.id))
  const frames = useQuery(queries.sceneFrames(work.id))

  const to = `/works/${work.id}/scenes`
  const empty = scenes.data?.length === 0

  return (
    <Widget
      caption={t('card.tab.scenes')}
      to={to}
      tone={empty ? 'empty' : 'plain'}
      actions={empty ? <InviteLink to={to}>{t('overview.startBoard')}</InviteLink> : undefined}
    >
      <Loaded query={scenes} plain skeleton={<WidgetSkeleton lines={3} />}>
        {(rows) => {
          if (rows.length === 0) return <Invite>{t('overview.noScenes')}</Invite>

          const { tally, complaints } = checkStoryboard(
            rows,
            blocks,
            framesByScene(frames.data ?? []),
            durationOf(work),
          )
          const owed = (
            <Chip variant={complaints.length === 0 ? 'good' : 'warn'}>
              {complaints.length === 0
                ? t('scenes.check.chipDone')
                : t('scenes.check.chip', { number: complaints.length })}
            </Chip>
          )

          if (presentation !== 'card') {
            return (
              <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
                <span className="text-sm text-dim">{t('scenes.check.tally', { ...tally })}</span>
                {owed}
              </div>
            )
          }

          const of = (count: number, key: string, label: string) => ({
            key,
            label,
            value: count,
            max: tally.scenes,
            shown: String(count),
          })
          return (
            <>
              <div className="flex min-w-0 items-baseline gap-2.5">
                <span className="font-mono text-2xl leading-none font-semibold tabular-nums">
                  {tally.scenes}
                </span>
                <span className="text-xs text-faint">
                  {t('overview.scenesCount', { count: tally.scenes })}
                </span>
                {owed}
              </div>
              <MiniBars
                bars={[
                  of(tally.written, 'written', t('overview.scenesWritten')),
                  of(tally.framed, 'framed', t('overview.scenesFramed')),
                  of(tally.filmed, 'filmed', t('overview.scenesFilmed')),
                ]}
              />
            </>
          )
        }}
      </Loaded>
    </Widget>
  )
}
