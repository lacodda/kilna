import { useTranslation } from 'react-i18next'
import type {
  CoverColumn,
  CoverCrop,
  CoverLayoutOption,
  CoverPlace,
  CoverRow,
  CoverSize,
  Framing,
} from '@/lib/api/types'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SchemeView } from '@/features/cover/SchemeView'

const COLUMNS: readonly CoverColumn[] = ['left', 'centre', 'right']
const ROWS: readonly CoverRow[] = ['top', 'middle', 'bottom']
const SIZES: readonly CoverSize[] = ['small', 'emblem', 'large', 'full', 'over', 'macro']
const CROPS: readonly CoverCrop[] = ['full', 'waist', 'bust', 'head', 'detail']
const PLACES: readonly CoverPlace[] = [
  'left',
  'right',
  'top',
  'bottom',
  'behind',
  'overlap',
  'vertical',
  'corner',
  'poster',
]

interface Props {
  framing: Framing | null
  /** The eight layouts, each with its starting frame drawn small. */
  layouts: CoverLayoutOption[]
  /** Whether the picture carries words, and so a place for its title. */
  lettering: boolean
  disabled?: boolean
  onChange: (framing: Framing) => void
}

/**
 * The built frame (v0.88): eight layouts drawn small, then where the hero
 * stands on a three by three grid, how big, how much of them shows and -
 * on a picture with words - where the title goes.
 *
 * Picking a layout sets the other five to its own; each can then be moved
 * on its own, and the layout still says how the objects are composed. The
 * thumbnails are the layouts' starting frames as the backend draws them, so
 * a layout looks here the way it will look on the stage beside the prompt.
 */
export function FramingPicker({ framing, layouts, lettering, disabled, onChange }: Props) {
  const { t } = useTranslation()
  const set = (patch: Partial<Framing>) => {
    if (framing !== null) onChange({ ...framing, ...patch })
  }

  return (
    <div className="flex flex-col gap-2.5">
      <ul aria-label={t('cover.framing.layouts')} className="grid grid-cols-4 gap-1.5">
        {layouts.map((option) => {
          const on = framing?.layout === option.framing.layout
          const name = t(`cover.layout.${option.framing.layout}`)
          return (
            <li key={option.framing.layout} className="flex">
              <Button
                variant="ghost"
                size={null}
                aria-pressed={on}
                disabled={disabled}
                title={name}
                onClick={() => onChange(option.framing)}
                className={cn(
                  'w-full flex-col items-stretch gap-1 p-1 text-left text-2xs font-normal whitespace-normal',
                  on && 'border-accent text-accent',
                )}
              >
                <SchemeView scheme={option.scheme} label={name} decorative />
                <span className="truncate">{name}</span>
              </Button>
            </li>
          )
        })}
      </ul>

      {framing !== null && (
        <div className="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
          <span className="caption">{t('cover.framing.where')}</span>
          <SegmentedControl
            aria-label={t('cover.framing.where')}
            value={`${framing.column}:${framing.row}`}
            disabled={disabled}
            onValueChange={(next) => {
              const [column, row] = next.split(':') as [CoverColumn, CoverRow]
              set({ column, row })
            }}
            className="grid h-auto w-fit grid-cols-3 gap-0.5"
          >
            {ROWS.flatMap((row) =>
              COLUMNS.map((column) => (
                <Segment
                  key={`${column}:${row}`}
                  value={`${column}:${row}`}
                  aria-label={t('cover.framing.at', {
                    column: t(`cover.column.${column}`),
                    row: t(`cover.row.${row}`),
                  })}
                  className="h-4.5 w-6.5 px-0"
                >
                  <span aria-hidden className="size-1.5 rounded-full bg-current" />
                </Segment>
              )),
            )}
          </SegmentedControl>

          <span className="caption">{t('cover.framing.size')}</span>
          <SegmentedControl
            aria-label={t('cover.framing.size')}
            value={framing.size}
            disabled={disabled}
            onValueChange={(size) => set({ size: size as CoverSize })}
            className="h-auto flex-wrap"
          >
            {SIZES.map((size) => (
              <Segment key={size} value={size} className="px-2 py-0.5 text-xs">
                {t(`cover.size.${size}`)}
              </Segment>
            ))}
          </SegmentedControl>

          <span className="caption">{t('cover.framing.crop')}</span>
          <SegmentedControl
            aria-label={t('cover.framing.crop')}
            value={framing.crop}
            disabled={disabled}
            onValueChange={(crop) => set({ crop: crop as CoverCrop })}
            className="h-auto flex-wrap"
          >
            {CROPS.map((crop) => (
              <Segment key={crop} value={crop} className="px-2 py-0.5 text-xs">
                {t(`cover.crop.${crop}`)}
              </Segment>
            ))}
          </SegmentedControl>

          {lettering && (
            <>
              <span className="caption">{t('cover.framing.place')}</span>
              <SegmentedControl
                aria-label={t('cover.framing.place')}
                value={framing.place}
                disabled={disabled}
                onValueChange={(place) => set({ place: place as CoverPlace })}
                className="h-auto flex-wrap"
              >
                {PLACES.map((place) => (
                  <Segment key={place} value={place} className="px-2 py-0.5 text-xs">
                    {t(`cover.place.${place}`)}
                  </Segment>
                ))}
              </SegmentedControl>
            </>
          )}
        </div>
      )}
    </div>
  )
}
