import { useTranslation } from 'react-i18next'
import type { Guard, ProfileConfig } from '@/lib/api/types'
import { formatNumber } from '@/lib/format'
import { GUARD_DEFAULTS, guardOf, guardWith, type GuardField } from '@/lib/repeats'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { NumberField } from '@/components/ui/number-field'

/** The three numbers, each with what it is called and what it is counted in. */
const FIELDS: readonly { field: GuardField; label: string; unit: string }[] = [
  { field: 'window_days', label: 'editor.guardWindow', unit: 'editor.guardWindowUnit' },
  { field: 'rare_rank', label: 'editor.guardRank', unit: 'editor.guardRankUnit' },
  { field: 'rare_in_works', label: 'editor.guardWorks', unit: 'editor.guardWorksUnit' },
]

/**
 * How the guard of repeats reads "recent" and "rare" (v0.90, ADR 0054): the
 * window inside which a shared rare word is red, the rank of the language's
 * word list a word is rare from, and in how many of the owner's works it may
 * stand and still be rare.
 *
 * A profile that names no guard reads kilna's defaults, and the fields show
 * them as values rather than as blanks: the guard is always on, and an empty
 * box would read as "off". Tuned back to the defaults, the profile names none
 * again (`guardWith`).
 */
export function GuardEditor({
  config,
  onChange,
}: {
  config: ProfileConfig
  onChange: (guard: Guard | null) => void
}) {
  const { t } = useTranslation()
  const guard = guardOf(config)
  const tuned = config.guard != null

  return (
    <FieldGroup
      label={t('editor.guard')}
      help={
        <>
          {t('editor.guardHint')}{' '}
          {t('editor.guardDefaults', {
            window: formatNumber(GUARD_DEFAULTS.window_days, 0),
            rank: formatNumber(GUARD_DEFAULTS.rare_rank, 0),
            works: formatNumber(GUARD_DEFAULTS.rare_in_works, 0),
          })}
        </>
      }
    >
      <div className="flex flex-col gap-2">
        {FIELDS.map(({ field, label, unit }) => (
          // A row, not a `<label>`: a label around the field would forward a
          // click on its words to the first control inside it - the stepper's
          // minus - and take one off the number.
          <div key={field} className="flex flex-wrap items-center gap-2 text-sm">
            <span aria-hidden className="w-44 shrink-0 text-dim">
              {t(label)}
            </span>
            <NumberField
              className="w-72"
              min={1}
              step={1}
              largeStep={field === 'rare_rank' ? 1_000 : 10}
              value={guard[field]}
              unit={t(unit)}
              aria-label={t(label)}
              onValueChange={(value) => {
                // An emptied box is a number being retyped, not an answer:
                // the guard keeps what it had until a number is there.
                if (value === null) return
                onChange(guardWith(config, field, value))
              }}
            />
          </div>
        ))}
        {tuned && (
          <Button size="xs" variant="ghost" className="self-start" onClick={() => onChange(null)}>
            {t('editor.guardReset')}
          </Button>
        )}
      </div>
    </FieldGroup>
  )
}
