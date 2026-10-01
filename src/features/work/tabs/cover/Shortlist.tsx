import { useTranslation } from 'react-i18next'
import { takeIdea } from '@/lib/api/ideas'
import type { Cover, CoverBoard } from '@/lib/api/types'
import { shortlist } from '@/lib/ideas'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Chip } from '@/components/ui/chip'

interface Props {
  board: CoverBoard
  cover: Cover
}

/**
 * The shortlist across the constructor's idea block (the mockup's chips over
 * the idea): every starred idea, one press to build the cover from it. The
 * one the cover is built from now - the same idea and scene - stands pressed.
 *
 * Nothing when nothing is starred: the board is one press away above.
 */
export function Shortlist({ board, cover }: Props) {
  const { t } = useTranslation()
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

  return (
    <div role="group" aria-label={t('ideas.shortlist')} className="flex flex-wrap gap-1.5">
      {starred.map((card) => {
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
        return (
          <Chip
            key={idea.id}
            pressed={current}
            disabled={take.isPending}
            onPressedChange={() => {
              if (!current) take.mutate(idea.id)
            }}
          >
            {name}
          </Chip>
        )
      })}
    </div>
  )
}
