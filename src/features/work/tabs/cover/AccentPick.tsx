import { useTranslation } from 'react-i18next'
import type { CoverAccent, CoverPaletteColour } from '@/lib/api/types'
import { HEX } from '@/lib/styleBrick'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { DraftText } from '@/features/cover/Section'

interface Props {
  accent: CoverAccent | null
  /** The channel's palette, from its card. */
  palette: CoverPaletteColour[]
  disabled?: boolean
  onChange: (accent: CoverAccent | null) => void
}

/**
 * The colour the composition and the lettering lean on: the disc behind the
 * hero, a drip of the lettering, a highlighted word of the captions.
 *
 * Offered from the channel's palette; picked, it belongs to the cover, with
 * the words a generator is given for it - a palette's line is a note about
 * the colour, and "hot pink" draws better than a hex alone. Both can be
 * written here.
 */
export function AccentPick({ accent, palette, disabled, onChange }: Props) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-1.5">
      {palette.length === 0 ? (
        <p className="text-xs text-faint">{t('cover.accent.noPalette')}</p>
      ) : (
        <ul aria-label={t('cover.accent.title')} className="flex flex-wrap gap-1.5">
          {palette.map((colour) => {
            const on = accent?.color.toUpperCase() === colour.color.toUpperCase()
            const name = `${colour.name} · ${colour.color}`
            return (
              <li key={colour.id}>
                <Button
                  variant="icon"
                  size={null}
                  aria-pressed={on}
                  aria-label={name}
                  title={name}
                  disabled={disabled}
                  onClick={() => onChange(on ? null : { name: colour.name, color: colour.color })}
                  className={cn(
                    'size-6 rounded-full ring-1 ring-line-2',
                    on && 'ring-2 ring-accent ring-offset-2 ring-offset-raise',
                  )}
                  style={{ background: colour.color }}
                />
              </li>
            )
          })}
        </ul>
      )}
      <div className="grid grid-cols-[minmax(0,1fr)_7rem] gap-1.5">
        <DraftText
          single
          label={t('cover.accent.words')}
          value={accent?.name ?? ''}
          placeholder={t('cover.accent.wordsPlaceholder')}
          disabled={disabled || accent === null}
          onCommit={(name) => accent !== null && onChange({ ...accent, name })}
        />
        <DraftText
          single
          label={t('cover.accent.colour')}
          value={accent?.color ?? ''}
          placeholder="#RRGGBB"
          disabled={disabled}
          onCommit={(text) => {
            const color = text.trim()
            if (color === '') onChange(null)
            else if (HEX.test(color))
              onChange({ name: accent?.name ?? '', color: color.toUpperCase() })
          }}
        />
      </div>
    </div>
  )
}
