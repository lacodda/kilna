import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { StarredAnswer } from '@/lib/api/types'
import { formatMoment } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { EmptyState } from '@/components/ui/empty-state'
import { RowButton } from '@/components/ui/list-row'
import { SkeletonList } from '@/components/ui/skeleton'
import { Pane } from '@/components/frame'

interface Props {
  /** The work whose chats' answers these are. Absent: every chat's. */
  workId?: string
  /** The answer the conversation beside it is open on. */
  open: string | null
  onOpen: (answer: StarredAnswer) => void
}

/**
 * The starred answers (v0.106): what the assistant said that was worth
 * coming back to, newest star first, each with the chat it was said in -
 * and, in the drawer, which holds every work's chats, the work. Opening one
 * opens its chat on it.
 */
export function StarredList({ workId, open, onOpen }: Props) {
  const { t } = useTranslation()
  const starred = useQuery(queries.starred(workId))
  const answers = starred.data ?? []

  return (
    <Pane
      label={t('assistant.starredTitle')}
      bodyClassName="p-1.5"
      head={<span className="caption">{t('assistant.starredTitle')}</span>}
    >
      {starred.isPending ? (
        <SkeletonList rows={3} />
      ) : answers.length === 0 ? (
        <EmptyState
          plain
          title={t('assistant.starredEmptyTitle')}
          body={t('assistant.starredEmpty')}
        />
      ) : (
        <ul className="flex flex-col gap-0.5">
          {answers.map((answer) => {
            const where = [
              answer.chat_title ?? answer.first_prompt ?? t('assistant.untitled'),
              workId === undefined ? (answer.work_title ?? '') : '',
              formatMoment(answer.created_at),
            ]
              .filter((part) => part !== '')
              .join(' · ')
            return (
              <li key={answer.message_id}>
                <RowButton
                  selected={answer.message_id === open}
                  description={where}
                  onClick={() => {
                    onOpen(answer)
                  }}
                >
                  {answer.excerpt}
                </RowButton>
              </li>
            )
          })}
        </ul>
      )}
    </Pane>
  )
}
