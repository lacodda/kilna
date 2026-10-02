import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ExternalLink, LoaderCircle } from 'lucide-react'
import {
  createRelease,
  deleteRelease,
  markReleased,
  unmarkReleased,
  unscheduleRelease,
} from '@/lib/api/releases'
import type { ScheduledRelease, Work } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { openExternal, shortLink } from '@/lib/link'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { today } from '@/lib/month'
import { KindGlyph } from '@/lib/releaseIcon'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { labelOf, say as sayLabel, useVocabulary } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Select } from '@/components/AppSelect'
import { MarkReleasedDialog } from '@/components/MarkReleasedDialog'
import { RowMenu, type RowAction } from '@/components/RowMenu'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { ReleaseDetails } from '@/features/work/tabs/releases/ReleaseDetails'
import { ReleaseReadiness } from '@/features/work/tabs/releases/ReleaseReadiness'
import { useReleaseMeta } from '@/features/work/tabs/releases/releaseMeta'

/**
 * A publication's release, whole, where the publication is read (v0.90): the
 * place it goes out, the day and the hour, whether it went, the link, what it
 * goes out under and the button that writes it, what is proposed and the
 * files that go with it.
 *
 * A publication goes out once (ADR 0051), so this is the release's one home.
 * Until v0.90 a Releases tab listed them, folded, one row per place; the list
 * was always one row long in the owner's workspace, and the row had to be
 * unrolled to be worked on. Now the block is open where the overview is, and
 * a publication with no release yet offers the place to plan it for.
 */
export function ReleaseBlock({ work }: { work: Work }) {
  const releases = useQuery(queries.releasesForWork(work.id))

  if (releases.data === undefined) {
    return <Skeleton className="h-24 w-full" />
  }
  const release = releases.data[0]
  return release === undefined ? (
    <PlanRelease work={work} />
  ) : (
    <PlannedRelease release={release} key={release.id} />
  )
}

/** The query areas a change to a work's release disturbs: the calendar, the
 *  queue and the work's own; the mutation hook adds the journal itself. */
function refreshedFor(workId: string) {
  return [keys.releasesForWork(workId), keys.releases, keys.calendar, keys.releaseQueue] as const
}

/** No release yet: the places the kind goes out through, and the button. */
function PlanRelease({ work }: { work: Work }) {
  const { t } = useTranslation()
  const doors = useVocabulary(work.id).release_kinds
  const [picked, setPicked] = useState<string | null>(null)
  // Read against the list rather than stored as the first key: the list is
  // empty until the work has loaded.
  const door =
    picked !== null && doors.some((entry) => entry.key === picked) ? picked : (doors[0]?.key ?? '')

  const add = useAppMutation({
    mutationFn: () => createRelease({ work_id: work.id, kind: door }),
    failure: 'toast.releaseSaveFailed',
    refresh: refreshedFor(work.id),
    onSuccess: () => say.ok(t('toast.releaseCreated')),
  })

  return (
    <div data-density="compact" className="flex flex-wrap items-center gap-2">
      <p className="mr-auto text-sm text-faint">{t('releases.none')}</p>
      <Select
        className="w-44"
        aria-label={t('releases.kind')}
        value={door}
        onChange={setPicked}
        options={doors.map((entry) => ({ value: entry.key, label: sayLabel(entry.label) }))}
      />
      <Button
        variant="primary"
        onClick={() => add.mutate()}
        disabled={door === '' || add.isPending}
      >
        {t('releases.add')}
      </Button>
    </div>
  )
}

