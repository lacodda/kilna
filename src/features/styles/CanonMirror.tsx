import { useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import type { StyleType } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { say as sayLabel } from '@/lib/useProfile'
import { Badge } from '@/components/ui/badge'
import { TileButton } from '@/components/ui/tile-button'
import { CardAvatar } from '@/features/canon/CardAvatar'

/**
 * The heroes of the canon, shown where the type they stand in for is
 * (v0.133, FR-05 5.3): a mirror of the canon's cards of the type's
 * `canon_kind`, with their portraits and layers, each opening its card. The
 * canon is the one place a hero is written - nothing is copied into the
 * dictionary, so there is nothing here to fall out of step.
 */
export function CanonMirror({ type }: { type: StyleType }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const kind = type.canon_kind ?? null
  const cards = useQuery({ ...queries.cardsMatching({ kind }), enabled: kind !== null })
  const heroes = (cards.data ?? []).filter((card) => card.kind === kind)
  if (kind === null || heroes.length === 0) return null

  return (
    <section aria-label={t('styles.heroesOfCanon')} className="flex flex-col gap-1.5">
      <h2 className="caption">{t('styles.heroesOfCanon')}</h2>
      <ul className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-2">
        {heroes.map((card) => (
          <li key={card.id} className="flex">
            <TileButton
              onClick={() => void navigate(`/canon/${card.id}`)}
              className="w-full flex-row items-center gap-2.5 rounded-lg bg-raise p-2"
            >
              <CardAvatar title={card.title} portrait={card.portrait} size="lg" />
              <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="truncate text-sm font-semibold text-text">
                  {card.title ?? t('cover.hero.untitled')}
                </span>
                <span className="truncate text-xs text-faint">
                  {card.aliases.length > 0 ? card.aliases.join(', ') : sayLabel(type.label)}
                </span>
              </span>
              <Badge
                variant={card.layer === 'public' ? 'good' : 'soft'}
                title={t(`canon.layer.long.${card.layer}`)}
              >
                {t(`canon.layer.short.${card.layer}`)}
              </Badge>
            </TileButton>
          </li>
        ))}
      </ul>
    </section>
  )
}
