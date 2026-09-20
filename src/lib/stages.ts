import type { ProfileConfig, Stage } from '@/lib/api'

/**
 * The stops of the stage dial, and how a percentage reads against them.
 *
 * The craft names its own — a novel's "second draft" is not a song's "mixed" —
 * and a craft that names none gets the line's, because a dial with nothing to
 * snap to is not a dial. Answered in one place so a card, a row and a calendar
 * chip cannot disagree about what an empty list means.
 */
export const DEFAULT_STAGES: Stage[] = [
  { key: 'idea', label: 'Idea', percent: 0, colour: 'plain' },
  { key: 'raw', label: 'Rough draft', percent: 17, colour: 'plain' },
  { key: 'half', label: 'Half there', percent: 33, colour: 'warn' },
  { key: 'nearly', label: 'Nearly there', percent: 50, colour: 'warn' },
  { key: 'polish', label: 'Polishing', percent: 67, colour: 'warn' },
  { key: 'done', label: 'Finished', percent: 83, colour: 'accent' },
  { key: 'final', label: 'Final', percent: 100, colour: 'good' },
]

/** The stops this profile works by, in order. */
export function stagesOf(config: ProfileConfig): Stage[] {
  const named = config.stages ?? []
  const stops = named.length > 0 ? named : DEFAULT_STAGES
  return [...stops].sort((a, b) => a.percent - b.percent)
}

/**
 * The stop a percentage belongs to: the last one it has reached.
 *
 * Not the nearest. A work at 79 is still polishing, and rounding it up to
 * "Finished" would tell the author their song is done when they said it was
 * nearly.
 */
export function stageAt(
  config: ProfileConfig,
  percent: number | null | undefined,
): Stage | undefined {
  if (percent === null || percent === undefined) return undefined
  const stops = stagesOf(config)
  let found: Stage | undefined
  for (const stop of stops) {
    if (stop.percent <= percent) found = stop
  }
  return found
}