function PlannedRelease({ release }: { release: ScheduledRelease }) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const doors = useVocabulary(release.work_id).release_kinds
  const door = doors.find((entry) => entry.key === release.kind)
  const refreshed = refreshedFor(release.work_id)
  const now = today()
  const [marking, setMarking] = useState<ScheduledRelease | null>(null)

  const released = release.status === 'released'
  const url = release.url

  // Said on the head line, so it is seen before the fields are reached: the
  // meta is being written, or some of it waits to be taken.
  const meta = useReleaseMeta(release.work_kind)
  const writing = meta.writing(release.id)
  const proposals = useQuery(queries.releaseProposals(release.id))
  const waiting = (proposals.data ?? []).reduce((sum, one) => sum + one.fields.length, 0)

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
  const markOut = useAppMutation({
    mutationFn: ({ id, url: link, at }: { id: string; url: string | null; at: string | null }) =>
      markReleased(id, link, at),
    failure: 'toast.releaseSaveFailed',
    refresh: refreshed,
    onSuccess: () => say.ok(t('toast.releaseReleased')),
  })
  const unmark = useAppMutation({
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

  const copyLink = (link: string) => {
    navigator.clipboard.writeText(link).then(
      () => say.ok(t('toast.linkCopied')),
      (cause: unknown) => say.failedTo(t('toast.linkCopyFailed'), cause),
    )
  }

  const actions: RowAction[] = []
  if (released) {
    actions.push({
      key: 'unrelease',
      label: t('releases.unmarkReleased'),
      onSelect: () => unmark.mutate(release.id),
    })
  } else {
    actions.push({
      key: 'release',
      label: t('calendar.markReleased'),
      onSelect: () => setMarking(release),
    })
    if (release.scheduled_at !== null) {
      actions.push({
        key: 'unschedule',
        label: t('calendar.unschedule'),
        onSelect: () => unschedule.mutate(release.id),
      })
    }
  }
  if (url !== null) {
    actions.push({ key: 'copy', label: t('releases.copyLink'), onSelect: () => copyLink(url) })
  }
  actions.push({
    key: 'delete',
    label: t('releases.delete'),
    danger: true,
    onSelect: () => remove.mutate(release.id),
  })

  return (
    <div className="-mx-3 -mb-2.5 flex flex-col">
      <div className="flex min-w-0 flex-wrap items-center gap-x-2.5 gap-y-1 px-3 pb-2 text-sm">
        <KindGlyph icon={door?.icon} className="size-3.5 shrink-0" />
        <span className="min-w-0 truncate font-medium">{labelOf(doors, release.kind)}</span>
        <span
          className={cn(
            'shrink-0 text-xs',
            released
              ? 'text-good'
              : release.scheduled_at === null
                ? 'text-faint'
                : 'font-mono tabular-nums text-dim',
          )}
        >
          {released
            ? t('releases.releasedOn', {
                date: release.released_at === null ? '' : formatDay(release.released_at),
              })
            : release.scheduled_at === null
              ? t('releases.unscheduled')
              : formatDay(release.scheduled_at)}
        </span>
        {writing ? (
          <span className="flex shrink-0 items-center gap-1 text-xs text-info">
            <LoaderCircle aria-hidden className="size-3 animate-spin" />
            {t('releases.task.writingMeta')}
          </span>
        ) : (
          waiting > 0 && (
            <Badge variant="accent" className="shrink-0">
              {t('releases.proposals.waiting', { count: waiting })}
            </Badge>
          )
        )}
        <span className="ml-auto flex shrink-0 items-center gap-1.5">
          <ReleaseReadiness release={release} today={now} />
          {url !== null && (
            <Button
              variant="link"
              onClick={() => void openExternal(url)}
              title={url}
              className="min-w-0 shrink text-xs"
            >
              <span className="max-w-48 truncate">{shortLink(url)}</span>
              <ExternalLink aria-hidden />
            </Button>
          )}
          <RowMenu actions={actions} label={t('releases.actions')} />
        </span>
      </div>
      <ReleaseDetails release={release} refreshed={refreshed} />

      <MarkReleasedDialog
        release={marking}
        today={now}
        onOpenChange={(open) => {
          if (!open) setMarking(null)
        }}
        onConfirm={(id, link, at) => markOut.mutate({ id, url: link, at })}
      />
    </div>
  )
}
