import { useTranslation } from 'react-i18next'
import type {
  CoverCorner,
  CoverMark,
  CoverMarkOption,
  CoverMarkPlace,
  CoverMarkWay,
} from '@/lib/api/types'
import { fileSrc } from '@/lib/api/assets'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'

const CORNERS: readonly CoverCorner[] = ['topLeft', 'topRight', 'bottomLeft', 'bottomRight']

interface Props {
  mark: CoverMark
  /** The variants of the channel's mark, from its card. */
  options: CoverMarkOption[]
  disabled?: boolean
  onChange: (mark: CoverMark) => void
}

/**
 * The channel's mark on the cover: which variant of the family - the one
 * whose meaning fits the song - where it goes, and how it gets there.
 *
 * Laid over by default: a generator distorts a logo, and the mark has to be
 * exact, so its own file is put over the finished picture when it is
 * exported. Drawn, its description goes into the prompt and its file to the
 * generator as a reference. Hidden in the picture it can only be drawn.
 */
export function MarkPick({ mark, options, disabled, onChange }: Props) {
  const { t } = useTranslation()
  if (options.length === 0) {
    return <p className="text-xs text-faint">{t('cover.mark.none')}</p>
  }
  const chosen = options.find((option) => option.id === mark.variant)
  const set = (patch: Partial<CoverMark>) => onChange({ ...mark, ...patch })
  const hidden = mark.place === 'hidden'

  return (
    <div className="flex flex-col gap-2.5">
      <ul aria-label={t('cover.mark.variants')} className="grid grid-cols-6 gap-1.5">
        {options.map((option) => {
          const on = option.id === mark.variant
          return (
            <li key={option.id} className="flex">
              <Button
                variant="ghost"
                size={null}
                aria-pressed={on}
                title={option.name}
                disabled={disabled}
                onClick={() => set({ variant: on ? null : option.id })}
                className={cn(
                  'w-full flex-col gap-0.5 p-1 font-normal',
                  on && 'border-accent text-accent',
                )}
              >
                {option.file === null ? (
                  <span className="grid aspect-square w-full place-items-center rounded-sm bg-soft text-xs font-semibold">
                    {option.code ?? '·'}
                  </span>
                ) : (
                  <img
                    src={fileSrc(option.file.path)}
                    alt=""
                    className="aspect-square w-full rounded-sm object-contain"
                    draggable={false}
                  />
                )}
                <span className="w-full truncate text-center font-mono text-2xs">
                  {option.code ?? option.name}
                </span>
              </Button>
            </li>
          )
        })}
      </ul>
      {chosen !== undefined && <p className="text-xs text-dim">{chosen.name}</p>}

      <div className="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
        <span className="caption">{t('cover.mark.where')}</span>
        <SegmentedControl
          aria-label={t('cover.mark.where')}
          value={mark.place}
          disabled={disabled}
          onValueChange={(place) => set({ place: place as CoverMarkPlace })}
          className="h-auto flex-wrap"
        >
          {(['corner', 'hidden', 'none'] as const).map((place) => (
            <Segment key={place} value={place} className="px-2 py-0.5 text-xs">
              {t(`cover.mark.place.${place}`)}
            </Segment>
          ))}
        </SegmentedControl>

        {mark.place === 'corner' && (
          <>
            <span className="caption">{t('cover.mark.corner')}</span>
            <SegmentedControl
              aria-label={t('cover.mark.corner')}
              value={mark.corner}
              disabled={disabled}
              onValueChange={(corner) => set({ corner: corner as CoverCorner })}
            >
              {CORNERS.map((corner) => (
                <Segment
                  key={corner}
                  value={corner}
                  aria-label={t(`cover.corner.${corner}`)}
                  className="px-2"
                >
                  {t(`cover.cornerArrow.${corner}`)}
                </Segment>
              ))}
            </SegmentedControl>
          </>
        )}

        {mark.place !== 'none' && (
          <>
            <span className="caption">{t('cover.mark.how')}</span>
            <SegmentedControl
              aria-label={t('cover.mark.how')}
              value={hidden ? 'drawn' : mark.way}
              disabled={disabled}
              onValueChange={(way) => set({ way: way as CoverMarkWay })}
              className="h-auto flex-wrap"
            >
              <Segment value="overlay" disabled={hidden} className="px-2 py-0.5 text-xs">
                {t('cover.mark.way.overlay')}
              </Segment>
              <Segment value="drawn" className="px-2 py-0.5 text-xs">
                {t('cover.mark.way.drawn')}
              </Segment>
            </SegmentedControl>
          </>
        )}
      </div>
      {mark.place !== 'none' && (
        <p className="text-xs text-faint">
          {hidden || mark.way === 'drawn' ? t('cover.mark.drawnHint') : t('cover.mark.overlayHint')}
        </p>
      )}
    </div>
  )
}
