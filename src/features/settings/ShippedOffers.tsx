import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import type { OfferRef, ShippedOffer } from '@/lib/api/types'
import { keepMine, takeShipped } from '@/lib/api/workspace'
import { diffLines } from '@/lib/diff'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { changes, entryName, listOf } from '@/lib/shippedOffer'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'

/**
 * What kilna ships now, beside the entries of the profile that read
 * otherwise (ADR 0087).
 *
 * An entry nobody touched takes a newer version by itself at a start; this
 * is for the rest - one its owner changed, or one met for the first time
 * already reading otherwise - where the newer version waits to be compared
 * and nothing is written until a person chooses. Nothing waits, nothing is
 * shown.
 */
export function ShippedOffers({
  only,
  except,
  folded = false,
}: {
  /** Shown as a line with the count until asked for: Skills keeps its own
   *  list in view. */
  folded?: boolean
  /** The one list shown: Skills shows the actions. */
  only?: string
  /** A list left out: the profile leaves the actions to Skills. */
  except?: string
} = {}) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [open, setOpen] = useState(!folded)
  const offers = (useQuery(queries.shippedOffers(profile.id)).data ?? []).filter(
    (offer) =>
      (only === undefined || offer.collection === only) &&
      (except === undefined || offer.collection !== except),
  )

  const take = useAppMutation({
    mutationFn: (offer: OfferRef) => takeShipped(profile.id, offer),
    refresh: refresh.profile,
    onSuccess: () => say.ok(t('settings.shipped.taken')),
    failure: 'toast.shippedTakeFailed',
  })
  const keep = useAppMutation({
    mutationFn: (kept: OfferRef[]) => keepMine(profile.id, kept),
    refresh: refresh.profile,
    onSuccess: (_, kept) => say.ok(t('settings.shipped.kept', { count: kept.length })),
    failure: 'toast.shippedKeepFailed',
  })

  if (offers.length === 0) return null
  const busy = take.isPending || keep.isPending
  const refOf = (offer: ShippedOffer): OfferRef => ({
    collection: offer.collection,
    key: offer.key,
  })

  return (
    <section className="flex max-w-3xl flex-col gap-2" aria-label={t('settings.shipped.title')}>
      <FieldGroup label={t('settings.shipped.title')} help={t('settings.shipped.hint')}>
        <div>
          <Button onClick={() => keep.mutate(offers.map(refOf))} disabled={busy}>
            {t('settings.shipped.keepAll', { count: offers.length })}
          </Button>
          {!open && (
            <Button variant="ghost" className="ml-2" onClick={() => setOpen(true)}>
              {t('settings.shipped.show', { count: offers.length })}
            </Button>
          )}
        </div>
      </FieldGroup>
      <ul className={open ? 'flex flex-col gap-3' : 'hidden'}>
        {offers.map((offer) => {
          const list = listOf(offer.collection, profile.config)
          const listWord = t(`settings.shipped.list.${list.word}`, { defaultValue: list.word })
          const name = entryName(offer.mine, offer.key)
          return (
            <li
              key={`${offer.collection}/${offer.key}`}
              className="flex flex-col gap-2 rounded-md border border-line p-3"
            >
              <h3 className="flex flex-wrap items-baseline gap-x-2 text-sm font-normal">
                <span className="font-medium text-text">{name}</span>
                <span className="text-dim">
                  {list.of === undefined
                    ? listWord
                    : t('settings.shipped.inKind', { list: listWord, kind: list.of })}
                </span>
              </h3>
              <dl className="flex flex-col gap-1.5 text-xs">
                {changes(offer.mine, offer.standard).map((change) => (
                  <div key={change.field} className="flex flex-col gap-0.5">
                    <dt className="font-mono text-faint">{change.field}</dt>
                    <dd>
                      {change.long ? (
                        <ol className="selectable max-h-60 overflow-y-auto rounded-md border border-line bg-bg py-1 font-mono leading-relaxed">
                          {diffLines(change.mine, change.standard).map((line, index) => (
                            <li
                              key={index}
                              data-change={line.kind}
                              className={cn(
                                'px-2 whitespace-pre-wrap',
                                line.kind === 'added' && 'bg-good-soft',
                                line.kind === 'removed' && 'bg-bad-soft text-dim',
                              )}
                            >
                              <span
                                aria-hidden
                                className="mr-2 inline-block w-2 text-dim select-none"
                              >
                                {line.kind === 'added' ? '+' : line.kind === 'removed' ? '−' : ' '}
                              </span>
                              {line.text === '' ? ' ' : line.text}
                            </li>
                          ))}
                        </ol>
                      ) : (
                        <span className="grid grid-cols-[auto_1fr] gap-x-2">
                          <span className="text-faint">{t('settings.shipped.mine')}</span>
                          <span className="text-dim">{change.mine || '—'}</span>
                          <span className="text-faint">{t('settings.shipped.standard')}</span>
                          <span className="text-text">{change.standard || '—'}</span>
                        </span>
                      )}
                    </dd>
                  </div>
                ))}
              </dl>
              <div className="flex gap-2">
                <Button onClick={() => keep.mutate([refOf(offer)])} disabled={busy}>
                  {t('settings.shipped.keep')}
                </Button>
                <Button variant="primary" onClick={() => take.mutate(refOf(offer))} disabled={busy}>
                  {t('settings.shipped.take')}
                </Button>
              </div>
            </li>
          )
        })}
      </ul>
    </section>
  )
}
