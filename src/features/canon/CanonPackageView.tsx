import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { CanonPackage, FactReview } from '@/lib/api/types'
import { sayReason } from '@/lib/errors'
import { queries } from '@/lib/query/queries'
import { Checkbox } from '@/components/ui/checkbox'
import { LayerMark } from '@/features/canon/SectionBlock'
import { cn } from '@/lib/utils'

interface Props {
  messageId: string
  pack: CanonPackage
  /** The items kept - `card:0`, `fact:2`, `relation:1` - or all when absent. */
  chosen: readonly string[]
  onChosen?: (items: string[]) => void
  /** Answered already: the items are read, not chosen. */
  answered: boolean
}

/** Every item of a package, by its key. */
export function itemsOf(pack: CanonPackage): string[] {
  return [
    ...pack.cards.map((_, index) => `card:${String(index)}`),
    ...pack.facts.map((_, index) => `fact:${String(index)}`),
    ...pack.links.map((_, index) => `relation:${String(index)}`),
  ]
}

/**
 * A proposal for the canon, item by item, read against the canon as it
 * stands now: each new card; each fact with the card and section it lands in,
 * the fact it refines or retires, the facts it contradicts side by side, and
 * a mark when the card already says the same; each relation. A box beside
 * each keeps it or leaves it out - one wrong fact must not cost the right
 * ones.
 */
export function CanonPackageView({ messageId, pack, chosen, onChosen, answered }: Props) {
  const { t } = useTranslation()
  const review = useQuery({ ...queries.canonReview(messageId), enabled: !answered })
  const reviews: FactReview[] = review.data ?? []
  const nameOf = (id: string) =>
    pack.cards.find((card) => card.handle === id)?.title ??
    reviews.find((_, index) => pack.facts[index]?.card === id)?.card_title ??
    id

  const toggle = (item: string, on: boolean) =>
    onChosen?.(on ? [...chosen, item] : chosen.filter((one) => one !== item))
  const box = (item: string) =>
    answered || onChosen === undefined ? null : (
      <Checkbox
        checked={chosen.includes(item)}
        onCheckedChange={(on) => toggle(item, on === true)}
        aria-label={t('canon.keepItem')}
        className="mt-0.5"
      />
    )

  return (
    <div className="flex flex-col gap-2 text-xs">
      {pack.cards.length > 0 && (
        <section className="flex flex-col gap-1">
          <b className="text-2xs tracking-caption text-faint uppercase">{t('canon.newCards')}</b>
          {pack.cards.map((card, index) => (
            <label key={card.handle} className="flex items-start gap-2">
              {box(`card:${String(index)}`)}
              <span>
                <b className="font-semibold text-text">{card.title}</b> · {card.kind}
                {card.work_id !== undefined && card.work_id !== null && (
                  <span className="text-faint"> · {t('canon.livesAtWork')}</span>
                )}
              </span>
            </label>
          ))}
        </section>
      )}

      {pack.facts.length > 0 && (
        <section className="flex flex-col gap-1.5">
          <b className="text-2xs tracking-caption text-faint uppercase">{t('canon.facts')}</b>
          {pack.facts.map((fact, index) => {
            const seen = reviews[index]
            return (
              <div key={`${String(index)}-${fact.body ?? ''}`} className="flex items-start gap-2">
                {box(`fact:${String(index)}`)}
                {fact.layer !== undefined && fact.layer !== null ? (
                  <LayerMark layer={fact.layer} />
                ) : (
                  <LayerMark layer="public" />
                )}
                <div className="min-w-0 flex-1">
                  <div className="text-faint">
                    {seen?.card_title ?? nameOf(fact.card ?? '')}
                    {seen?.section_label !== null && seen?.section_label !== undefined
                      ? ` · ${seen.section_label}`
                      : ''}
                    {fact.change !== 'add' && ` · ${t(`canon.change.${fact.change ?? 'add'}`)}`}
                  </div>
                  {fact.change === 'retire' ? (
                    <div>
                      <span className="text-faint line-through">{seen?.target?.body}</span>
                      <span className="text-dim"> — {fact.reason}</span>
                    </div>
                  ) : (
                    <div className="text-sm text-text">
                      {fact.change === 'refine' &&
                        seen?.target !== null &&
                        seen?.target !== undefined && (
                          <span className="mr-1 text-faint line-through">{seen.target.body}</span>
                        )}
                      {fact.body}
                    </div>
                  )}
                  {fact.source?.line !== undefined && fact.source.line !== null && (
                    <div className="text-faint">«{fact.source.line}»</div>
                  )}
                  {seen?.duplicate === true && (
                    <span className="rounded-sm bg-soft px-1 text-2xs text-faint">
                      {t('canon.alreadyInCanon')}
                    </span>
                  )}
                  {(fact.contradicts ?? []).map((clash) => {
                    const held = seen?.contradicted.find((one) => one.id === clash.fact_id)
                    return (
                      <div
                        key={clash.fact_id}
                        className="mt-0.5 rounded-md border border-warn/40 bg-warn-soft px-1.5 py-0.5 text-warn"
                      >
                        {t('canon.contradicts', { fact: held?.body ?? clash.fact_id })}
                        {clash.why !== '' && <span className="text-dim"> — {clash.why}</span>}
                      </div>
                    )
                  })}
                </div>
              </div>
            )
          })}
        </section>
      )}

      {pack.links.length > 0 && (
        <section className="flex flex-col gap-1">
          <b className="text-2xs tracking-caption text-faint uppercase">{t('canon.relations')}</b>
          {pack.links.map((link, index) => (
            <label key={`${link.from}-${link.to}`} className="flex items-start gap-2">
              {box(`relation:${String(index)}`)}
              <span>
                {nameOf(link.from)} — {nameOf(link.to)}
                {link.label !== undefined && link.label !== null && (
                  <span className="text-faint">: {link.label}</span>
                )}
              </span>
            </label>
          ))}
        </section>
      )}

      {pack.dropped.length > 0 && !answered && (
        <section className={cn('flex flex-col gap-0.5 text-warn')}>
          <b className="text-2xs tracking-caption uppercase">{t('canon.leftOut')}</b>
          {pack.dropped.map((reason, index) => (
            <span key={String(index)}>
              {String(reason.params.item ?? '')}: {sayReason(reason)}
            </span>
          ))}
        </section>
      )}
    </div>
  )
}
