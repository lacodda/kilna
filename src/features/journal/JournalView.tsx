import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { markJournalRead } from '@/lib/api/journal'
import { needsALook } from '@/lib/journalLook'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { EmptyState } from '@/components/ui/empty-state'
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
  const waiting = (entries.data ?? []).filter(needsALook)

  return (
    <Frame
      head={
        <>
          <p className="text-sm text-faint">{t('journal.hint')}</p>
          {/* Two chips rather than a segment, as the mockup draws the
              filter: the second one is tinted and counted while something
              waits, so the head says there is something to look at before
              anyone switches to it. Pressing the chip that is on leaves it
              on - one of the two is always the view. */}
          <ChipGroup
            aria-label={t('journal.title')}
            value={[unreadOnly ? 'unread' : 'all']}
            onValueChange={([picked]) => {
              if (picked !== undefined) setUnreadOnly(picked === 'unread')
            }}
            className="ml-auto"
          >
            <Chip value="all">{t('journal.all')}</Chip>
            {/* The count is written into the words rather than handed to
                the chip's `count`: that one is drawn as a separate span and
                read with the label glued to it - "Needs a look1". */}
            <Chip value="unread" variant={waiting.length > 0 ? 'warn' : 'outline'}>
              {waiting.length > 0
                ? `${t('journal.unreadOnly')} · ${waiting.length}`
                : t('journal.unreadOnly')}
            </Chip>
          </ChipGroup>
          <Button
            variant="ghost"
            size="sm"
            disabled={markRead.isPending || waiting.length === 0}
            // Nothing to mark says so, rather than sitting there greyed out
            // for no reason anyone can find.
            disabledReason={waiting.length === 0 ? t('empty.journalClearTitle') : undefined}
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
          const shown = unreadOnly ? waiting : data
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
            <Pane label={t('journal.title')}>
              <JournalLines entries={shown} />
            </Pane>
          )
        }}
      </Loaded>
    </Frame>
  )
}
