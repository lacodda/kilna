import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { kindVerdicts } from '@/lib/api'
import { keys } from '@/lib/query'
import { labelOf, useVocabulary } from '@/lib/useProfile'
import { cn } from '@/lib/utils'

interface Props {
  workId: string
}

/**
 * The same score, read down every channel the craft ships to.
 *
 * A clip lives or dies on its hook and its picture; the same song as an audio
 * release is carried by its words. Weighing one set of answers by each kind's
 * own weights turns "this song is a 67" into the more useful "this song is a
 * clip and an ordinary audio release" - one score, several honest verdicts,
 * which is what `axis_weights` was put in the profile for in v0.50.
 *
 * Kinds that reweigh nothing are shown too, greyed: leaving them out would
 * read as "not applicable here" rather than "weighs them the way you do".
 */
export function KindVerdicts({ workId }: Props) {
  const { t } = useTranslation()
  // The doors and tiers of this work's own kind: a verdict per release kind
  // is a reading of one work, and its kind says which kinds and which tiers.
  const { release_kinds: releaseKinds, tiers } = useVocabulary(workId)

  const verdicts = useQuery({
    queryKey: keys.kindVerdicts(workId),
    queryFn: () => kindVerdicts(workId),
  })

  const rows = verdicts.data ?? []
  if (rows.length === 0) return null

  // Nothing to compare when no kind bends the weights: every row would carry
  // the same number the total already shows.
  if (!rows.some((row) => row.reweighed)) return null

  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-[10px] font-semibold uppercase tracking-[0.08em] text-faint">
        {t('score.byKind')}
      </span>

      <ul className="flex flex-wrap gap-1.5">
        {rows.map((row) => (
          <li
            key={row.kind}
            title={row.reweighed ? t('score.byKindReweighed') : t('score.byKindSame')}
            className={cn(
              'flex items-baseline gap-1.5 rounded-lg border border-line px-2 py-1 text-xs',
              !row.reweighed && 'opacity-60',
            )}
          >
            <span className="text-dim">{labelOf(releaseKinds, row.kind)}</span>
            <span className="font-mono font-semibold tabular-nums">{row.total.toFixed(1)}</span>
            {row.tier !== null && (
              <span className="rounded bg-soft px-1 py-0.5 text-[10px]">
                {labelOf(tiers, row.tier)}
              </span>
            )}
          </li>
        ))}
      </ul>
    </div>
  )
}
