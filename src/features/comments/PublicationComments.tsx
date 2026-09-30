import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ArrowUpRight } from 'lucide-react'
import type { Comment, Publication } from '@/lib/api/types'
import { standingOf } from '@/lib/comments'
import { formatDay } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { labelOf, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { STANDING_DOT } from '@/features/comments/CommentBoard'

/** One publication's comments, with what the summary says about it. */
interface Group {
  workId: string
  publication: Publication | undefined
  comments: Comment[]
}

/**
 * The comments of a work that does not go out itself - a song - read off the
 * works that do (v0.86, ADR 0047).
 *
 * The audience comments under a clip, an audio, a short; never under the song.
 * So the song's tab is a summary, not an inbox: what was said, grouped by
 * where it was said, each group a link to that publication's own comments -
 * where a reply is drafted and answered. A board here would invite writing a
 * reply away from the comment it answers, and filing a new comment under a
 * work nobody can comment on.
 */
export function PublicationComments({ workId }: { workId: string }) {
  const { t } = useTranslation()
  const comments = useQuery(queries.commentsMatching({ under: workId }))
  const publications = useQuery(queries.publications(workId))

  return (
    <Frame
      head={
        <>
          <h2 className="caption">{t('comments.summary.title')}</h2>
          <span className="text-xs text-faint">{t('comments.summary.hint')}</span>
        </>
      }
    >
      <Pane label={t('comments.summary.title')}>
        <Loaded
          query={comments}
          skeleton={<SkeletonList rows={4} />}
          isEmpty={(data) => data.length === 0}
          emptyState={<EmptyState plain title={t('comments.summary.none')} className="p-3" />}
          plain
        >
          {(data) => (
            <div className="flex flex-col">
              {groupsOf(data, publications.data?.items ?? []).map((group) => (
                <PublicationGroup key={group.workId} group={group} />
              ))}
            </div>
          )}
        </Loaded>
      </Pane>
    </Frame>
  )
}

/**
 * The comments by the publication they were left under, in the order the
 * publications are listed on the song - the order its overview shows them in.
 */
function groupsOf(comments: Comment[], publications: Publication[]): Group[] {
  const byWork = new Map<string, Comment[]>()
  for (const comment of comments) {
    if (comment.work_id === null) continue
    byWork.set(comment.work_id, [...(byWork.get(comment.work_id) ?? []), comment])
  }
  const rank = (id: string) => {
    const at = publications.findIndex((one) => one.work_id === id)
    return at === -1 ? publications.length : at
  }
  return [...byWork.entries()]
    .sort(([a], [b]) => rank(a) - rank(b))
    .map(([id, mine]) => ({
      workId: id,
      publication: publications.find((one) => one.work_id === id),
      comments: mine,
    }))
}

function PublicationGroup({ group }: { group: Group }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { config } = useProfile()
  const waiting = group.comments.filter((one) => standingOf(one) === 'waiting').length
  const title = group.publication?.title ?? t('comments.summary.somewhere')

  return (
    <section aria-label={title} className="border-b border-line last:border-b-0">
      <header className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 bg-softer px-3 py-2">
        <b className="min-w-0 truncate text-sm font-semibold">{title}</b>
        {group.publication !== undefined && (
          <span className="text-xs text-faint">
            {labelOf(config.work_kinds, group.publication.kind)}
          </span>
        )}
        <span className="text-xs text-dim">
          {t('comments.summary.count', { count: group.comments.length })}
          {waiting > 0 && (
            <span className="text-accent-2">
              {' · '}
              {t('comments.waitingCount', { count: waiting })}
            </span>
          )}
        </span>
        <Button
          size="xs"
          variant="link"
          className="ml-auto"
          onClick={() => void navigate(`/works/${group.workId}/comments`)}
        >
          {t('comments.summary.open')}
          <ArrowUpRight aria-hidden />
        </Button>
      </header>
      <ul className="flex flex-col">
        {group.comments.map((comment) => (
          <SummaryRow key={comment.id} comment={comment} />
        ))}
      </ul>
    </section>
  )
}

/** One comment, read only: who, the opening of what they said, where it stands. */
function SummaryRow({ comment }: { comment: Comment }) {
  const { t } = useTranslation()
  const standing = standingOf(comment)

  return (
    <li className="flex min-w-0 items-center gap-2.5 px-3 py-1.5 text-sm">
      <span aria-hidden className={cn('size-1.5 shrink-0 rounded-full', STANDING_DOT[standing])} />
      <span className="shrink-0 font-medium">{comment.author ?? t('comments.someone')}</span>
      <span className="min-w-0 flex-1 truncate text-dim">{comment.body}</span>
      <span className="shrink-0 text-xs text-faint">
        {t(`comments.standing.${standing}`)}
        {comment.commented_on !== null && ` · ${formatDay(comment.commented_on)}`}
      </span>
    </li>
  )
}
