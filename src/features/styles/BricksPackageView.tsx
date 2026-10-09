import { useTranslation } from 'react-i18next'
import type { BricksPackage, ProposedBrick } from '@/lib/api/types'
import { sayReason } from '@/lib/errors'
import { StyleIcon } from '@/lib/styleIcon'
import { say as sayLabel, styleTypesOf, useProfile } from '@/lib/useProfile'
import { Checkbox } from '@/components/ui/checkbox'

interface Props {
  pack: BricksPackage
  /** The items kept - `brick:0`, `brick:2` - by the key an apply names them with. */
  chosen: readonly string[]
  onChosen?: (items: string[]) => void
  /** Answered already: the bricks are read, not chosen. */
  answered?: boolean
}

/** The item keys of a package, as an apply names them. */
export function brickItems(pack: BricksPackage): string[] {
  return pack.bricks.map((_, index) => `brick:${String(index)}`)
}

/**
 * Bricks proposed for the dictionary (v0.94), one item each: the phrase as
 * the generator reads it, its type, and what it means in the window's
 * language - with a box to leave it out, as a package of words is read: one
 * wrong guess must not cost the right ones.
 */
export function BricksPackageView({ pack, chosen, onChosen, answered = false }: Props) {
  const { t } = useTranslation()
  const toggle = (item: string, on: boolean) =>
    onChosen?.(on ? [...chosen, item] : chosen.filter((one) => one !== item))

  return (
    <div className="flex flex-col gap-2 text-xs">
      <ul className="flex flex-col gap-1.5">
        {pack.bricks.map((brick, index) => {
          const item = `brick:${String(index)}`
          return (
            <li key={item} className="flex items-start gap-2">
              {!answered && onChosen !== undefined && (
                <Checkbox
                  checked={chosen.includes(item)}
                  onCheckedChange={(on) => toggle(item, on)}
                  aria-label={t('phrases.keepItem', { phrase: brick.phrase })}
                  className="mt-0.5"
                />
              )}
              <Brick brick={brick} />
            </li>
          )
        })}
      </ul>
      {(pack.dropped ?? []).length > 0 && !answered && (
        <section className="flex flex-col gap-0.5 text-warn">
          <b className="text-2xs tracking-caption uppercase">{t('words.leftOut')}</b>
          {(pack.dropped ?? []).map((reason, index) => (
            <span key={String(index)}>{sayReason(reason)}</span>
          ))}
        </section>
      )}
    </div>
  )
}

function Brick({ brick }: { brick: ProposedBrick }) {
  const types = styleTypesOf(useProfile().config)
  const type = types.find((one) => one.key === brick.type)
  return (
    <span className="flex min-w-0 flex-1 flex-col gap-0.5">
      <span className="flex flex-wrap items-baseline gap-x-2">
        <b className="font-mono text-sm font-semibold text-text">{brick.phrase}</b>
        <span className="inline-flex items-center gap-1 text-faint">
          <StyleIcon of={type} aria-hidden className="size-3" />
          {type === undefined ? brick.type : sayLabel(type.label)}
        </span>
      </span>
      <span className="text-dim">{sayLabel(brick.explanation)}</span>
    </span>
  )
}
