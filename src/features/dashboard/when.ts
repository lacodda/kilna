import type { TFunction } from 'i18next'

/** "today", "in 3 days", "2 days late" — a distance, never a raw date. */
export function when(t: TFunction, daysLeft: number): string {
  if (daysLeft < 0) return t('dashboard.late', { count: -daysLeft })
  if (daysLeft === 0) return t('dashboard.today')
  return t('dashboard.inDays', { count: daysLeft })
}
