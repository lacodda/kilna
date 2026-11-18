import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Star } from 'lucide-react'
import { takeIdea } from '@/lib/api/ideas'
import type { Cover, CoverBoard } from '@/lib/api/types'
import { shortlist } from '@/lib/ideas'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

interface Props {
  board: CoverBoard
  cover: Cover
}

/** Past this many, the shortlist offers a search. */
const SEARCHED_FROM = 5

/**
 * The shortlist at the top of the constructor (v0.135, FR-07 7.3): every
 * starred idea as a row - where it came from, its headline and a line or two
 * of it - held at the top while the column scrolls, one press to build the
 * cover from it. The one the cover is built from now stands pressed. A long
 * shortlist is searched.
 *
 * Nothing when nothing is starred: the board is one press away above.
 */
export function Shortlist({ board, cover }: Props) {
  const { t } = useTranslation()
  const [text, setText] = useState('')
  const starred = shortlist(board)
  const workId = board.ideas[0]?.idea.work_id
  const take = useAppMutation({
    mutationFn: (id: string) => takeIdea(id),
    failure: 'ideas.takeFailed',
    refresh: [
      ...(workId === undefined ? [] : [keys.work(workId), keys.coverBoard(workId)]),
      keys.pictures,
      keys.covers,
    ],
  })
  if (starred.length === 0) return null

  const needle = text.trim().toLowerCase()
  const shown = starred.filter(
    ({ idea }) =>
      needle === '' ||
      `${idea.headline} ${idea.concept.idea} ${idea.concept.scene}`.toLowerCase().includes(needle),
  )

  return (
    <div className="sticky top-0 z-10 -mx-3 flex flex-col gap-1.5 border-b border-line bg-raise px-3 py-2">
      <span className="caption flex items-center gap-1.5">
        <Star aria-hidden className="size-3 fill-current text-warn" />
        {t('ideas.shortlist')}
      </span>
      {starred.length >= SEARCHED_FROM && (
        <Input
          type="search"
          value={text}
          placeholder={t('ideas.searchShortlist')}
          aria-label={t('ideas.searchShortlist')}
          onChange={(event) => setText(event.target.value)}
        />
      )}
      <ul
        aria-label={t('ideas.shortlist')}
        className="flex max-h-56 flex-col gap-1 overflow-y-auto"
      >
        {shown.map((card) => {
          const { idea } = card
          const current =
            idea.concept.idea.trim() === cover.idea.trim() &&
            idea.concept.scene.trim() === cover.scene.trim()
          const name =
            idea.headline.trim() ||
            card.look.hero ||
            (idea.source === 'sibling' && card.from_title !== null
              ? t('ideas.source.from', { title: card.from_title })
              : t(`ideas.untitled.${idea.source}`))
          const preview = idea.concept.idea.trim() || idea.concept.scene.trim()
          return (
            <li key={idea.id}>
              <Button
                variant="icon"
                size={null}
                aria-pressed={current}
                disabled={take.isPending}
                onClick={() => {
                  if (!current) take.mutate(idea.id)
                }}
                className={cn(
                  'w-full flex-col items-start gap-0.5 px-2 py-1.5 text-left font-normal whitespace-normal',
                  current && 'bg-accent-soft',
                )}
              >
                <span className="flex w-full min-w-0 items-baseline gap-2">
                  <b className="min-w-0 flex-1 truncate text-sm font-semibold text-text">{name}</b>
                  <span className="shrink-0 text-2xs text-faint">
                    {t(`ideas.source.${idea.source}`)}
                  </span>
                </span>
                {preview !== '' && <span className="line-clamp-2 text-xs text-dim">{preview}</span>}
              </Button>
            </li>
          )
        })}
      </ul>
    </div>
  )
}
