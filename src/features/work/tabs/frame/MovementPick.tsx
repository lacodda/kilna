import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Plus, X } from 'lucide-react'
import type { StyleBrick } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { byOffer, isOffered, styleName } from '@/lib/styleBrick'
import { say as sayLabel, styleTypesOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { Popover, PopoverPopup, PopoverTrigger } from '@/components/ui/popover'

/** The most movements one loop is built from - the backend's `MOST_MOVEMENTS`. */
const MOST_MOVEMENTS = 5

interface Props {
  /** The movements chosen, by brick id, in their order. */
  chosen: readonly string[]
  /** The camera holds still: a movement that moves it is marked as one the
   *  loop will leave out. */
  stillCamera: boolean
  onChange: (chosen: string[]) => void
}

/**
 * The movements of a loop from the dictionary's Motion branch (v0.136, FR-07
 * 7.3): up to five, in the order the loop says them, each taken out with its
 * cross. The list offers the ready movements of sets that are on, the
 * starred first, by type; one that moves the camera says so while the
 * camera holds still - its contract keeps it out of the loop, and the frame
 * says why with the one change that lets it in.
 */
export function MovementPick({ chosen, stillCamera, onChange }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const [open, setOpen] = useState(false)
  const [text, setText] = useState('')
  const all = useQuery(queries.styleBricksMatching(null, ''))
  const types = styleTypesOf(config).filter((type) => type.form === 'motion')
  const typeOf = (brick: StyleBrick) => types.find((type) => type.key === brick.type_key)
  const byId = new Map((all.data ?? []).map((brick) => [brick.id, brick]))
  const needle = text.trim().toLowerCase()
  const offered = (all.data ?? [])
    .filter(
      (brick) =>
        typeOf(brick) !== undefined &&
        isOffered(types, brick) &&
        !chosen.includes(brick.id) &&
        (needle === '' ||
          `${styleName(brick)} ${brick.description ?? ''}`.toLowerCase().includes(needle)),
    )
    .sort(byOffer(new Set()))
  const full = chosen.length >= MOST_MOVEMENTS
  const movesCamera = (brick: StyleBrick) => brick.contract?.provides?.includes('camera:moving')

  return (
    <div className="flex flex-col gap-1.5">
      <ul aria-label={t('frame.movements')} className="flex flex-wrap gap-1.5">
        {chosen.map((id) => {
          const brick = byId.get(id)
          const name = brick === undefined ? t('cover.brick.gone') : styleName(brick)
          return (
            <li key={id}>
              <Chip
                variant={
                  brick !== undefined && stillCamera && movesCamera(brick) ? 'dashed' : undefined
                }
              >
                {name}
                <Button
                  variant="icon"
                  size="icon-xs"
                  aria-label={t('frame.removeMovement', { name })}
                  onClick={() => onChange(chosen.filter((one) => one !== id))}
                >
                  <X aria-hidden />
                </Button>
              </Chip>
            </li>
          )
        })}
      </ul>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger
          render={<Button size="xs" className="self-start" disabled={full || types.length === 0} />}
        >
          <Plus aria-hidden />
          {full ? t('frame.movementsFull', { most: MOST_MOVEMENTS }) : t('frame.addMovement')}
        </PopoverTrigger>
        <PopoverPopup arrow={false} align="start" className="flex w-96 flex-col gap-2 p-2">
          <Input
            value={text}
            autoFocus
            placeholder={t('frame.findMovement')}
            aria-label={t('frame.findMovement')}
            onChange={(event) => setText(event.target.value)}
          />
          <ul
            aria-label={t('frame.movements')}
            className="flex max-h-80 flex-col gap-0.5 overflow-y-auto"
          >
            {offered.length === 0 && (
              <li className="px-2 py-3 text-sm text-faint">{t('cover.brick.nothing')}</li>
            )}
            {offered.map((brick) => {
              const type = typeOf(brick)
              const away = stillCamera && movesCamera(brick)
              return (
                <li key={brick.id}>
                  <Button
                    variant="icon"
                    size={null}
                    onClick={() => {
                      onChange([...chosen, brick.id])
                      setOpen(false)
                    }}
                    className="w-full flex-col items-start gap-0.5 px-2 py-1.5 text-left font-normal whitespace-normal"
                  >
                    <span className="flex w-full items-baseline gap-2">
                      <b className="min-w-0 flex-1 truncate text-sm font-semibold text-text">
                        {styleName(brick)}
                      </b>
                      <span className="shrink-0 text-2xs text-faint">
                        {type === undefined ? '' : sayLabel(type.label)}
                      </span>
                    </span>
                    <span className="line-clamp-2 text-xs text-dim">
                      {(brick.description ?? '').replace(/^MOTION:\s*/, '')}
                    </span>
                    {away && <span className="text-2xs text-warn">{t('frame.movesCamera')}</span>}
                  </Button>
                </li>
              )
            })}
          </ul>
        </PopoverPopup>
      </Popover>
    </div>
  )
}
