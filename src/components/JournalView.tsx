import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { listJournal, markJournalRead } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { SkeletonList } from '@/components/ui/Skeleton'
import { JournalLines } from '@/components/JournalFeed'
import { cn } from '@/lib/utils'

/**
 * Everything that happened in this profile, newest first.
 *
 * The whole screen is read-only. Nothing here can be edited or undone from the
 * feed — an entry is a record of a decision, and a record that can be rewritten
 * is not one.
 */
export function JournalView() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [unreadOnly, setUnreadOnly] = useState(false)

  const entries = useQuery({ queryKey: keys.journalFeed, queryFn: listJournal })

  const markRead = useMutation({
    mutationFn: markJournalRead,
    onSuccess: () => void client.invalidateQueries({ queryKey: keys.journal }),
    onError: (cause) => say.failedTo(t('toast.loadFailed'), cause),
  })

  if (entries.isPending) return <SkeletonList rows={6} />

  if (entries.isError) {
    return (
      <p role="alert" className="text-sm text-bad">
        {t('toast.loadFailed')}
      </p>
    )
  }

  // Filtering on the client: the feed is one page of at most two hundred, and a
  // round trip to hide some of them would be slower than not hiding them.
  const needsALook = entries.data.filter(
    (entry) => entry.level === 'warn' && entry.read_at === null,
  )
  const shown = unreadOnly ? needsALook : entries.data

  return (
    <div className="flex flex-col gap-3">
      <header className="flex items-center gap-3">
        <h2 className="text-sm font-semibold">{t('journal.title')}</h2>
        <p className="text-xs text-dim">{t('journal.hint')}</p>

        <div className="ml-auto flex items-center gap-1.5">
          {[false, true].map((only) => (
            <Button
              key={String(only)}
              variant={unreadOnly === only ? 'soft' : 'ghost'}
              size="sm"
              aria-pressed={unreadOnly === only}
              onClick={() => setUnreadOnly(only)}
            >
              {t(only ? 'journal.unreadOnly' : 'journal.all')}
              {only && needsALook.length > 0 && (
                <span className={cn('ml-1.5 tabular-nums text-warn')}>{needsALook.length}</span>
              )}
            </Button>
          ))}
          <Button
            variant="ghost"
            size="sm"
            disabled={markRead.isPending || needsALook.length === 0}
            onClick={() => markRead.mutate()}
          >
            {t('journal.markRead')}
          </Button>
        </div>
      </header>

      <JournalLines
        entries={shown}
        emptyTitle={t(unreadOnly ? 'empty.journalClearTitle' : 'empty.journalTitle')}
        emptyBody={t(unreadOnly ? 'empty.journalClearBody' : 'empty.journalBody')}
      />
    </div>
  )
}
