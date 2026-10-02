import { useTranslation } from 'react-i18next'
import { Check, Pin } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api/types'
import type { CatalogueRow, ColumnId } from '@/lib/catalogue'
import { coverImageFor } from '@/lib/cover'
import { formatDay, formatDelta, formatNumber } from '@/lib/format'
import { badgeVariantOf } from '@/lib/markIcon'
import { today } from '@/lib/month'
import { daysBetween, missing, urgency } from '@/lib/readiness'
import { nextTier } from '@/lib/scoring'
import { useCovers } from '@/lib/useCovers'
import { allOf, labelOf, say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { TableCell } from '@/components/ui/table'
import { MarkAvatars } from '@/components/MarkAvatars'
import { StagePicker } from '@/components/StagePicker'
import { WorkRepeatMark } from '@/components/RepeatMark'
import { RowStar } from './RowStar'
import { STUCK_LEFT_TITLE, TITLE_HOLD } from './columns'

interface Props {
  column: ColumnId
  row: CatalogueRow
  /** The table shows one kind of work: naming it on every row says nothing. */
  kindNarrowed: boolean
  /** The Status column is drawn, so the title need not say it again. */
  statusShown: boolean
  /** Whether the title is being held against the left edge with its heading. */
  titleStuck?: boolean
}

/** One cell, drawn from the column that asked for it. */
export function CatalogueCell({
  column,
  row,
  kindNarrowed,
  statusShown,
  titleStuck = false,
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const vocabulary = vocabularyOf(profile.config, row.kind)
  const covers = useCovers()

  switch (column) {
    case 'id':
      // Monospaced and dimmed: it is here to be copied and compared, not read
      // as part of the sentence a row makes.
      return (
        <TableCell className="truncate font-mono text-xs text-faint">
          {row.work_id.slice(0, 8)}
        </TableCell>
      )

    case 'title':
      return (
        // The title and what it is stay on one line. Squeezed, a two-word title
        // broke mid-phrase and the row grew to three lines; the table now
        // scrolls sideways instead of folding - and a column dragged narrow
        // clips the title with an ellipsis rather than wrapping it.
        //
        // It is also held against the left edge, right after the tick, so a row
        // scrolled sideways can still be named. See STUCK_LEFT_TITLE.
        <TableCell
          className={cn('overflow-hidden whitespace-nowrap', titleStuck && STUCK_LEFT_TITLE)}
          style={titleStuck ? TITLE_HOLD : undefined}
        >
          <span className="inline-flex max-w-full items-center gap-2">
            <RowStar row={row} />
            {/* The cover as a chip beside the title rather than a column of
                its own: a column of pictures costs every row its height,
                and what the catalogue is read down is titles. */}
            <span
              aria-hidden
              className="size-5 shrink-0 rounded-sm border border-line/60"
              style={{ background: coverImageFor(row.work_id, covers.get(row.work_id)) }}
            />
            <span className="min-w-0 truncate font-semibold" title={row.title}>
              {row.title}
            </span>
            {/* The guard's mark right after the name it is about (ADR 0054):
                a catalogue is read down its titles when a song is being
                picked for the next slot, and that is where a repeat has to
                be seen. */}
            <WorkRepeatMark workId={row.work_id} />
            {/* Where it stands is said once. The Status column says it since
                v0.79; a table whose columns were chosen before that column
                existed has no Status column, and the title keeps saying it
                there rather than the rows going quiet about it. */}
            {!statusShown && <StatusBadge row={row} />}
            {/* What it is, as an aside to the title rather than a second badge:
                the mockup's "· instrumental". */}
            {!kindNarrowed && (
              <span className="shrink-0 text-xs text-faint">
                {'· '}
                {labelOf(profile.config.work_kinds, row.kind)}
              </span>
            )}
          </span>
        </TableCell>
      )

    case 'marks': {
      // A mark the profile no longer defines is not drawn - the same rule the
      // card follows, so the two screens never disagree about what a work says.
      // A profile written before marks existed has none at all, and every mark
      // on every work is then unknown, which is the correct reading.
      // In the profile's order, the order the card draws them in.
      const shown = (profile.config.marks ?? []).filter((mark) => row.marks.includes(mark.key))
      return (
        <TableCell className="overflow-hidden whitespace-nowrap">
          {shown.length === 0 ? (
            <span className="text-faint">{'—'}</span>
          ) : (
            <MarkAvatars marks={shown} className="align-middle" />
          )}
        </TableCell>
      )
    }

    case 'status':
      return (
        <TableCell className="whitespace-nowrap">
          <StatusBadge row={row} />
        </TableCell>
      )

    case 'stage':
      return (
        // The dial and nothing else: a word per row would be a second column
        // of text beside the title, and the whole point of a dial is that a
        // column of them is read at a glance. The word is in the tooltip.
        <TableCell onClick={(event) => event.stopPropagation()}>
          <StagePicker workId={row.work_id} percent={row.stage} compact />
        </TableCell>
      )

    case 'versions':
      return (
        <TableCell numeric className={cn('font-mono', row.version_count === 0 && 'text-faint')}>
          {row.version_count}
        </TableCell>
      )

    case 'tier': {
      // How far the next tier is, when the row has been judged at all. Only
      // the distance: which axis is cheapest needs the axis values, and a
      // catalogue row carries the total, not the score behind it. Naming the
      // axis here would mean shipping every work's axes to draw a table.
      const ahead =
        row.total === null || row.tier_pinned ? undefined : nextTier(vocabulary.tiers, row.total)

      return (
        <TableCell className="overflow-hidden whitespace-nowrap">
          {row.tier === null ? (
            <span className="text-faint">{'—'}</span>
          ) : (
            <span className="flex items-center gap-1.5">
              <Badge variant="accent" className="px-2">
                {labelOf(vocabulary.tiers, row.tier)}
              </Badge>
              {/* A tier held by hand is not a tier the score arrived at, and
                  a reader who cannot tell them apart is reading a number that
                  means two different things. */}
              {row.tier_pinned && (
                <Pin
                  aria-label={t('catalogue.tierPinned')}
                  className="size-3 shrink-0 text-faint"
                />
              )}
              {ahead !== undefined && row.total !== null && (
                <span
                  className="text-xs text-faint tabular-nums"
                  title={t('catalogue.toNextTier', {
                    gap: formatNumber(ahead.min - row.total),
                    tier: ahead.label,
                  })}
                >
                  {formatDelta(ahead.min - row.total)}
                </span>
              )}
            </span>
          )}
        </TableCell>
      )
    }

    case 'total':
      return (
        // The score, and whether it still describes the work: "changed since"
        // sits beside the number it doubts, as the mockup has it, rather than
        // beside the date in a column that is off by default. Before the
        // number rather than after it, so the numbers of a right-aligned
        // column still line up.
        <TableCell
          numeric
          className={cn('whitespace-nowrap font-mono', row.total === null && 'text-faint')}
        >
          {row.stale && (
            <Badge
              variant="warn"
              className="mr-1.5 px-1.5 font-sans"
              title={t('catalogue.staleHint')}
            >
              {t('catalogue.stale')}
            </Badge>
          )}
          {row.total === null ? '—' : formatNumber(row.total)}
        </TableCell>
      )

    case 'release':
      return <ReleaseCell release={row.next_release} />

    case 'ready':
      return <ReadyCell release={row.next_release} />

    case 'scored':
      return (
        // A date is one word. Left to wrap it broke in two, which reads as two
        // dates rather than as one.
        <TableCell className="truncate text-xs text-dim">
          {row.scored_at === null ? '—' : formatDay(row.scored_at)}
        </TableCell>
      )

    case 'created':
      return (
        <TableCell className="truncate text-xs text-dim">{formatDay(row.created_at)}</TableCell>
      )

    case 'updated':
      return (
        <TableCell className="truncate text-xs text-dim">{formatDay(row.updated_at)}</TableCell>
      )
  }
}

/** Where a work stands, as a badge in the status's own colour. A status the
 * profile has since dropped is named by its bare key rather than vanishing. */
function StatusBadge({ row }: { row: CatalogueRow }) {
  const profile = useProfile()
  const status = vocabularyOf(profile.config, row.kind).statuses.find((s) => s.key === row.status)
  return (
    <Badge variant={badgeVariantOf(status?.colour)} className="shrink-0 px-2">
      {status === undefined ? row.status : sayLabel(status.label)}
    </Badge>
  )
}

/** The day the work goes out next, and on what - the kind is in the tooltip,
 * since the column is read for the date. */
function ReleaseCell({ release }: { release: ScheduledRelease | null }) {
  const { t } = useTranslation()
  const profile = useProfile()
  if (release === null || release.scheduled_at === null) {
    return <TableCell className="font-mono text-faint">{'—'}</TableCell>
  }
  const date = formatDay(release.scheduled_at)
  return (
    <TableCell
      className="whitespace-nowrap font-mono text-xs tabular-nums"
      title={t('catalogue.releaseHint', {
        kind: labelOf(allOf(profile.config, 'release_kinds'), release.kind),
        date,
      })}
    >
      {date}
    </TableCell>
  )
}

// The colour belongs to the deadline, not to the gap, as on the calendar's
// chip: the same missing cover is a note a month out and a red flag two days
// before the slot.
const GAP_TONE: Record<ReturnType<typeof urgency>, BadgeProps['variant']> = {
  calm: 'outline',
  soon: 'warn',
  urgent: 'bad',
}

/**
 * Whether the next release can go out, in words: a tick when it can, what it
 * still lacks when it cannot. The verdict is the backend's, the same one the
 * calendar's chip reads, so the two screens cannot disagree. Nothing booked is
 * an empty cell - there is no release to be ready.
 */
function ReadyCell({ release }: { release: ScheduledRelease | null }) {
  const { t } = useTranslation()
  const profile = useProfile()
  if (release === null || release.scheduled_at === null) return <TableCell />

  if (release.readiness.ready) {
    return (
      <TableCell className="text-good" title={t('calendar.ready')}>
        <Check aria-hidden className="size-3.5" />
        <span className="sr-only">{t('calendar.ready')}</span>
      </TableCell>
    )
  }

  const names = missing(release.readiness).map((gap) =>
    gap === 'score'
      ? t('calendar.missingScore')
      : labelOf(allOf(profile.config, 'version_roles'), gap),
  )
  const tone = GAP_TONE[urgency(daysBetween(today(), release.scheduled_at))]
  return (
    <TableCell className="whitespace-nowrap">
      <Badge
        variant={tone}
        className="px-2"
        title={t('calendar.notReadyHint', { list: names.join(', ') })}
      >
        {names.join(', ')}
      </Badge>
    </TableCell>
  )
}
