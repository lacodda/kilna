import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { type UseQueryResult, useQuery } from '@tanstack/react-query'
import { Eye, EyeOff, Plus, X } from 'lucide-react'
import type { Score } from '@/lib/api/types'
import { formatDay, formatTotal, formatTotalDelta } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { labelOf, useVocabulary } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { EmptyState } from '@/components/ui/empty-state'
import { RowButton } from '@/components/ui/list-row'
import { SkeletonList } from '@/components/ui/skeleton'
import { Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { cn } from '@/lib/utils'

interface Props {
  workId: string
  history: UseQueryResult<Score[]>
  /** The score open on the right; `undefined` while a new one is given. */
  openId: string | undefined
  /** What a score is called: which draft it judged. */
  titleOf: (score: Score) => string
  blind: boolean
  /** Blind, and no verdict of this card's own yet: the rows are held back. */
  hiding: boolean
  onBlind: (blind: boolean) => void
  onReveal: () => void
  onOpen: (scoreId: string) => void
  onNew: () => void
  onDelete: (scoreId: string) => void
}

/**
 * The scores a work has been given, newest first, each with what it did to
 * the one before it - and, under them, what was written about the work.
 *
 * The history of totals is the most valuable thing on the tab, which is why
 * it is a list beside the open score rather than a picker folded into a menu.
 */
export function ScoreList({
  workId,
  history,
  openId,
  titleOf,
  blind,
  hiding,
  onBlind,
  onReveal,
  onOpen,
  onNew,
  onDelete,
}: Props) {
  const { t } = useTranslation()
  const { tiers, version_roles: roles } = useVocabulary(workId)
  const versions = useQuery(queries.versions(workId))
  const count = history.data?.length ?? 0

  // The roles that comment rather than stand as the work: a review, a
  // critique. Read from the profile, the same rule the versions tab and the
  // catalogue's count draw by.
  const commenting = new Set(
    roles.filter((role) => role.comments_on !== undefined).map((role) => role.key),
  )
  // Newest first, as the scores above are.
  const written = (versions.data ?? [])
    .filter((version) => commenting.has(version.role))
    .sort((left, right) => right.created_at.localeCompare(left.created_at))

  return (
    <Pane
      label={t('score.history', { count })}
      bodyClassName="flex flex-col gap-3 p-1.5"
      head={
        <>
          <span className="caption">{t('score.history', { count })}</span>
          {count > 0 && (
            <Chip
              pressed={blind}
              onPressedChange={onBlind}
              title={t('score.blindOnHint')}
              className="ml-auto"
            >
              {blind ? (
                <EyeOff aria-hidden className="size-3.5" />
              ) : (
                <Eye aria-hidden className="size-3.5" />
              )}
              {t('score.blindOn')}
            </Chip>
          )}
        </>
      }
      foot={
        <Button variant="ghost" size="sm" className="w-full justify-center" onClick={onNew}>
          <Plus aria-hidden />
          {t('score.new')}
        </Button>
      }
    >
      <Loaded
        query={history}
        skeleton={<SkeletonList rows={3} />}
        isEmpty={(data) => data.length === 0}
        // Plain, and its way out is the button at the foot.
        emptyState={<EmptyState plain title={t('score.none')} className="p-2" />}
        plain
      >
        {(scores) =>
          hiding ? (
            <p className="rounded-md border border-dashed border-line p-2 text-xs text-dim">
              {t('score.blindHidden')}{' '}
              <Button variant="link" onClick={onReveal}>
                {t('score.blindReveal')}
              </Button>
            </p>
          ) : (
            <ul className="flex flex-col gap-0.5">
              {scores.map((score, index) => {
                // What this score did to the one before it: the list is
                // newest first, so the one before is the next row down.
                const before = scores[index + 1]?.total
                const delta = before === undefined ? undefined : score.total - before
                const moved = delta === undefined ? null : formatTotalDelta(delta)
                const tone =
                  delta === undefined || moved === null
                    ? 'text-faint'
                    : delta > 0
                      ? 'text-good'
                      : 'text-bad'

                return (
                  <li key={score.id} className="group relative">
                    <RowButton
                      selected={score.id === openId}
                      onClick={() => onOpen(score.id)}
                      title={score.note ?? undefined}
                      // Room at the right edge for the delete laid over it.
                      className="pr-8"
                      description={
                        <span className="font-mono">
                          {formatDay(score.scored_at)}
                          {score.tier !== null && ` · ${labelOf(tiers, score.tier)}`}
                          {/* Who judged, when it was not you: null has meant
                              the author since v0.50, so your own rows stay
                              unlabelled and a second opinion shows by
                              contrast. */}
                          {score.rater !== null && score.rater !== '' && ` · ${score.rater}`}
                        </span>
                      }
                      end={
                        <>
                          <span className={cn('font-mono', tone)}>{moved ?? '—'}</span>
                          {/* A column of its own width, so the changes line
                              up down the list whatever the totals' digits. */}
                          <span className="min-w-11 text-right font-mono text-base font-semibold text-text">
                            {formatTotal(score.total)}
                          </span>
                        </>
                      }
                    >
                      {titleOf(score)}
                    </RowButton>
                    {/* Over the row's right edge, on hover: deleting is rare,
                        and a cross on every row reads as the row's purpose. */}
                    <Button
                      variant="danger"
                      size="icon-sm"
                      title={t('score.delete')}
                      aria-label={t('score.delete')}
                      onClick={() => onDelete(score.id)}
                      className="absolute top-1/2 right-1 -translate-y-1/2 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
                    >
                      <X aria-hidden className="size-3.5" />
                    </Button>
                  </li>
                )
              })}
            </ul>
          )
        }
      </Loaded>

      {/* What was written about this work, beside what it was marked. A
          critique is a version in a commenting role; these are the same rows
          the Versions tab holds, reached from a second place rather than
          copied. A score is a number with a reason, and the reason should be
          one click from the number. */}
      {written.length > 0 && (
        <section className="flex flex-col gap-0.5 border-t border-line pt-2">
          <h4 className="px-2 pb-1 caption">{t('score.written')}</h4>
          {written.map((version) => (
            <Link
              key={version.id}
              to={`/works/${workId}/versions?version=${version.id}`}
              className="flex items-baseline gap-2 rounded-md px-2 py-1.5 no-underline transition-colors hover:bg-soft"
            >
              <span className="shrink-0 text-xs text-dim">{labelOf(roles, version.role)}</span>
              <span className="min-w-0 flex-1 truncate text-sm text-text">
                {version.label ?? t('versions.revision', { number: version.revision })}
              </span>
              <span className="shrink-0 font-mono text-xs text-faint">
                {formatDay(version.created_at)}
              </span>
            </Link>
          ))}
        </section>
      )}
    </Pane>
  )
}
