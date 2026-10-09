import type {
  Lab,
  ProfileConfig,
  PromptTemplate,
  TrialBoard,
  TrialCard,
  TrialRequest,
} from '@/lib/api/types'
import { vocabularyOf } from '@/lib/useProfile'

/**
 * An experiment's board of trials (v0.95, ADR 0061), as the window reads it:
 * which trials a filter shows, how a series is laid out as a tree, what a
 * task about the board is called and what it asks for.
 */

/** The most trials one run is asked for - the backend's `answer::MOST`. */
export const MOST_TRIALS = 12

/** How many a sweep of the field asks for when nobody says. */
export const TRIALS_ON_SWEEP = 6

/** How many variations around a core one ask makes, when nobody says. */
export const TRIALS_AROUND = 4

/** The board's filters, in the plan's order. */
export const TRIAL_FILTERS = ['all', 'keep', 'open', 'drop'] as const
export type TrialFilter = (typeof TRIAL_FILTERS)[number]

/** Whether a trial shows under a filter. A dropped trial stays under "all",
 *  stepped back: it is still an example of what does not work. */
function shows(card: TrialCard, filter: TrialFilter): boolean {
  const verdict = card.trial.verdict
  switch (filter) {
    case 'all':
      return true
    case 'keep':
      return verdict === 'keep'
    case 'open':
      return verdict === null
    case 'drop':
      return verdict === 'drop'
  }
}

/** How many trials each filter shows. */
export function counts(cards: readonly TrialCard[]): Record<TrialFilter, number> {
  const out = { all: 0, keep: 0, open: 0, drop: 0 }
  for (const card of cards) {
    for (const filter of TRIAL_FILTERS) if (shows(card, filter)) out[filter] += 1
  }
  return out
}

/** A trial as a row of the tree: how deep it stands under the trial it varies. */
export interface TreeRow {
  card: TrialCard
  depth: number
}

/** A series of the board, its trials as a tree. */
export interface SeriesRows {
  series: string
  rows: TreeRow[]
}

/**
 * The board as the list draws it: series in the order each began, and in
 * each the trials as a tree - a variation under the trial it varies, when
 * that one stands in the same series; at the root of its series otherwise
 * (a variation sent to a series of its own). Siblings keep the board's order.
 * A trial a filter hides takes nothing with it: its children rise to where it
 * stood, so a kept variation of a dropped core still shows under "kept".
 */
export function seriesOf(board: TrialBoard, filter: TrialFilter = 'all'): SeriesRows[] {
  const out: SeriesRows[] = []
  for (const series of board.series) {
    const inSeries = board.trials.filter((card) => card.trial.series === series)
    const ids = new Set(inSeries.map((card) => card.trial.id))
    const children = new Map<string, TrialCard[]>()
    const roots: TrialCard[] = []
    for (const card of inSeries) {
      const parent = card.trial.parent_id
      if (parent !== null && ids.has(parent)) {
        children.set(parent, [...(children.get(parent) ?? []), card])
      } else {
        roots.push(card)
      }
    }
    const rows: TreeRow[] = []
    const walk = (card: TrialCard, depth: number) => {
      const shown = shows(card, filter)
      if (shown) rows.push({ card, depth })
      for (const child of children.get(card.trial.id) ?? []) walk(child, shown ? depth + 1 : depth)
    }
    for (const root of roots) walk(root, 0)
    if (rows.length > 0) out.push({ series, rows })
  }
  return out
}

/** The lab of a kind, when its works are experiments. */
export function labOf(config: ProfileConfig, kind: string | undefined): Lab | null {
  if (kind === undefined) return null
  return config.work_kinds.find((entry) => entry.key === kind)?.lab ?? null
}

/** The kinds a kept trial of an experiment of `kind` may go into: every other
 *  kind that has the role its lab keeps trials in. */
export function harvestKindsOf(config: ProfileConfig, kind: string): string[] {
  const role = labOf(config, kind)?.harvest ?? null
  if (role === null) return []
  return config.work_kinds
    .filter(
      (entry) =>
        entry.key !== kind &&
        vocabularyOf(config, entry.key).version_roles.some((one) => one.key === role),
    )
    .map((entry) => entry.key)
}

/** The lab kinds an experiment can be made of a work of `kind` in: those
 *  whose trials go into a role the work has. */
export function labKindsFor(config: ProfileConfig, kind: string): string[] {
  return config.work_kinds
    .filter((entry) => entry.lab !== null && entry.lab !== undefined && entry.key !== kind)
    .filter((entry) => harvestKindsOf(config, entry.key).includes(kind))
    .map((entry) => entry.key)
}

/** The profile's action that proposes trials for a board of this kind. */
export function labActionOf(
  config: ProfileConfig,
  kind: string | undefined,
): PromptTemplate | undefined {
  return config.prompts.find(
    (prompt) =>
      prompt.scope === 'lab' &&
      prompt.produces === 'trials' &&
      ((prompt.kinds ?? []).length === 0 || (kind !== undefined && prompt.kinds!.includes(kind))),
  )
}

/** The key a board's task runs under - the backend's `lab_key`. */
export function labTaskKey(action: string, workId: string, request: TrialRequest): string {
  return `${action}:lab:${workId}:${request.around ?? '-'}:${request.fix ? 'fix' : 'vary'}:${request.series.trim()}`
}

/** The experiment a task key is about, when it is a board's. */
export function boardOfLabTask(key: string | undefined): string | null {
  if (key === undefined) return null
  const [, scope, workId] = key.split(':')
  return scope === 'lab' && workId !== undefined && workId !== '' ? workId : null
}

/** What a task asks about, read back from its key: the trial varied or fixed. */
export function aroundOfLabTask(key: string): string | null {
  const around = key.split(':')[3]
  return around === undefined || around === '-' ? null : around
}

/** A sweep of the field: `count` trials as far apart as the direction allows. */
export function sweep(count: number, series: string): TrialRequest {
  return { count, series, around: null, fix: false }
}

/** Variations around a trial, each moving one thing of it. */
export function around(trialId: string, count: number, series: string): TrialRequest {
  return { count, series, around: trialId, fix: false }
}

/** The fix of a trial that was heard: one variation answering what came out. */
export function fix(trialId: string, series: string): TrialRequest {
  return { count: 1, series, around: trialId, fix: true }
}

/** How many trials a request makes - the rows drawn while it runs. */
export function totalOf(request: TrialRequest): number {
  return request.fix ? 1 : request.count
}

/** The first words of a trial's text, for a row with no angle. */
export function opening(body: string, length = 60): string {
  const line = body.replace(/\s+/g, ' ').trim()
  return line.length > length ? `${line.slice(0, length - 1)}…` : line
}
