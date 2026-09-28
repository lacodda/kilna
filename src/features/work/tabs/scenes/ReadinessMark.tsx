import { useTranslation } from 'react-i18next'
import type { Readiness } from '@/lib/scenes'
import { Chip } from '@/components/ui/chip'

/** The chip each state wears: the mockup's `chip good / acc / warn / dash`. */
const TONE = {
  shot: 'good',
  ready: 'accent',
  started: 'warn',
  empty: 'dashed',
} as const satisfies Record<Readiness, string>

/**
 * How far a scene is filled in, as a chip.
 *
 * Computed from what the scene holds, never stored: ADR 0020 turned down a
 * status column for the frame because it derives from what is there. A kind
 * with no prompt blocks asks only for a description, and the chip's tooltip
 * says so rather than implying blocks nobody named.
 *
 * `shot` is a step past `ready`, so it wears the good colour where `ready`
 * wears the accent - the same judgement, one step further - and an untouched
 * row wears a broken border, the mark of something not there yet.
 */
export function ReadinessMark({ readiness, blocks }: { readiness: Readiness; blocks: number }) {
  const { t } = useTranslation()
  const hint =
    readiness === 'shot'
      ? t('scenes.readinessHintShot')
      : blocks > 0
        ? t('scenes.readinessHint')
        : t('scenes.readinessHintNoBlocks')

  return (
    <Chip variant={TONE[readiness]} title={hint}>
      {t(`scenes.readiness_${readiness}`)}
    </Chip>
  )
}
