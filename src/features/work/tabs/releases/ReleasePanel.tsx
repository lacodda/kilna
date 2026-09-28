import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronRight, ExternalLink } from 'lucide-react'
import {
  createRelease,
  deleteRelease,
  markReleased,
  unmarkReleased,
  unscheduleRelease,
  updateRelease,
} from '@/lib/api/releases'
import type { ReleasePatch, ScheduledRelease } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { daysBetween } from '@/lib/readiness'
import { today } from '@/lib/month'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { announceEdited } from '@/lib/edited'
import { labelOf, say as sayLabel, useVocabulary } from '@/lib/useProfile'
import { KindGlyph } from '@/lib/releaseIcon'
import { openExternal, shortLink } from '@/lib/link'
import { cn } from '@/lib/utils'
import { formatDay } from '@/lib/format'
import { ReadyMarks } from '@/components/ReadyMarks'
import { MarkReleasedDialog } from '@/components/MarkReleasedDialog'
import { ReleaseFields } from '@/features/work/tabs/releases/ReleaseFields'
import { ReleaseRowEditor } from '@/features/work/tabs/releases/ReleaseRowEditor'
import { Button } from '@/components/ui/button'
import { RowContextMenu, RowMenu, type RowAction } from '@/components/RowMenu'
import { Select } from '@/components/AppSelect'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'

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
 * the calendar's chip does, minus the drag that only a grid can have.
 */
export function ReleasePanel({ workId, workTitle }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const kinds = useVocabulary(workId).release_kinds

  const [kind, setKind] = useState(kinds[0]?.key ?? '')
  const [editing, setEditing] = useState<ScheduledRelease | null>(null)
  const [marking, setMarking] = useState<ScheduledRelease | null>(null)
  // Which release has its metadata open. One at a time: four descriptions
  // unrolled at once is a page nobody can find their place on, and the
  // question being asked is always about one release.
  const [showing, setShowing] = useState<string | null>(null)

  const releases = useQuery(queries.releasesForWork(workId))

  // What a release touches, whether it is added, removed or brought back -
  // the calendar, the queue, and the work's own list; the hook adds the
  // journal itself.
  const refreshed = [keys.releasesForWork(workId), keys.releases, keys.calendar, keys.releaseQueue]

  const add = useAppMutation({
    mutationFn: () => createRelease({ work_id: workId, kind, title: workTitle }),
    failure: 'toast.releaseSaveFailed',
    refresh: refreshed,
    onSuccess: () => say.ok(t('toast.releaseCreated')),
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

  const save = useAppMutation({
    mutationFn: ({ id, patch }: { id: string; patch: ReleasePatch }) => updateRelease(id, patch),
    failure: 'toast.releaseSaveFailed',
    onSuccess: () => {
      announceEdited({
        client,
        message: t('toast.releaseEdited'),
        refresh: refreshed,
      })
    },
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

  const releasesData = releases.data ?? []
  const now = today()

  const actionsFor = (entry: ScheduledRelease): RowAction[] => {
    const out: RowAction[] = [
      { key: 'edit', label: t('releases.edit'), onSelect: () => setEditing(entry) },
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
          <Select
            className="w-48"
            aria-label={t('releases.kind')}
            value={kind}
            onChange={setKind}
            options={kinds.map((k) => ({ value: k.key, label: sayLabel(k.label) }))}
          />
          <Button onClick={() => add.mutate()} disabled={kind === '' || add.isPending}>
            {t('releases.add')}
          </Button>
        </>
      }
    >
      <Scroll label={t('card.tab.releases')}>
        <Loaded
          query={releases}
          skeleton={<SkeletonList rows={2} secondary={false} />}
          isEmpty={(data) => data.length === 0}
          // Plain, and its way out is the kind and the button above it.
          emptyState={<EmptyState plain title={t('releases.none')} />}
          plain
        >
          {() => (
            <ul className="flex flex-col gap-1">
              {releasesData.map((entry) => {
                const released = entry.status === 'released'
                const url = entry.url
                const kindEntry = kinds.find((entry_) => entry_.key === entry.kind)

                const open = showing === entry.id

                return (
                  <li key={entry.id} className="flex flex-col gap-1">
                    <RowContextMenu
                      actions={actionsFor(entry)}
                      render={
                        <div
                          className={cn(
                            'flex items-center gap-3 rounded-xl border border-line px-3 py-1.5 text-sm',
                            // What went out is history sitting in the list, not a plan
                            // competing for attention - the same dimming the chip uses.
                            released && 'opacity-70',
                            // Lit while its own menu is open, so it is clear which
                            // release the actions belong to.
                            'data-[popup-open]:bg-soft',
                          )}
                        />
                      }
                    >
                      <Button
                        variant="icon"
                        size="icon-sm"
                        onClick={() => setShowing(open ? null : entry.id)}
                        aria-expanded={open}
                        aria-label={t('releases.meta.title')}
                        // Pulled into the row's padding, so the row stays the height
                        // of its words rather than of the button.
                        className="-my-1 -ml-1.5"
                      >
                        <ChevronRight
                          aria-hidden
                          className={cn('transition-transform', open && 'rotate-90')}
                        />
                      </Button>
                      <KindGlyph icon={kindEntry?.icon} className="size-3.5 shrink-0 text-dim" />
                      <span className="font-medium">{labelOf(kinds, entry.kind)}</span>

                      <ReadyMarks
                        readiness={entry.readiness}
                        released={released}
                        daysLeft={
                          released || entry.scheduled_at === null
                            ? null
                            : daysBetween(now, entry.scheduled_at)
                        }
                      />

                      <span className={cn('text-xs', released ? 'text-good' : 'text-dim')}>
                        {released
                          ? t('releases.releasedOn', {
                              date: entry.released_at === null ? '' : formatDay(entry.released_at),
                            })
                          : entry.scheduled_at === null
                            ? t('releases.unscheduled')
                            : formatDay(entry.scheduled_at)}
                      </span>

                      {url !== null && (
                        <Button
                          variant="link"
                          onClick={() => void openExternal(url)}
                          title={url}
                          className="min-w-0 text-xs"
                        >
                          <ExternalLink aria-hidden />
                          <span className="truncate">{shortLink(url)}</span>
                        </Button>
                      )}

                      <span className="ml-auto">
                        <RowMenu actions={actionsFor(entry)} label={t('releases.actions')} />
                      </span>
                    </RowContextMenu>

                    {open && (
                      <div className="pl-6">
                        <ReleaseFields release={entry} />
                      </div>
                    )}
                  </li>
                )
              })}
            </ul>
          )}
        </Loaded>
      </Scroll>

      <ReleaseRowEditor
        release={editing}
        onOpenChange={(open) => {
          if (!open) setEditing(null)
        }}
        onSave={(id, patch) => save.mutate({ id, patch })}
      />

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
