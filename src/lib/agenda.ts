import type { TimeOff } from '@/lib/api/types'

/**
 * The agenda's runs: a month as a list, every day with something on it a
 * group of its own, and the days between folded into one line each (v0.96).
 *
 * A run of empty days is the agenda's answer to "is that week empty" - the
 * question the month grid answers by showing it. It is split where a closed
 * span starts or ends, so "Away" is one line and the three free days after it
 * another: a gap the plan should fill reads differently from days nobody
 * meant to fill.
 */
export type AgendaRun =
  | { kind: 'day'; date: string; closed: TimeOff | null }
  | { kind: 'gap'; from: string; to: string; days: number; closed: TimeOff | null }

/**
 * Fold `days` into runs. A day is shown on its own when it holds something,
 * or when it is today - the list's one fixed point, where reading starts.
 */
export function agendaRuns(
  days: readonly string[],
  holds: (date: string) => boolean,
  closed: ReadonlyMap<string, TimeOff>,
  now: string,
): AgendaRun[] {
  const runs: AgendaRun[] = []
  for (const date of days) {
    const span = closed.get(date) ?? null
    if (holds(date) || date === now) {
      runs.push({ kind: 'day', date, closed: span })
      continue
    }
    const last = runs[runs.length - 1]
    if (last !== undefined && last.kind === 'gap' && last.closed?.id === span?.id) {
      last.to = date
      last.days += 1
    } else {
      runs.push({ kind: 'gap', from: date, to: date, days: 1, closed: span })
    }
  }
  return runs
}
