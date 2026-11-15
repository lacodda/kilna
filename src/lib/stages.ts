import type { ProfileConfig, Stage } from '@/lib/api/types'

/**
 * The stops of the stage dial, and how a percentage reads against them.
 *
 * The craft names its own — a novel's "second draft" is not a song's "mixed" —
 * and a craft that names none gets the line's, because a dial with nothing to
 * snap to is not a dial. Answered in one place so a card, a row and a calendar
 * chip cannot disagree about what an empty list means.
 */
const DEFAULT_STAGES: Stage[] = [
  { key: 'idea', label: 'Idea', percent: 0, colour: 'plain' },
  { key: 'raw', label: 'Rough draft', percent: 17, colour: 'plain' },
  { key: 'half', label: 'Half there', percent: 33, colour: 'warn' },
  { key: 'nearly', label: 'Nearly there', percent: 50, colour: 'warn' },
  { key: 'polish', label: 'Polishing', percent: 67, colour: 'warn' },
  { key: 'done', label: 'Finished', percent: 83, colour: 'accent' },
  { key: 'final', label: 'Final', percent: 100, colour: 'good' },
]

/**
 * The ticked entry standing for "no stage at all".
 *
 * A stage is stored as the percentage of its stop, so a filter could say
 * "standing at this stop" and had no way to say "standing nowhere" - and the
 * works that had never been given a stage were exactly the ones the owner
 * wanted to find. Negative, because every real stop is 0..=100, and a
 * sentinel that cannot collide with a value is the only kind worth having.
 */
export const NO_STAGE = -1

/**
 * Whether a work stands at one of the ticked stops.
 *
 * A work with no stage matches only the "no stage" tick, and a work with one
 * matches only its own stop. Ticking several asks for any of them, the way
 * two ticked marks do. One rule for the catalogue's funnel and the calendar's
 * queue (v0.111): the queue used to ask "has reached this stop" through a
 * dropdown of its own, which could not say "no stage" and read a stop's
 * number differently from the column beside it.
 */
export function standsAt(stage: number | null, ticked: ReadonlySet<number>): boolean {
  return stage === null ? ticked.has(NO_STAGE) : ticked.has(stage)
}

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
