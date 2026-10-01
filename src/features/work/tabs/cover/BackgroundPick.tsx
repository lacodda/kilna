import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { queries } from '@/lib/query/queries'
import { styleName } from '@/lib/styleBrick'
import { styleTypesOf, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

interface Props {
  brickId: string | null
  disabled?: boolean
  onChange: (brickId: string | null) => void
}

/**
 * The ground of the picture: a row of the dictionary's colour bricks, each
 * drawn as its colour.
 *
 * A row of swatches rather than a list, the mockup's way: a ground is picked
 * by the eye against the rest of the cover, and its name is the tooltip.
 * The swatch is the brick's own colour - the subject here, not the styling.
 */
export function BackgroundPick({ brickId, disabled, onChange }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const all = useQuery(queries.styleBricksMatching(null, ''))
  const colourTypes = new Set(
    styleTypesOf(config)
      .filter((type) => type.form === 'colour')
      .map((type) => type.key),
  )
  const grounds = (all.data ?? []).filter(
    (brick) =>
      colourTypes.has(brick.type_key) && brick.status === 'ready' && brick.colours[0] !== undefined,
  )
  const chosen = grounds.find((brick) => brick.id === brickId)

  return (
    <div className="flex flex-col gap-1.5">
      <ul aria-label={t('cover.ground.title')} className="flex flex-wrap gap-1.5">
        {grounds.map((brick) => {
          const on = brick.id === brickId
          const name = `${styleName(brick)} · ${brick.colours[0]}`
          return (
            <li key={brick.id}>
              <Button
                variant="icon"
                size={null}
                aria-pressed={on}
                aria-label={name}
                title={name}
                disabled={disabled}
                onClick={() => onChange(on ? null : brick.id)}
                className={cn(
                  'size-6 rounded-md ring-1 ring-line-2',
                  on && 'ring-2 ring-accent ring-offset-2 ring-offset-raise',
                )}
                style={{ background: brick.colours[0] }}
              />
            </li>
          )
        })}
      </ul>
      <span className="font-mono text-xs text-faint">
        {chosen === undefined
          ? brickId === null
            ? t('cover.ground.none')
            : t('cover.brick.gone')
          : `${styleName(chosen)} · ${chosen.colours[0]}`}
      </span>
    </div>
  )
}
