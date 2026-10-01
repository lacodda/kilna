import { useTranslation } from 'react-i18next'
import type { CoverMark, CoverView } from '@/lib/api/types'
import { fileSrc } from '@/lib/api/assets'
import { say as sayLabel } from '@/lib/useProfile'
import { Panel } from '@/components/ui/panel'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SchemeView } from '@/features/cover/SchemeView'

interface Props {
  view: CoverView
  mark: CoverMark
  onFormat: (format: string) => void
}

/**
 * The stage: the cover's shape, picked among its doors', and the built
 * frame drawn in it - with the channel's mark in its corner when it is laid
 * over at export, so the corner is chosen against the picture.
 *
 * A shape whose door the work does not go out through yet is offered all the
 * same and said to be so: the square of a streaming cover is worth seeing
 * before the release that needs it.
 */
export function CoverStage({ view, mark, onFormat }: Props) {
  const { t } = useTranslation()
  const laid = mark.place === 'corner' && mark.way === 'overlay'
  const file = view.marks.find((option) => option.id === mark.variant)?.file ?? null
  // Held when any door of this shape is: the shape is what the cover owes.
  const held =
    view.formats.length === 0 ||
    view.formats.some((format) => format.format === view.format && format.held)

  return (
    <Panel className="flex flex-col gap-2 p-2.5">
      <div className="flex flex-wrap items-center gap-2">
        {view.formats.length > 0 && (
          <SegmentedControl
            aria-label={t('cover.format')}
            value={view.format}
            onValueChange={onFormat}
          >
            {/* One segment per shape: two doors of one shape are one cover. */}
            {shapesOf(view.formats).map((format) => (
              <Segment key={format.format} value={format.format} className="text-xs">
                {`${format.format} · ${sayLabel(format.label)}`}
              </Segment>
            ))}
          </SegmentedControl>
        )}
        <span className="text-2xs text-faint">
          {held ? t('cover.formatHint') : t('cover.formatNotHeld')}
        </span>
      </div>
      <div className="grid h-[clamp(10rem,32vh,20rem)] place-items-center rounded-md bg-soft p-2.5">
        {view.scheme === null ? (
          <p className="max-w-80 text-center text-sm text-faint">{t('cover.noScheme')}</p>
        ) : (
          <SchemeView
            scheme={view.scheme}
            label={t('cover.schemeLabel')}
            className="h-full max-w-full"
            mark={
              laid && file !== null ? (
                <img
                  src={fileSrc(file.path)}
                  alt=""
                  className="size-full object-contain drop-shadow"
                  draggable={false}
                />
              ) : (
                // Where it goes, as a box: drawn there by the generator, or
                // a variant with no file yet to lay over.
                <span className="block size-full rounded-sm border border-dashed border-accent" />
              )
            }
          />
        )}
      </div>
    </Panel>
  )
}

/**
 * One entry per shape, named by its first door - or by a door the work goes
 * out through, when the first is one it does not: a clip's YouTube and its
 * premiere are one 16:9 cover.
 */
function shapesOf(formats: CoverView['formats']): CoverView['formats'] {
  const out: CoverView['formats'] = []
  for (const format of formats) {
    const at = out.findIndex((one) => one.format === format.format)
    if (at === -1) out.push(format)
    else if (!out[at]!.held && format.held) out[at] = format
  }
  return out
}
