import { useTranslation } from 'react-i18next'
import type { Rhythm } from '@/lib/api/types'
import { FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NumberField } from '@/components/ui/number-field'

/** How often releases go out, and at what time of day by default. */
export function RhythmEditor({
  rhythm,
  onChange,
}: {
  rhythm: Rhythm | null | undefined
  onChange: (rhythm: Rhythm | null) => void
}) {
  const { t } = useTranslation()

  return (
    <FieldGroup label={t('editor.rhythm')} help={t('editor.rhythmHint')}>
      <div className="flex items-center gap-2">
        <NumberField
          className="w-44"
          min={1}
          step={1}
          value={rhythm?.every_days ?? null}
          unit={t('editor.rhythmDaysUnit')}
          aria-label={t('editor.rhythmDays')}
          onValueChange={(days) => {
            // Clearing the field removes the rhythm entirely — "no pace"
            // is a valid answer, and the layout button explains it.
            onChange(
              days === null
                ? null
                : {
                    every_days: Math.max(1, Math.trunc(days)),
                    default_time: rhythm?.default_time ?? null,
                  },
            )
          }}
        />
        <label className="ml-4 flex items-center gap-2 text-sm">
          {t('editor.rhythmTime')}
          <Input
            className="w-28"
            type="time"
            value={rhythm?.default_time ?? ''}
            disabled={rhythm == null}
            aria-label={t('editor.rhythmTime')}
            onChange={(event) => {
              if (rhythm == null) return
              onChange({
                ...rhythm,
                default_time: event.target.value === '' ? null : event.target.value,
              })
            }}
          />
        </label>
      </div>
    </FieldGroup>
  )
}
