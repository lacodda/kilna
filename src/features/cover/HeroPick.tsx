import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { AlertTriangle } from 'lucide-react'
import type { CardSummary, CoverHero, CoverHeroState } from '@/lib/api/types'
import { cardKindOf } from '@/lib/canon'
import { queries } from '@/lib/query/queries'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Popover, PopoverPopup, PopoverTrigger } from '@/components/ui/popover'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { CardAvatar } from '@/features/canon/CardAvatar'

interface Props {
  hero: CoverHero | null
  /** The chosen card as the backend read it: its description, its pictures. */
  state: CoverHeroState | null
  disabled?: boolean
  onChange: (hero: CoverHero | null) => void
}

/**
 * Who the picture is of: the hero the scene describes, or a card of the
 * canon - described for a generator from its facts, with its pictures handed
 * over as references.
 *
 * Only a public card is offered: a cover reads the public layer of the canon
 * and nothing else (`canon::seen_by`), and a hero the cover cannot read would
 * be named on the picture and described nowhere. The root card - the channel
 * itself - is no hero either.
 */
export function HeroPick({ hero, state, disabled, onChange }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const [open, setOpen] = useState(false)
  const [text, setText] = useState('')
  const cards = useQuery({ ...queries.cardsMatching({}), enabled: open || hero !== null })

  const needle = text.trim().toLowerCase()
  const offered = (cards.data ?? []).filter(
    (card: CardSummary) =>
      card.layer === 'public' &&
      cardKindOf(config, card.kind)?.root !== true &&
      (needle === '' ||
        `${card.title ?? ''} ${card.aliases.join(' ')}`.toLowerCase().includes(needle)),
  )
  const chosen = (cards.data ?? []).find((card) => card.id === hero?.card)

  return (
    <div className="flex flex-col gap-2">
      <SegmentedControl
        aria-label={t('cover.hero.from')}
        value={hero === null ? 'scene' : 'card'}
        disabled={disabled}
        onValueChange={(next) => {
          if (next === 'scene') onChange(null)
          else setOpen(true)
        }}
      >
        <Segment value="scene">{t('cover.hero.fromScene')}</Segment>
        <Segment value="card">{t('cover.hero.fromCanon')}</Segment>
      </SegmentedControl>

      {hero === null ? (
        <span className="text-xs text-faint">{t('cover.hero.sceneHint')}</span>
      ) : (
        <div className="flex min-w-0 items-center gap-2.5 rounded-lg border border-line bg-raise p-1.5">
          <CardAvatar title={chosen?.title ?? state?.title} portrait={chosen?.portrait ?? null} />
          <span className="flex min-w-0 flex-1 flex-col">
            <b className="truncate text-sm font-semibold">
              {state?.title ?? chosen?.title ?? t('cover.hero.gone')}
            </b>
            {chosen !== undefined && (
              <span className="truncate text-xs text-faint">
                {sayLabel(cardKindOf(config, chosen.kind)?.label ?? chosen.kind)}
                {chosen.work_title !== null ? ` · ${chosen.work_title}` : ''}
              </span>
            )}
          </span>
        </div>
      )}

      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger render={<Button size="xs" className="self-start" disabled={disabled} />}>
          {hero === null ? t('cover.hero.pick') : t('cover.hero.swap')}
        </PopoverTrigger>
        <PopoverPopup arrow={false} align="start" className="flex w-80 flex-col gap-2 p-2">
          <Input
            value={text}
            autoFocus
            placeholder={t('cover.hero.search')}
            aria-label={t('cover.hero.search')}
            onChange={(event) => setText(event.target.value)}
          />
          <ul
            aria-label={t('cover.hero.cards')}
            className="flex max-h-80 flex-col gap-0.5 overflow-y-auto"
          >
            {offered.length === 0 && (
              <li className="px-2 py-3 text-sm text-faint">{t('cover.hero.nothing')}</li>
            )}
            {offered.map((card) => (
              <li key={card.id}>
                <Button
                  variant="icon"
                  size={null}
                  aria-pressed={card.id === hero?.card}
                  onClick={() => {
                    onChange({ card: card.id, references: hero?.references ?? true })
                    setOpen(false)
                  }}
                  className={cn(
                    'w-full justify-start gap-2 px-1.5 py-1 text-left font-normal',
                    card.id === hero?.card && 'bg-accent-soft',
                  )}
                >
                  <CardAvatar title={card.title} portrait={card.portrait} size="sm" />
                  <span className="min-w-0 flex-1 truncate text-sm text-text">
                    {card.title ?? t('cover.hero.untitled')}
                  </span>
                  <span className="shrink-0 text-2xs text-faint">
                    {sayLabel(cardKindOf(config, card.kind)?.label ?? card.kind)}
                  </span>
                </Button>
              </li>
            ))}
          </ul>
        </PopoverPopup>
      </Popover>

      {hero !== null && state !== null && (
        <>
          {state.description === null ? (
            <p className="flex items-start gap-1.5 text-xs text-warn">
              <AlertTriangle aria-hidden className="mt-0.5 size-3.5 shrink-0" />
              {t('cover.hero.undescribed')}
            </p>
          ) : (
            <p className="rounded-md border border-line bg-soft px-2.5 py-2 font-mono text-xs leading-relaxed text-dim">
              {state.description}
            </p>
          )}
          <Checkbox
            checked={hero.references && state.pictures.length > 0}
            disabled={disabled || state.pictures.length === 0}
            onCheckedChange={(references) => onChange({ ...hero, references })}
          >
            {state.pictures.length === 0
              ? t('cover.hero.noPictures')
              : t('cover.hero.references', { count: state.pictures.length })}
          </Checkbox>
        </>
      )}
    </div>
  )
}
