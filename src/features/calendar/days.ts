import type { TFunction } from 'i18next'
import type { TimeOff } from '@/lib/api/types'
import type { RowAction } from '@/components/RowMenu'

/**
 * The language a grid of days is drawn in, with the week starting on Monday
 * whatever the language would say: a release plan is read in working weeks
 * (`lib/month`), and English alone would otherwise start it on Sunday.
 */
export function mondayFirst(language: string): string {
  return `${language}-u-fw-mon`
}

/**
 * The right-click menu of a day, the same on the month, the week and the
 * agenda: add a work that goes out on it, close it (or edit the span that
 * closes it), and shift everything planned from it on.
 */
export function dayActions(
  t: TFunction,
  span: TimeOff | null,
  run: { add: () => void; timeOff: () => void; shift: () => void },
): RowAction[] {
  return [
    { key: 'add', label: t('calendar.addOnDay'), onSelect: run.add },
    {
      key: 'timeOff',
      label: span === null ? t('timeOff.closeDay') : t('timeOff.editNamed', { title: span.label }),
      onSelect: run.timeOff,
    },
    { key: 'shift', label: t('shift.fromDay'), onSelect: run.shift },
  ]
}
