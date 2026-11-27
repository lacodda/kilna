import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isColour, pickerValue } from '@/lib/colour'
import { Input } from '@/components/ui/input'

/**
 * A colour: the system's picker, and the same colour as `#RRGGBB` text
 * beside it. What is typed is told only once it is a whole colour, so a
 * value half typed never reaches what is drawn from it.
 */
export function ColourField({
  label,
  value,
  onChange,
}: {
  label: string
  value: string
  onChange: (value: string) => void
}) {
  const { t } = useTranslation()
  const [typed, setTyped] = useState<string | null>(null)
  return (
    <span className="flex items-center gap-2">
      <input
        type="color"
        aria-label={t('colour.pick', { what: label })}
        className="size-7 cursor-pointer rounded-md border border-line bg-transparent p-0"
        value={pickerValue(value)}
        onChange={(event) => onChange(event.target.value.toUpperCase())}
      />
      <Input
        aria-label={label}
        className="w-28 font-mono"
        value={typed ?? value}
        onChange={(event) => {
          const text = event.target.value.trim()
          setTyped(text)
          if (isColour(text)) onChange(text.toUpperCase())
        }}
        onBlur={() => setTyped(null)}
      />
    </span>
  )
}
