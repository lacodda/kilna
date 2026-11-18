import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { Copyable } from '@/components/ui/copyable'
import { StatusDot } from '@/components/ui/status-dot'
import { useQuery } from '@tanstack/react-query'
import { queries } from '@/lib/query/queries'
import { MarkMenuItems } from '@/components/MarkMenuItems'
import { updateWork } from '@/lib/api/works'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useTranslation } from 'react-i18next'
import { Pin, RefreshCw } from 'lucide-react'
import type { ScoredWork } from '@/lib/api/types'
import { tierRanges, type CatalogueRow, type ColumnId } from '@/lib/catalogue'
import { coverImageFor } from '@/lib/cover'
import { formatDay, formatNumber } from '@/lib/format'
import { badgeVariantOf } from '@/lib/markIcon'
import { today } from '@/lib/month'
import { daysBetween, missing, sayNeed, urgency } from '@/lib/readiness'
import { nextTier } from '@/lib/scoring'
import { useCovers } from '@/lib/useCovers'
import { allOf, labelOf, say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { TableCell } from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import { MarkAvatars } from '@/components/MarkAvatars'
import { StagePicker } from '@/components/StagePicker'
import { VibePicker } from '@/components/VibePicker'
import { WorkRepeatMark } from '@/components/RepeatMark'
import { RowStar } from './RowStar'
import { STUCK_LEFT_TITLE, TITLE_HOLD } from './columns'

interface Props {
  column: ColumnId
  row: CatalogueRow
  /** Every row by its work, for what an original's derived works are. */
  everyRow?: ReadonlyMap<string, CatalogueRow>
  /** Open another work: a derived one, from its original's row. */
  onOpen?: (workId: string) => void
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
  everyRow,
  onOpen,
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
      // as part of the sentence a row makes. A press copies the whole id
      // (v0.140) and stays in the cell - it would open the work otherwise.
      return (
        <TableCell className="truncate" onClick={kept} onKeyDown={kept}>
          <Copyable
            value={row.work_id}
            label={t('catalogue.copyId')}
            copiedLabel={t('catalogue.idCopied')}
            title={row.work_id}
            className="text-faint"
          >
            {row.work_id.slice(0, 8)}
          </Copyable>
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

    case 'marks':
      return (
        <TableCell className="overflow-hidden whitespace-nowrap">
          <MarksCell row={row} />
        </TableCell>
      )

    case 'status':
      return (
        <TableCell className="whitespace-nowrap">
          <StatusBadge row={row} />
        </TableCell>
      )

    case 'vibe':
      return (
        // The mark and nothing else, set where it stands: its three words
        // are what it says when it is pointed at. A row with none shows a
        // quiet way in.
        <TableCell onClick={(event) => event.stopPropagation()}>
          <VibePicker
            workId={row.work_id}
            // A row's own vibe is the one it shows when it is not taken
            // from another work.
            own={row.vibe === undefined || row.vibe.from !== undefined ? undefined : row.vibe.vibe}
            shown={row.vibe}
            compact
          />
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
      // A kind that keeps no versions and holds none says nothing, quietly.
      return (
        <TableCell numeric className={cn('font-mono', row.version_count === 0 && 'text-faint')}>
          {row.version_count === 0 && vocabulary.version_roles.length === 0
            ? '—'
            : row.version_count}
        </TableCell>
      )

    case 'total':
      return <TotalCell row={row} />

    case 'release':
      return <ReleaseCell row={row} />

    case 'collection':
      return <CollectionCell row={row} />

    case 'tags':
      return <TagsCell row={row} />

    case 'derived':
      return <DerivedCell row={row} everyRow={everyRow} onOpen={onOpen} />

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
function ReleaseCell({ row }: { row: CatalogueRow }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const release = row.next_release
  if (release === null || release.scheduled_at === null) {
    // Nothing booked: the planned slot, dashed as on the calendar (v0.140).
    if (row.plan != null) {
      const day = formatDay(row.plan.scheduled_at)
      return (
        <TableCell
          className="whitespace-nowrap font-mono text-xs tabular-nums text-dim"
          title={t('catalogue.planHint', { date: day })}
        >
          <span className="rounded-sm border border-dashed border-line-2 px-1">{day}</span>
          <span className="ml-1 font-sans text-faint">{t('catalogue.plan')}</span>
        </TableCell>
      )
    }
    return <TableCell className="font-mono text-faint">{'—'}</TableCell>
  }
  const date = formatDay(release.scheduled_at)
  // Whether it can go out, as a dot before the day (v0.140): green when it
  // can, and what it still lacks in the dot's name and on hover - amber while
  // there is time, red once the day is close. The verdict is the backend's,
  // the one the calendar's chip reads.
  const names = missing(release.readiness).map((gap) => sayNeed(gap, profile.config, release, t))
  const close = urgency(daysBetween(today(), release.scheduled_at)) === 'urgent'
  const said = release.readiness.ready
    ? t('calendar.ready')
    : t('calendar.notReadyHint', { list: names.join(', ') })
  return (
    <TableCell
      className="whitespace-nowrap font-mono text-xs tabular-nums"
      title={`${t('catalogue.releaseHint', {
        kind: labelOf(allOf(profile.config, 'release_kinds'), release.kind),
        date,
      })} · ${said}`}
    >
      <span className="inline-flex items-center gap-1.5">
        <StatusDot
          size="sm"
          status={release.readiness.ready ? 'good' : close ? 'bad' : 'warn'}
          label={said}
          showLabel={false}
        />
        {date}
      </span>
    </TableCell>
  )
}

/**
 * The score and the tier it earned (v0.140): the tier's letter by rank beside
 * the number, a turned arrow where the work was rewritten after its score,
 * a pin where the tier is held by hand. The tier's word, the day it was
 * scored and the distance to the next tier are on hover.
 */
function TotalCell({ row }: { row: CatalogueRow }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const tiers = vocabularyOf(profile.config, row.kind).tiers
  const ranked = tierRanges(tiers).find((one) => one.tier.key === row.tier)
  // How far the next tier is, when the row has been judged at all.
  const ahead = row.total === null || row.tier_pinned ? undefined : nextTier(tiers, row.total)
  const hint = [
    ranked === undefined ? undefined : `${ranked.letter} · ${sayLabel(ranked.tier.label)}`,
    row.scored_at === null
      ? undefined
      : t('catalogue.scoredOn', { date: formatDay(row.scored_at) }),
    ahead === undefined || row.total === null
      ? undefined
      : t('catalogue.toNextTier', {
          gap: formatNumber(ahead.min - row.total),
          tier: sayLabel(ahead.label),
        }),
    row.stale ? t('catalogue.staleHint') : undefined,
  ].filter((part) => part !== undefined)
  return (
    <TableCell
      numeric
      className={cn('whitespace-nowrap font-mono', row.total === null && 'text-faint')}
      title={hint.join(' · ') || undefined}
    >
      <span className="inline-flex items-center justify-end gap-1.5">
        {row.stale && (
          <RefreshCw aria-label={t('catalogue.staleHint')} className="size-3 shrink-0 text-warn" />
        )}
        {row.tier_pinned && (
          <Pin aria-label={t('catalogue.tierPinned')} className="size-3 shrink-0 text-faint" />
        )}
        {ranked !== undefined && (
          <Badge
            variant="accent"
            className="px-1.5 font-sans"
            aria-label={sayLabel(ranked.tier.label)}
          >
            {ranked.letter}
          </Badge>
        )}
        <span>{row.total === null ? '—' : formatNumber(row.total)}</span>
      </span>
    </TableCell>
  )
}

/** The collection a work is in, by its name (v0.140). */
function CollectionCell({ row }: { row: CatalogueRow }) {
  const collections = useQuery(queries.collections())
  const held = collections.data?.find((one) => one.id === row.collection_id)
  return (
    <TableCell className="truncate text-sm" title={held?.title}>
      {held === undefined ? <span className="text-faint">{'—'}</span> : held.title}
    </TableCell>
  )
}

/** How many tags a row shows before it says how many more there are. */
const TAGS_SHOWN = 2

/** The work's tags, the first two and how many more (v0.140). */
function TagsCell({ row }: { row: CatalogueRow }) {
  const { t } = useTranslation()
  if (row.tags.length === 0) {
    return <TableCell className="text-faint">{'—'}</TableCell>
  }
  const rest = row.tags.length - TAGS_SHOWN
  return (
    <TableCell className="overflow-hidden whitespace-nowrap" title={row.tags.join(', ')}>
      <span className="inline-flex items-center gap-1">
        {row.tags.slice(0, TAGS_SHOWN).map((tag) => (
          <Badge key={tag} variant="soft" className="px-1.5">
            {tag}
          </Badge>
        ))}
        {rest > 0 && (
          <span
            className="text-xs text-faint"
            aria-label={t('catalogue.moreTags', { count: rest })}
          >
            {`+${rest}`}
          </span>
        )}
      </span>
    </TableCell>
  )
}

/**
 * What was made from an original (v0.140): one chip a kind - its audio, its
 * clips, its shorts - green once all of them are out, blue while one is
 * booked. A press opens the work, or a menu of them when there are several;
 * either way it stays in the cell.
 */
function DerivedCell({
  row,
  everyRow,
  onOpen,
}: {
  row: CatalogueRow
  everyRow?: ReadonlyMap<string, CatalogueRow>
  onOpen?: (workId: string) => void
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  const made = (row.publications ?? [])
    .map((id) => everyRow?.get(id))
    .filter((one): one is CatalogueRow => one !== undefined)
  if (made.length === 0) return <TableCell className="text-faint">{'—'}</TableCell>
  const kinds = [...new Set(made.map((one) => one.kind))]
  return (
    <TableCell className="overflow-hidden whitespace-nowrap" onClick={kept} onKeyDown={kept}>
      <span className="inline-flex items-center gap-1">
        {kinds.map((kind) => {
          const ofKind = made.filter((one) => one.kind === kind)
          const out = ofKind.every((one) => one.released > 0)
          const booked = ofKind.some((one) => one.next_release !== null)
          const word = labelOf(profile.config.work_kinds, kind)
          const chip = (
            <Badge variant={out ? 'good' : booked ? 'info' : 'outline'} className="px-1.5">
              {ofKind.length > 1 ? `${word} ${ofKind.length}` : word}
            </Badge>
          )
          const only = ofKind[0]
          if (ofKind.length === 1 && only !== undefined) {
            return (
              <Button
                key={kind}
                variant="ghost"
                size="xs"
                className="h-auto px-0"
                aria-label={t('catalogue.openMade', { title: only.title })}
                title={only.title}
                onClick={() => onOpen?.(only.work_id)}
              >
                {chip}
              </Button>
            )
          }
          return (
            <Menu key={kind}>
              <MenuTrigger
                render={
                  <Button
                    variant="ghost"
                    size="xs"
                    className="h-auto px-0"
                    aria-label={t('catalogue.madeOfKind', { kind: word, count: ofKind.length })}
                  />
                }
              >
                {chip}
              </MenuTrigger>
              <MenuPopup align="start" onClick={kept} onKeyDown={kept}>
                {ofKind.map((one) => (
                  <MenuItem key={one.work_id} onClick={() => onOpen?.(one.work_id)}>
                    {one.title}
                  </MenuItem>
                ))}
              </MenuPopup>
            </Menu>
          )
        })}
      </span>
    </TableCell>
  )
}

/** A press inside a cell that acts on its own: it does not open the row. */
const kept = (event: { stopPropagation: () => void }) => event.stopPropagation()

/**
 * The marks of a row, raised and lowered where they are shown (v0.139,
 * FR-12 12.1): the cell opens the card's menu over itself, empty or not.
 * Its presses stay in the cell - the menu is drawn elsewhere on the page but
 * is the row's child, and a press bubbling up would open the work.
 *
 * A mark the profile no longer defines is not drawn - the same rule the card
 * follows, so the two screens never disagree about what a work says - and
 * stays on the work, untouched, when another is raised.
 */
function MarksCell({ row }: { row: ScoredWork }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const marks = profile.config.marks ?? []
  const shown = marks.filter((mark) => row.marks.includes(mark.key))
  const save = useAppMutation({
    mutationFn: (next: string[]) => updateWork(row.work_id, { marks: next }),
    failure: 'toast.workSaveFailed',
    refresh: [keys.catalogue, keys.work(row.work_id)],
  })
  if (marks.length === 0) return <span className="text-faint">{'—'}</span>
  return (
    <Menu>
      <MenuTrigger
        onClick={kept}
        onKeyDown={kept}
        render={
          <Button
            variant="icon"
            size="xs"
            aria-label={
              shown.length === 0
                ? t('work.addMark')
                : t('work.marksRaised', {
                    marks: shown.map((mark) => sayLabel(mark.label)).join(', '),
                  })
            }
            className="rounded-full px-1"
          />
        }
      >
        {shown.length === 0 ? (
          <span className="text-faint">{'—'}</span>
        ) : (
          <MarkAvatars marks={shown} className="align-middle" />
        )}
      </MenuTrigger>
      <MenuPopup align="start" onClick={kept} onKeyDown={kept}>
        <MarkMenuItems
          marks={marks}
          stateOf={(key) => row.marks.includes(key)}
          onToggle={(key, next) =>
            save.mutate(next ? [...row.marks, key] : row.marks.filter((one) => one !== key))
          }
        />
      </MenuPopup>
    </Menu>
  )
}
