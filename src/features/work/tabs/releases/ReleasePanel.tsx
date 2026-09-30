import { useState } from 'react'
import { useSearchParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  createRelease,
  deleteRelease,
  markReleased,
  unmarkReleased,
  unscheduleRelease,
} from '@/lib/api/releases'
import type { ScheduledRelease } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { today } from '@/lib/month'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { say as sayLabel, useVocabulary } from '@/lib/useProfile'
import { MarkReleasedDialog } from '@/components/MarkReleasedDialog'
import { Button } from '@/components/ui/button'
import type { RowAction } from '@/components/RowMenu'
import { Select } from '@/components/AppSelect'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { ReleaseRow } from '@/features/work/tabs/releases/ReleaseRow'

interface Props {
  workId: string
  workTitle: string
}

/**
 * Where a work's releases live.
 *
 * Until v0.45 this tab could add a release and delete it, and nothing else: the
 * date, the link and the mark all lived in the calendar, so a release opened
 * from the work it belongs to could not be acted on at all. It now offers what
 * the calendar's chip does, minus the drag that only a grid can have - and
 * since v0.80 a release is edited where it stands, in its row unrolled, with
 * the calendar's own form.
 */
export function ReleasePanel({ workId, workTitle }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const kinds = useVocabulary(workId).release_kinds

  // The kind picked for the next release. Read against the list rather than
  // stored as the first key: the list is empty until the work has loaded, and
  // a first key taken then was an empty choice that never filled in.
  const [picked, setPicked] = useState<string | null>(null)
  const kind =
    picked !== null && kinds.some((entry) => entry.key === picked) ? picked : (kinds[0]?.key ?? '')

  const [marking, setMarking] = useState<ScheduledRelease | null>(null)
  // Which release is unrolled. One at a time: four descriptions unrolled at
  // once is a page nobody can find their place on, and the question being
  // asked is always about one release. A release named in the address opens
  // on arrival: the status bar over a publication's cover sends the person
  // here, to the release whose meta waits for them.
  const [params] = useSearchParams()
  const [showing, setShowing] = useState<string | null>(() => params.get('release'))

  const releases = useQuery(queries.releasesForWork(workId))

  // What a release touches, whether it is added, removed or brought back -
  // the calendar, the queue, and the work's own list; the hook adds the
  // journal itself.
  const refreshed = [keys.releasesForWork(workId), keys.releases, keys.calendar, keys.releaseQueue]

  const add = useAppMutation({
    mutationFn: () => createRelease({ work_id: workId, kind, title: workTitle }),
    failure: 'toast.releaseSaveFailed',
    refresh: refreshed,
    onSuccess: (created) => {
      // Scheduling is what the button is for, and a new release has no day
      // yet: it opens where the day is picked.
      setShowing(created.id)
      say.ok(t('toast.releaseCreated'))
    },
  })

  const remove = useAppMutation({
    mutationFn: deleteRelease,
    failure: 'toast.releaseSaveFailed',
    onSuccess: (deletionId) =>
      announceDeleted({
        client,
        deletionId,
        message: t('toast.releaseDeleted'),
        refresh: refreshed,
      }),
  })

  const release = useAppMutation({
    mutationFn: ({ id, url, at }: { id: string; url: string | null; at: string | null }) =>
      markReleased(id, url, at),
    failure: 'toast.releaseSaveFailed',
    refresh: refreshed,
    onSuccess: () => say.ok(t('toast.releaseReleased')),
  })

  const unrelease = useAppMutation({
    mutationFn: unmarkReleased,
    failure: 'toast.releaseSaveFailed',
    refresh: refreshed,
    onSuccess: () => say.ok(t('toast.releaseUnreleased')),
  })

  const unschedule = useAppMutation({
    mutationFn: unscheduleRelease,
    failure: 'toast.releaseSaveFailed',
    refresh: refreshed,
    onSuccess: () => say.ok(t('toast.releaseUnscheduled')),
  })

  const copyLink = (url: string) => {
    navigator.clipboard.writeText(url).then(
      () => say.ok(t('toast.linkCopied')),
      (cause: unknown) => say.failedTo(t('toast.linkCopyFailed'), cause),
    )
  }

  const now = today()

  const actionsFor = (entry: ScheduledRelease): RowAction[] => {
    const out: RowAction[] = [
      // Editing is the row unrolled: the form is there, not in a dialog.
      { key: 'edit', label: t('releases.edit'), onSelect: () => setShowing(entry.id) },
    ]

    if (entry.status === 'released') {
      out.push({
        key: 'unrelease',
        label: t('releases.unmarkReleased'),
        onSelect: () => unrelease.mutate(entry.id),
      })
    } else {
      out.push({
        key: 'release',
        label: t('calendar.markReleased'),
        onSelect: () => setMarking(entry),
      })
      if (entry.scheduled_at !== null) {
        out.push({
          key: 'unschedule',
          label: t('calendar.unschedule'),
          onSelect: () => unschedule.mutate(entry.id),
        })
      }
    }

    if (entry.url !== null) {
      const url = entry.url
      out.push({ key: 'copy', label: t('releases.copyLink'), onSelect: () => copyLink(url) })
    }

    out.push({
      key: 'delete',
      label: t('releases.delete'),
      danger: true,
      onSelect: () => remove.mutate(entry.id),
    })

    return out
  }

  return (
    <Frame
      // Adding stands above the list rather than following its last row
      // down: with a dozen releases it had gone under the card's edge.
      head={
        <>
          <h2 className="caption">{t('card.tab.releases')}</h2>
          {/* The mockup's head row: the kind and the button at the far end,
              at the height of the rows they add to. */}
          <div data-density="compact" className="ml-auto flex items-center gap-2">
            <Select
              className="w-44"
              // Not "Kind": an unrolled row has a Kind of its own, and two
              // controls of one name are one name too few.
              aria-label={t('releases.newKind')}
              value={kind}
              onChange={setPicked}
              options={kinds.map((k) => ({ value: k.key, label: sayLabel(k.label) }))}
            />
            <Button
              variant="primary"
              onClick={() => add.mutate()}
              disabled={kind === '' || add.isPending}
            >
              {t('releases.add')}
            </Button>
          </div>
        </>
      }
    >
      <Pane label={t('card.tab.releases')}>
        <Loaded
          query={releases}
          skeleton={<SkeletonList rows={2} secondary={false} />}
          isEmpty={(data) => data.length === 0}
          // Plain, and its way out is the kind and the button above it.
          emptyState={<EmptyState plain title={t('releases.none')} className="p-3" />}
          plain
        >
          {(data) => (
            <ul className="flex flex-col">
              {data.map((entry) => (
                <ReleaseRow
                  key={entry.id}
                  release={entry}
                  open={showing === entry.id}
                  onToggle={() => setShowing(showing === entry.id ? null : entry.id)}
                  actions={actionsFor(entry)}
                  today={now}
                  refreshed={refreshed}
                />
              ))}
            </ul>
          )}
        </Loaded>
      </Pane>

      <MarkReleasedDialog
        release={marking}
        today={now}
        onOpenChange={(open) => {
          if (!open) setMarking(null)
        }}
        onConfirm={(id, url, at) => release.mutate({ id, url, at })}
      />
    </Frame>
  )
}
