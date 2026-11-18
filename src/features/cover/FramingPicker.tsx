import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Ban, Star } from 'lucide-react'
import { useStarredLayouts } from '@/lib/starredLayouts'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import type {
  CoverColumn,
  CoverCrop,
  CoverLayoutOption,
  CoverPlace,
  CoverRow,
  CoverSize,
  CoverSubject,
  Framing,
} from '@/lib/api/types'
import { cn } from '@/lib/utils'
import { TileButton } from '@/components/ui/tile-button'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SchemeView } from '@/features/cover/SchemeView'
import { DiscreteScalePicker } from '@/components/DiscreteScalePicker'

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
  /** What the picture is of: with it, the shot scale starts at "no
   *  subject", which makes a picture of the scene alone (v0.135). */
  subject?: CoverSubject
  onSubject?: (subject: CoverSubject) => void
  disabled?: boolean
  onChange: (framing: Framing) => void
  /** "No frame" in one press: the picture is staged by its scene alone. */
  onClear?: () => void
}

/** Past this many layouts the list offers a search. */
const SEARCHED_FROM = 9

/** The shot scale's stops: no subject at all, then how much of it shows. */
type Shot = 'none' | CoverCrop

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
export function FramingPicker({
  framing,
  layouts,
  lettering,
  subject,
  onSubject,
  disabled,
  onChange,
  onClear,
}: Props) {
  const { t } = useTranslation()
  const [starred, setStarred] = useStarredLayouts()
  const [text, setText] = useState('')
  const needle = text.trim().toLowerCase()
  // The starred first, then the rest in their order; a search narrows both.
  const listed = layouts
    .filter(
      (option) =>
        needle === '' || t(`cover.layout.${option.framing.layout}`).toLowerCase().includes(needle),
    )
    .sort(
      (a, b) =>
        Number(starred.includes(b.framing.layout)) - Number(starred.includes(a.framing.layout)),
    )
  const set = (patch: Partial<Framing>) => {
    if (framing !== null) onChange({ ...framing, ...patch })
  }

  return (
    <div className="flex flex-col gap-2.5">
      {layouts.length + 1 >= SEARCHED_FROM && (
        <Input
          type="search"
          value={text}
          placeholder={t('cover.framing.search')}
          aria-label={t('cover.framing.search')}
          onChange={(event) => setText(event.target.value)}
        />
      )}
      <ul aria-label={t('cover.framing.layouts')} className="grid grid-cols-4 gap-1.5">
        {onClear !== undefined && needle === '' && (
          <li className="flex">
            <TileButton
              selected={framing === null}
              aria-pressed={framing === null}
              disabled={disabled}
              onClick={onClear}
              className="w-full items-center justify-center gap-1 p-1 text-2xs"
            >
              <Ban aria-hidden className="size-5 text-faint" />
              <span className="truncate">{t('cover.framing.none')}</span>
            </TileButton>
          </li>
        )}
        {listed.map((option) => {
          const on = framing?.layout === option.framing.layout
          const name = t(`cover.layout.${option.framing.layout}`)
          const star = starred.includes(option.framing.layout)
          return (
            <li key={option.framing.layout} className="relative flex">
              <TileButton
                selected={on}
                aria-pressed={on}
                disabled={disabled}
                title={name}
                onClick={() => onChange(option.framing)}
                className={cn('w-full gap-1 p-1 text-2xs', on && 'text-accent')}
              >
                <SchemeView scheme={option.scheme} label={name} decorative />
                <span className="truncate">{name}</span>
              </TileButton>
              {/* Beside the tile rather than in it: a button cannot hold one. */}
              <Button
                variant="icon"
                size="icon-xs"
                aria-pressed={star}
                aria-label={t('cover.framing.star', { name })}
                title={t('cover.framing.star', { name })}
                onClick={() => setStarred(option.framing.layout, !star)}
                className="absolute top-0.5 right-0.5"
              >
                <Star aria-hidden className={star ? 'fill-current text-warn' : undefined} />
              </Button>
            </li>
          )
        })}
      </ul>

      {framing !== null && (
        <div className="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
          <span className="caption">{t('cover.framing.where')}</span>
          {/* `h-auto`: a segmented control stands on the control row's
              height, and a grid or a wrapped row of segments ran over the
              row below. */}
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

          {/* Two scales of stops rather than rows of words (v0.135): how
              big the subject is, and how much of it shows - the second
              starting at no subject at all. */}
          <span className="caption">{t('cover.framing.size')}</span>
          <DiscreteScalePicker<CoverSize>
            label={t('cover.framing.size')}
            value={framing.size}
            disabled={disabled}
            onChange={(size) => set({ size })}
            steps={SIZES.map((size) => ({ value: size, label: t(`cover.size.${size}`) }))}
          />

          <span className="caption">{t('cover.framing.crop')}</span>
          <DiscreteScalePicker<Shot>
            label={t('cover.framing.crop')}
            value={subject === 'none' && onSubject !== undefined ? 'none' : framing.crop}
            disabled={disabled}
            onChange={(shot) => {
              if (shot === 'none') {
                onSubject?.('none')
                return
              }
              if (subject === 'none') onSubject?.('object')
              set({ crop: shot })
            }}
            steps={[
              ...(onSubject === undefined
                ? []
                : [{ value: 'none' as const, label: t('cover.framing.noSubject') }]),
              ...CROPS.map((crop) => ({ value: crop, label: t(`cover.crop.${crop}`) })),
            ]}
          />

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
