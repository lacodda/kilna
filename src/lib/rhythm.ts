import type { MonthlyGoal, ProfileConfig } from '@/lib/api/types'
import { say } from '@/lib/useProfile'

/**
 * What a monthly goal counts, as one string - `tier:clip`, `kind:short`: a
 * value for a select, and a goal's name in a complaint.
 */
export function goalKey(goal: MonthlyGoal): string {
  return 'tier' in goal ? `tier:${goal.tier}` : `kind:${goal.kind}`
}

/** The goal a key names, asking for `count` a month. */
export function goalOf(key: string, count: number): MonthlyGoal {
  const name = key.slice(key.indexOf(':') + 1)
  return key.startsWith('tier:') ? { tier: name, count } : { kind: name, count }
}

/** What a goal may count in this profile: every tier a kind of work has,
 *  once each, and every kind - in the profile's own words. */
export function goalChoices(
  config: Pick<ProfileConfig, 'work_kinds'>,
): { key: string; what: 'tier' | 'kind'; label: string }[] {
  const tiers = new Map<string, string>()
  for (const kind of config.work_kinds) {
    for (const tier of kind.tiers ?? []) {
      if (!tiers.has(tier.key)) tiers.set(tier.key, say(tier.label))
    }
  }
  return [
    ...[...tiers].map(([key, label]) => ({ key: `tier:${key}`, what: 'tier' as const, label })),
    ...config.work_kinds.map((kind) => ({
      key: `kind:${kind.key}`,
      what: 'kind' as const,
      label: say(kind.label),
    })),
  ]
}
