import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { zoneLabel, zoneNames } from '@/lib/zonedTime'
import {
  Combobox,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
  ComboboxPopup,
} from '@/components/ui/combobox'

interface Props {
  /** An IANA zone name, `America/Asuncion`. */
  value: string
  onChange: (zone: string) => void
  className?: string
  'aria-label'?: string
}

/**
 * Whose clock a release's hour is on (v0.97): every zone the runtime knows,
 * narrowed as a city is typed - "asun" finds Asuncion, "mosc" Moscow. The
 * list is the runtime's, so a zone added to the world next year is offered
 * by the next WebView rather than by the next kilna.
 */
export function ZonePicker({ value, onChange, className, 'aria-label': label }: Props) {
  const { t } = useTranslation()
  const zones = useMemo(() => zoneNames(), [])

  return (
    <Combobox
      items={zones}
      value={value === '' ? null : value}
      onValueChange={(next) => {
        if (typeof next === 'string' && next !== '') onChange(next)
      }}
      itemToStringLabel={(zone: string) => zoneLabel(zone)}
    >
      <ComboboxInput
        aria-label={label ?? t('calendar.zone')}
        placeholder={t('calendar.zonePlaceholder')}
        className={className}
      />
      <ComboboxPopup className="max-h-72 w-72 p-1">
        <ComboboxEmpty className="px-2 py-1.5 text-xs text-faint">
          {t('calendar.zoneNone')}
        </ComboboxEmpty>
        <ComboboxList>
          {(zone: string) => (
            <ComboboxItem key={zone} value={zone} className="rounded-md px-2 py-1 text-sm">
              {zoneLabel(zone)}
            </ComboboxItem>
          )}
        </ComboboxList>
      </ComboboxPopup>
    </Combobox>
  )
}
