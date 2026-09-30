import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Dated } from '@/lib/api/types'
import { seenThrough, whenOf, type LensChoice } from '@/lib/canon'
import { queries } from '@/lib/query/queries'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { LayerMark } from '@/features/canon/SectionBlock'
import { cn } from '@/lib/utils'

/**
 * The world in the order it happened: every fact with a time inside the
 * world, earliest first, grouped by year, each under the card it is a fact
 * of. The order is the backend's - the sort key read off the words, or typed
 * - and the lens dims what a task may not see, the same rule as on a card.
 */
export function CanonTimeline({
  lens,
  onOpen,
}: {
  lens: LensChoice
  onOpen: (id: string) => void
}) {
  const { t } = useTranslation()
  const dated = useQuery(queries.canonTimeline(null))

  return (
    <Pane label={t('canon.timeline')} bodyClassName="flex flex-col px-3.5 pb-4">
      <p className="pt-2 text-xs text-faint">{t('canon.timelineHint')}</p>
      <Loaded
        query={dated}
        skeleton={<SkeletonList rows={6} />}
        isEmpty={(data) => data.length === 0}
        emptyState={
          <EmptyState
            plain
            title={t('canon.timelineEmpty')}
            body={t('canon.timelineEmptyBody')}
            className="p-2"
          />
        }
        plain
      >
        {(facts) => (
          <div className="flex flex-col">
            {byYear(facts).map(([year, group]) => (
              <section key={year} aria-label={year} className="pt-3">
                <h3 className="pb-1 font-mono text-sm font-semibold text-dim">{year}</h3>
                <ol className="flex flex-col gap-0.5 border-l border-line pl-3">
                  {group.map((one) => (
                    <li
                      key={one.fact.id}
                      className={cn(
                        'grid grid-cols-[24px_minmax(0,1fr)] items-start gap-2.5 rounded-md px-2 py-1 hover:bg-soft/50',
                        !seenThrough(one.lenses, lens) && 'opacity-25',
                      )}
                    >
                      <LayerMark layer={one.fact.layer} />
                      <div className="min-w-0 text-sm">
                        <span className="mr-2 font-mono text-2xs text-dim">{whenOf(one.fact)}</span>
                        <Button
                          variant="link"
                          className="font-semibold"
                          onClick={() => onOpen(one.fact.note_id)}
                        >
                          {one.card_title ?? t('canon.untitled')}
                        </Button>
                        <span className="selectable"> — {one.fact.body}</span>
                        {one.fact.status !== 'canon' && (
                          <span className="ml-1.5 rounded-sm bg-info-soft px-1 text-2xs text-info">
                            {t(`canon.status.${one.fact.status}`)}
                          </span>
                        )}
                      </div>
                    </li>
                  ))}
                </ol>
              </section>
            ))}
          </div>
        )}
      </Loaded>
    </Pane>
  )
}

/** The dated facts in runs of one year, in the order they came: the year is
 *  the first four characters of the sort key. */
function byYear(facts: readonly Dated[]): [string, Dated[]][] {
  const runs: [string, Dated[]][] = []
  for (const one of facts) {
    const year = one.fact.when?.sort?.slice(0, 4) ?? '?'
    const last = runs.at(-1)
    if (last !== undefined && last[0] === year) last[1].push(one)
    else runs.push([year, [one]])
  }
  return runs
}
