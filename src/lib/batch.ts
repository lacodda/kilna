import i18n from '@/i18n'
import type { Skipped } from '@/lib/api/types'
import { sayReason } from '@/lib/errors'

/** How many passed-over items a toast names before it counts the rest. */
const NAMED = 5

/**
 * What to say about the items a batch passed over: a headline with the count,
 * and a line per item - its name and why - so "two were skipped" comes with
 * which two, and for what.
 *
 * `null` when nothing was skipped: a batch that reached everything says only
 * what it did.
 */
export function describeSkipped(
  skipped: readonly Skipped[],
): { title: string; description: string } | null {
  if (skipped.length === 0) return null
  const t = i18n.t.bind(i18n)
  const lines = skipped.slice(0, NAMED).map((item) =>
    t('batch.skippedRow', {
      title: item.title ?? t('batch.untitled'),
      reason: sayReason(item.reason),
    }),
  )
  if (skipped.length > NAMED) {
    lines.push(t('batch.andMore', { count: skipped.length - NAMED }))
  }
  return {
    title: t('batch.skipped', { count: skipped.length }),
    description: lines.join('\n'),
  }
}
