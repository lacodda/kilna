import { useTranslation } from 'react-i18next'
import type { UseQueryResult } from '@tanstack/react-query'
import type { CardSummary, FactStatus } from '@/lib/api/types'
import { groupCards } from '@/lib/canon'
import { canonIconOf } from '@/lib/canonIcon'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { SkeletonList } from '@/components/ui/skeleton'
import { Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { CardAvatar } from '@/features/canon/CardAvatar'

/** The status chips over the list: every card, or those holding facts in a state. */
const FILTERS = ['all', 'canon', 'draft', 'open'] as const
type Filter = (typeof FILTERS)[number]

interface Props {
  query: UseQueryResult<CardSummary[]>
  /** How many cards there are, whatever the filters. */
  total: number
  text: string
  onText: (text: string) => void
  status: FactStatus | undefined
  onStatus: (status: FactStatus | undefined) => void
  openId: string | null
  onOpen: (id: string) => void
}

/**
 * The cards of the canon, grouped by kind: the channel first, then the kinds
 * in the profile's order, and last the heroes who live at one work. A search
 * over names, aliases and facts, and chips for the cards that hold drafts to
 * settle or live zones to build on.
 */
export function CardList({ query, total, text, onText, status, onStatus, openId, onOpen }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const filter: Filter = status === undefined ? 'all' : (status as Filter)
  const filtered = status !== undefined || text.trim() !== ''

  return (
    <Pane
      label={t('nav.canon')}
      bodyClassName="p-1.5"
      head={
        <div className="flex w-full flex-col gap-2">
          <Input
            value={text}
            onChange={(event) => onText(event.target.value)}
            placeholder={t('canon.search')}
            aria-label={t('canon.search')}
          />
          <ChipGroup
            aria-label={t('canon.filter')}
            value={[filter]}
            onValueChange={(next) => {
              const picked = (next[0] ?? 'all') as Filter
              onStatus(picked === 'all' ? undefined : picked)
            }}
          >
            {FILTERS.map((one) => (
              <Chip key={one} value={one} count={one === 'all' ? total : undefined}>
                {t(`canon.filterName.${one}`)}
              </Chip>
            ))}
          </ChipGroup>
        </div>
      }
    >
      <Loaded
        query={query}
        skeleton={<SkeletonList rows={6} />}
        isEmpty={(data) => data.length === 0}
        emptyState={
          <EmptyState
            plain
            variant={filtered ? 'filtered' : 'empty'}
            title={filtered ? t('canon.noMatches') : t('canon.none')}
            className="p-2"
          />
        }
        plain
      >
        {(cards) => (
          <div className="flex flex-col">
            {groupCards(cards, config).map((group) => {
              const Icon = canonIconOf(group.kind)
              const caption =
                group.kind !== undefined
                  ? sayLabel(group.kind.label)
                  : t(group.key === 'heroes' ? 'canon.heroes' : 'canon.otherKinds')
              return (
                <section key={group.key} aria-label={caption}>
                  <h3 className="flex items-center gap-1.5 px-2 pt-2.5 pb-1 text-2xs font-semibold tracking-caption text-faint uppercase">
                    <Icon aria-hidden className="size-3" />
                    {caption}
                    <span className="ml-auto font-mono">{group.cards.length}</span>
                  </h3>
                  <ul className="flex flex-col gap-0.5">
                    {group.cards.map((card) => (
                      <li key={card.id}>
                        <RowButton
                          selected={card.id === openId}
                          onClick={() => onOpen(card.id)}
                          className="gap-2 px-2 py-1"
                          start={
                            <CardAvatar
                              title={card.title}
                              portrait={card.portrait ?? null}
                              size="sm"
                            />
                          }
                          end={
                            <>
                              {card.layer !== 'public' && (
                                <span
                                  className="font-mono text-2xs"
                                  title={t(`canon.layer.long.${card.layer}`)}
                                >
                                  {t(`canon.layer.short.${card.layer}`)}
                                </span>
                              )}
                              {card.work_title !== null && (
                                <span className="font-mono text-2xs" title={card.work_title}>
                                  {t('canon.ofWork')}
                                </span>
                              )}
                              {card.drafts > 0 && (
                                <span
                                  className="rounded-sm bg-info-soft px-1 font-mono text-2xs text-info"
                                  title={t('canon.draftsWaiting', { count: card.drafts })}
                                >
                                  {card.drafts}
                                </span>
                              )}
                            </>
                          }
                        >
                          {card.title ?? t('canon.untitled')}
                        </RowButton>
                      </li>
                    ))}
                  </ul>
                </section>
              )
            })}
          </div>
        )}
      </Loaded>
    </Pane>
  )
}
