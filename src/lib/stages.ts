import type { ProfileConfig, Stage } from '@/lib/api/types'

/**
 * The five places a work can stand at (FR-03, v0.119): a quarter each.
 *
 * One scale for every craft, so a dial reads the same in every profile. Until
 * v0.119 there were seven stops, and a work nobody had judged held no stage at
 * all: nobody could say what told two neighbouring stops apart, and "nobody
 * has said" drew like a bare idea and meant the same to anyone looking for
 * what to pick up. A work starts at the first stop now.
 */
const STOPS = [0, 25, 50, 75, 100] as const

/**
 * The line's own words for the five stops: what answers for a stop a profile
 * leaves out, so a card, a row and a calendar chip cannot disagree about what
 * a gap means. The words `default_stages` holds in Rust.
 *
 * Without the exact colours: those are the profile's, and every profile the
 * line ships, and every stored one it carries forward, has the five with
 * theirs. A stop found only here is drawn in the palette role it names -
 * the window writes no colour of its own.
 */
const DEFAULT_STAGES: Stage[] = [
  stop('idea', 'idea / untouched', 'идея / ещё не трогали', 0, 'plain'),
  stop('start', 'started / first drafts', 'старт / первые наброски', 25, 'info'),
  stop('half', 'halfway / working draft', 'половина / рабочий черновик', 50, 'warn'),
  stop('almost', 'almost ready / polishing', 'почти готово / доводка и полировка', 75, 'accent'),
  stop('final', 'final / ready to ship', 'финал, можно выпускать', 100, 'good'),
]

function stop(
  key: string,
  en: string,
  ru: string,
  percent: number,
  colour: Stage['colour'],
): Stage {
  return { key, label: { en, ru }, percent, colour }
}

/**
 * A number put on the scale: the stop nearest to it.
 *
 * What the backend does to everything it stores (`normalize_stage`); here for
 * what the window itself still holds from before - a saved view's ticked
 * stops, a search line written as `stage:83`. Anything below the scale - the
 * old "no stage" tick was `-1` - is the first stop, which is where the works
 * it meant now stand.
 */
export function stopOf(percent: number): number {
  if (percent <= 12) return 0
  if (percent <= 37) return 25
  if (percent <= 62) return 50
  if (percent <= 87) return 75
  return 100
}

/**
 * Whether a work stands at one of the ticked stops. Ticking several asks for
 * any of them, the way two ticked marks do. One rule for the catalogue's
 * funnel and the calendar's queue (v0.111).
 */
export function standsAt(stage: number, ticked: ReadonlySet<number>): boolean {
  return ticked.has(stage)
}

/** The five stops as this profile words them, in order. */
export function stagesOf(config: ProfileConfig): Stage[] {
  const named = config.stages ?? []
  return STOPS.map(
    (percent) =>
      named.find((one) => one.percent === percent) ??
      // Every stop is in the line's own list.
      DEFAULT_STAGES.find((one) => one.percent === percent)!,
  )
}

/** The stop a work stands at. */
export function stageAt(config: ProfileConfig, percent: number): Stage {
  const at = stopOf(percent)
  return stagesOf(config).find((one) => one.percent === at)!
}
