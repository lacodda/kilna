import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Star, X } from 'lucide-react'
import type { StyleBrick, StyleForm, StyleType } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { byOffer, isOffered, styleName } from '@/lib/styleBrick'
import { say as sayLabel, styleTypesOf, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { Popover, PopoverPopup, PopoverTrigger } from '@/components/ui/popover'
import { BrickFace } from '@/features/styles/StyleBrickCard'

interface Props {
  /** What the place is called: "Style of the picture", "Typography". */
  label: string
  /** What a brick in this place is made of. */
  form: StyleForm
  /** The one type the place takes - a layer takes the type of its own name
   *  (v0.135) - rather than every type of the form. */
  typeKey?: string
  brickId: string | null
  /** The channel's house styles, offered first. */
  houseStyles: readonly string[]
  disabled?: boolean
  onChange: (brickId: string | null) => void
}

/**
 * One place of the cover the style dictionary fills: the brick in it, and a
 * list to change it from.
 *
 * The place asks for a form, not a type (ADR 0048): the application knows
 * that the picture's style is made of pictures and the title of a lettering,
 * never which of the craft's words those are. So the list is every ready
 * brick of every type of that form - a character is not offered, its cards
 * stand in for it - with the channel's house styles first.
 */
export function BrickPick({
  label,
  form,
  typeKey,
  brickId,
  houseStyles,
  disabled,
  onChange,
}: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const [open, setOpen] = useState(false)
  const [text, setText] = useState('')
  const all = useQuery(queries.styleBricksMatching(null, ''))

  const types = useMemo(
    () =>
      styleTypesOf(config).filter(
        (type) =>
          (typeKey === undefined ? (type.form ?? 'picture') === form : type.key === typeKey) &&
          (type.retired === undefined || type.retired === null) &&
          (type.canon_kind === undefined || type.canon_kind === null),
      ),
    [config, form, typeKey],
  )
  const typeOf = (brick: StyleBrick) => types.find((type) => type.key === brick.type_key)
  const chosen = (all.data ?? []).find((brick) => brick.id === brickId)
  const offered = (all.data ?? [])
    .filter((brick) => isOffered(types, brick) && typeOf(brick) !== undefined)
    .filter((brick) => {
      const needle = text.trim().toLowerCase()
      return (
        needle === '' ||
        `${styleName(brick)} ${brick.name} ${brick.description ?? ''}`
          .toLowerCase()
          .includes(needle)
      )
    })
    .sort(byOffer(new Set(houseStyles)))

  // The list opens on the brick that is in the place, wherever it stands.
  const list = useRef<HTMLUListElement>(null)
  useEffect(() => {
    if (!open) return
    const frame = window.requestAnimationFrame(() =>
      list.current?.querySelector('[aria-pressed="true"]')?.scrollIntoView({ block: 'nearest' }),
    )
    return () => window.cancelAnimationFrame(frame)
  }, [open])
  // Filed under their families where the type has them: one heading each.
  const familyOf = (brick: StyleBrick) =>
    typeOf(brick)?.families?.find((one) => one.key === brick.family)
  const grouped = typeKey !== undefined && offered.some((brick) => familyOf(brick) !== undefined)
  const rows = grouped
    ? [...offered].sort((a, b) =>
        (familyOf(a)?.key ?? '\uffff').localeCompare(familyOf(b)?.key ?? '\uffff'),
      )
    : offered

  return (
    <div className="flex flex-col gap-2">
      <div className="flex min-w-0 items-center gap-2.5 rounded-lg border border-line bg-raise p-1.5">
        {chosen === undefined ? (
          <span className="flex size-10 shrink-0 items-center justify-center rounded-md border border-dashed border-line-2 text-xs text-faint">
            —
          </span>
        ) : (
          <BrickFace
            brick={chosen}
            type={
              typeOf(chosen) ?? styleTypesOf(config).find((type) => type.key === chosen.type_key)
            }
            className="size-10 shrink-0 overflow-hidden rounded-md px-0 text-sm"
          />
        )}
        <span className="flex min-w-0 flex-1 flex-col">
          <b className="truncate text-sm font-semibold">
            {chosen === undefined
              ? brickId === null
                ? t('cover.brick.none')
                : t('cover.brick.gone')
              : styleName(chosen)}
          </b>
          {chosen !== undefined && (
            <span className="truncate text-xs text-faint">
              {chosen.when_to_use ?? chosen.description ?? ''}
            </span>
          )}
        </span>
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger
            render={<Button size="xs" disabled={disabled} />}
            aria-label={t('cover.brick.change', { place: label })}
          >
            {brickId === null ? t('cover.brick.choose') : t('cover.brick.swap')}
          </PopoverTrigger>
          <PopoverPopup arrow={false} align="end" className="flex w-96 flex-col gap-2 p-2">
            <Input
              value={text}
              autoFocus
              placeholder={t('cover.brick.search')}
              aria-label={t('cover.brick.search')}
              onChange={(event) => setText(event.target.value)}
            />
            <ul
              ref={list}
              aria-label={label}
              className="flex max-h-80 flex-col gap-1 overflow-y-auto"
            >
              {offered.length === 0 && (
                <li className="px-2 py-3 text-sm text-faint">{t('cover.brick.nothing')}</li>
              )}
              {rows.map((brick, index) => {
                const family = familyOf(brick)
                const before = index === 0 ? undefined : rows[index - 1]
                const heading =
                  grouped &&
                  family !== undefined &&
                  (before === undefined || familyOf(before)?.key !== family.key)
                return (
                  <li key={brick.id}>
                    {heading && (
                      <span className="caption mt-1 block px-1.5">{sayLabel(family.label)}</span>
                    )}
                    <Choice
                      brick={brick}
                      type={typeOf(brick)}
                      house={houseStyles.includes(brick.id)}
                      on={brick.id === brickId}
                      onPick={() => {
                        onChange(brick.id)
                        setOpen(false)
                      }}
                    />
                  </li>
                )
              })}
            </ul>
          </PopoverPopup>
        </Popover>
        {brickId !== null && (
          <Button
            variant="icon"
            size="icon-sm"
            disabled={disabled}
            aria-label={t('cover.brick.clear', { place: label })}
            title={t('cover.brick.clear', { place: label })}
            onClick={() => onChange(null)}
          >
            <X />
          </Button>
        )}
      </div>
    </div>
  )
}

function Choice({
  brick,
  type,
  house,
  on,
  onPick,
}: {
  brick: StyleBrick
  type: StyleType | undefined
  house: boolean
  on: boolean
  onPick: () => void
}) {
  const { t } = useTranslation()
  const family = type?.families?.find((one) => one.key === brick.family)
  return (
    <Button
      variant="icon"
      size={null}
      aria-pressed={on}
      onClick={onPick}
      className={cn(
        'w-full items-center justify-start gap-2.5 p-1.5 text-left font-normal whitespace-normal',
        on && 'bg-accent-soft',
      )}
    >
      <BrickFace
        brick={brick}
        type={type}
        className="size-10 shrink-0 overflow-hidden rounded-md px-0 text-sm"
      />
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="flex min-w-0 items-center gap-1.5">
          <b className="truncate text-sm font-semibold text-text">{styleName(brick)}</b>
          {house && <Chip variant="accent">{t('cover.brick.house')}</Chip>}
          {brick.favorite === true && (
            <Star aria-label={t('styles.favorite')} className="size-3.5 shrink-0 text-warn" />
          )}
          {family !== undefined && <Chip>{sayLabel(family.label)}</Chip>}
        </span>
        <span className="line-clamp-2 text-xs text-dim">
          {brick.when_to_use ?? brick.description ?? ''}
        </span>
      </span>
    </Button>
  )
}
