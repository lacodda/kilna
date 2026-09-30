import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ChevronRight, ExternalLink, LoaderCircle } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { openExternal, shortLink } from '@/lib/link'
import { queries } from '@/lib/query/queries'
import { KindGlyph } from '@/lib/releaseIcon'
import { labelOf, useProfile, vocabularyOf } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { RowButton } from '@/components/ui/list-row'
import { RowContextMenu, RowMenu, type RowAction } from '@/components/RowMenu'
import { ReleaseDetails } from '@/features/work/tabs/releases/ReleaseDetails'
import { ReleaseReadiness } from '@/features/work/tabs/releases/ReleaseReadiness'
import { useReleaseMeta } from '@/features/work/tabs/releases/releaseMeta'

interface Props {
  release: ScheduledRelease
  /** Unrolled: its form, its fields and its files under the line. */
  open: boolean
  onToggle: () => void
  /** What the row's menu and a right click on it offer - one list for both. */
  actions: RowAction[]
  today: string
  /** The query areas an edit of this release disturbs. */
  refreshed: readonly (readonly unknown[])[]
}

/**
 * One release in its work's list: a line, and under it - when it is open -
 * everything about the release (`ReleaseDetails`).
 *
 * A line of the list rather than a card of its own, as the mockup draws it:
 * rows divided by a hairline inside one panel. The kind is the row's name
 * and the whole of it opens the release, not only the arrow; what is
 * measured about it - readiness, the day, the link - stands at the end.
 */
export function ReleaseRow({ release, open, onToggle, actions, today, refreshed }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const kinds = vocabularyOf(profile.config, release.work_kind).release_kinds
  const kind = kinds.find((entry) => entry.key === release.kind)

  const released = release.status === 'released'
  const url = release.url

  // Said on the line, so a folded release still tells that its meta is being
  // written, or that some of it waits to be taken.
  const meta = useReleaseMeta(release.work_kind)
  const writing = meta.writing(release.id)
  const proposals = useQuery(queries.releaseProposals(release.id))
  const waiting = (proposals.data ?? []).reduce((sum, one) => sum + one.fields.length, 0)

  return (
    <li className="border-b border-line last:border-b-0">
      <RowContextMenu
        actions={actions}
        render={
          <div
            className={cn(
              'flex min-w-0 items-center gap-2.5 px-3 py-2 text-sm',
              // What went out is history sitting in the list, not a plan
              // competing for attention - the same dimming the chip uses.
              released && !open && 'opacity-70',
              // Lit while its own menu is open, so it is clear which release
              // the actions belong to.
              'data-[popup-open]:bg-soft',
            )}
          />
        }
      >
        <RowButton
          onClick={onToggle}
          aria-expanded={open}
          start={
            <>
              <ChevronRight
                aria-hidden
                className={cn(
                  'mr-1.5 size-3.5 transition-transform',
                  open && 'rotate-90 text-accent',
                )}
              />
              <KindGlyph icon={kind?.icon} className="size-3.5" />
            </>
          }
          // Pulled into the row's padding, so the row stays the height of its
          // words rather than of the button.
          className="-my-1 -ml-1.5 w-auto flex-1 px-1.5 py-1"
        >
          {labelOf(kinds, release.kind)}
        </RowButton>

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

        <ReleaseReadiness release={release} today={today} />

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

        {url !== null && (
          <Button
            variant="link"
            onClick={() => void openExternal(url)}
            title={url}
            className="min-w-0 shrink text-xs"
          >
            <span className="truncate">{shortLink(url)}</span>
            <ExternalLink aria-hidden />
          </Button>
        )}

        <RowMenu actions={actions} label={t('releases.actions')} />
      </RowContextMenu>

      {open && <ReleaseDetails release={release} refreshed={refreshed} />}
    </li>
  )
}
