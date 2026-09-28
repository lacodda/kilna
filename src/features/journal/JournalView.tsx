import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { markJournalRead } from '@/lib/api/journal'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { JournalLines } from '@/features/journal/JournalFeed'

/**
 * Everything that happened in this profile, newest first.
 *
 * The whole screen is read-only. Nothing here can be edited or undone from the
 * feed — an entry is a record of a decision, and a record that can be rewritten
 * is not one.
 */
export function JournalView() {
  const { t } = useTranslation()
  const [unreadOnly, setUnreadOnly] = useState(false)

  const entries = useQuery(queries.journalFeed())

  const markRead = useAppMutation({
    mutationFn: markJournalRead,
    failure: 'toast.loadFailed',
  })

  // Filtering on the client: the feed is one page of at most two hundred, and a
  // round trip to hide some of them would be slower than not hiding them.
  const needsALook = (entries.data ?? []).filter(
    (entry) => entry.level === 'warn' && entry.read_at === null,
  )

  return (
    <Frame
      head={
        <>
          <p className="text-xs text-dim">{t('journal.hint')}</p>
          <SegmentedControl
            aria-label={t('journal.title')}
            value={unreadOnly ? 'unread' : 'all'}
            onValueChange={(value) => setUnreadOnly(value === 'unread')}
            className="ml-auto"
          >
            <Segment value="all">{t('journal.all')}</Segment>
            <Segment value="unread">
              {t('journal.unreadOnly')}
              {needsALook.length > 0 && (
                <span className="tabular-nums text-warn">{needsALook.length}</span>
              )}
            </Segment>
          </SegmentedControl>
          <Button
            variant="ghost"
            size="sm"
            disabled={markRead.isPending || needsALook.length === 0}
            onClick={() => markRead.mutate()}
          >
            {t('journal.markRead')}
          </Button>
        </>
      }
    >
      <Loaded
        query={entries}
        fill
        skeleton={<SkeletonList rows={6} />}
        isEmpty={(data) => data.length === 0}
        emptyState={
          <EmptyState
            title={t('empty.journalTitle')}
            body={t('empty.journalBody')}
            className="flex-1"
          />
        }
      >
        {(data) => {
          const shown = unreadOnly ? needsALook : data
          // Nothing that needs a look is a filter that matched nothing, not
          // an empty history: the way out is back to everything.
          if (shown.length === 0) {
            return (
              <EmptyState
                variant="filtered"
                title={t('empty.journalClearTitle')}
                body={t('empty.journalClearBody')}
                action={
                  <Button size="sm" onClick={() => setUnreadOnly(false)}>
                    {t('journal.all')}
                  </Button>
                }
                className="flex-1"
              />
            )
          }
          return (
            <Pane label={t('journal.title')} bodyClassName="px-3">
              <JournalLines entries={shown} />
            </Pane>
          )
        }}
      </Loaded>
    </Frame>
  )
}
