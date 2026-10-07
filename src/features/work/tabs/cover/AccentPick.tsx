import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { CoverAccent, CoverPaletteColour, StyleBrick } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { HEX, colourFill, styleName } from '@/lib/styleBrick'
import { styleTypesOf, useProfile } from '@/lib/useProfile'
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
 * Offered from the channel's palette first - the house colours - and then
 * from the accents of the style dictionary (v0.90.3), one colour or a
 * gradient. Picked, it belongs to the cover, copied with the words a
 * generator is given for it - a palette's line is a note about the colour,
 * and "hot pink" draws better than a hex alone. Both can be written here; a
 * colour typed by hand is one colour.
 */
export function AccentPick({ accent, palette, disabled, onChange }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const all = useQuery(queries.styleBricksMatching(null, ''))
  const accentTypes = new Set(
    styleTypesOf(config)
      .filter((type) => type.form === 'accent')
      .map((type) => type.key),
  )
  const dictionary = (all.data ?? []).filter(
    (brick) =>
      accentTypes.has(brick.type_key) && brick.status === 'ready' && brick.colours[0] !== undefined,
  )
  const stops = accent?.stops ?? []

  return (
    <div className="flex flex-col gap-1.5">
      {palette.length === 0 && dictionary.length === 0 && (
        <p className="text-xs text-faint">{t('cover.accent.noPalette')}</p>
      )}
      {palette.length > 0 && (
        <Swatches label={t('cover.accent.fromChannel')}>
          {palette.map((colour) => {
            const on = isOn(accent, [colour.color])
            return (
              <Swatch
                key={colour.id}
                name={`${colour.name} · ${colour.color}`}
                fill={colour.color}
                on={on}
                disabled={disabled}
                onClick={() =>
                  onChange(on ? null : { name: colour.name, color: colour.color, stops: [] })
                }
              />
            )
          })}
        </Swatches>
      )}
      {dictionary.length > 0 && (
        <Swatches label={t('cover.accent.fromDictionary')}>
          {dictionary.map((brick) => {
            const on = isOn(accent, brick.colours)
            return (
              <Swatch
                key={brick.id}
                name={`${styleName(brick)} · ${brick.colours.join(' → ')}`}
                fill={colourFill(brick.colours)}
                on={on}
                disabled={disabled}
                onClick={() => onChange(on ? null : accentOf(brick))}
              />
            )
          })}
        </Swatches>
      )}
      {stops.length > 1 && (
        <p className="flex items-center gap-2 font-mono text-xs text-faint">
          <span
            aria-hidden
            className="inline-block h-3 w-10 rounded-sm ring-1 ring-line-2"
            style={{ background: colourFill(stops) }}
          />
          {t('cover.accent.gradient', { stops: stops.join(' → ') })}
        </p>
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
            const color = text.trim().toUpperCase()
            if (color === '') onChange(null)
            // The same first colour typed back keeps the gradient; another
            // colour typed by hand is one colour.
            else if (HEX.test(color) && color !== accent?.color.toUpperCase())
              onChange({ name: accent?.name ?? '', color, stops: [] })
          }}
        />
      </div>
    </div>
  )
}

/** An accent of the dictionary as the cover keeps it, the way the backend
 * copies one an idea names (`Accent::of_brick`). */
function accentOf(brick: StyleBrick): CoverAccent {
  return {
    name: brick.name,
    color: brick.colours[0] ?? '',
    stops: brick.colours.length > 1 ? [...brick.colours] : [],
  }
}

/** Whether the cover's accent is these colours: a flat one by its colour, a
 * gradient by every stop in order. */
function isOn(accent: CoverAccent | null, colours: readonly string[]): boolean {
  if (accent === null) return false
  const mine = accent.stops.length > 1 ? accent.stops : [accent.color]
  return (
    mine.length === colours.length &&
    mine.every((colour, at) => colour.toUpperCase() === colours[at]?.toUpperCase())
  )
}

function Swatches({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-faint">{label}</span>
      <ul aria-label={label} className="flex flex-wrap gap-1.5">
        {children}
      </ul>
    </div>
  )
}

interface SwatchProps {
  name: string
  fill: string | undefined
  on: boolean
  disabled: boolean | undefined
  onClick: () => void
}

function Swatch({ name, fill, on, disabled, onClick }: SwatchProps) {
  return (
    <li>
      <Button
        variant="icon"
        size={null}
        aria-pressed={on}
        aria-label={name}
        title={name}
        disabled={disabled}
        onClick={onClick}
        className={cn(
          'size-6 rounded-full ring-1 ring-line-2',
          on && 'ring-2 ring-accent ring-offset-2 ring-offset-raise',
        )}
        // The colour is the subject here, not the styling: it is the accent's own.
        style={{ background: fill }}
      />
    </li>
  )
}
