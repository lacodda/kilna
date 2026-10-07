import { useEffect, useRef, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import type { VersionSummary } from '@/lib/api/types'
import { childrenIn, neighbour, parentIn } from '@/lib/history'
import type { VersionStats } from '@/lib/versionStats'
import type { Graph } from '@/lib/versionTree'
import { Button } from '@/components/ui/button'
import { RowButton } from '@/components/ui/list-row'
import { RowMenu } from '@/components/RowMenu'
import { GraphCell } from '@/features/work/tabs/versions/GraphCell'
import { cn } from '@/lib/utils'
import { formatDay } from '@/lib/format'

interface Props {
  versions: VersionSummary[]
  /** Which version was written from which, drawn beside the rows; `null`
   *  when the role records no lineage (`lib/versionTree`). */
  graph?: Graph | null
  /** What each row says about its text, by version id, for the ones whose
   *  text has come (`useLaneStats`). */
  stats?: Record<string, VersionStats>
  /** What the list says when the role holds nothing yet. */
  empty: ReactNode
  /** Whether these are versions of commentary. A review is written about the
   * text by an action run on it: it is never the work's current version, and
   * a copy of one would be about no revision at all. */
  commentary: boolean
  /** The score a version was given, by version id, for the ones that were
   * judged. A version scored twice shows the score that speaks for it. */
  /** The total that speaks for a version, already said as a total. */
  scores?: Record<string, string>
  openId: string | null
  /** Which version the open one is being compared against, if any. */
  comparedId: string | null
  onOpen: (id: string) => void
  onCompare: (id: string) => void
  onMakeCurrent: (id: string) => void
  onDelete: (id: string) => void
  /** Start a new draft from this version's body. */
  onDeriveFrom: (id: string) => void
}

/**
 * Every version of one role, newest first, with the tree of which was written
 * from which drawn beside it (ADR 0055).
 *
 * A row is a button rather than a link: which draft is open is a view state of
 * this tab, not an address of its own. Only the tab itself is addressable.
 *
 * The arrows walk the history, so reading through six revisions is six presses
 * rather than six aimed clicks: up and down along the list, left to the
 * version the open one was written from, right to the newest one written from
 * it. The list is a listbox for that reason: the open row is the selected
 * option, and only it is in the tab order.
 */
export function VersionList({
  versions,
  graph = null,
  stats,
  empty,
  commentary,
  scores,
  openId,
  comparedId,
  onOpen,
  onCompare,
  onMakeCurrent,
  onDelete,
  onDeriveFrom,
}: Props) {
  const { t } = useTranslation()
  const list = useRef<HTMLUListElement>(null)

  // Arrowing moves the open version, and the focus has to follow it or the next
  // press comes from where the finger was rather than from what is on screen.
  //
  // Only when a row itself held the focus. Stepping from the editor must not
  // yank the cursor out of the text, and arrowing while the ± or delete button
  // of a row is focused must not drag the focus off that button either — the
  // keystroke bubbles up from there, but it was not aimed at the row.
  useEffect(() => {
    const from = document.activeElement
    if (openId === null || !(from instanceof HTMLElement) || from.dataset.version === undefined) {
      return
    }
    const row = list.current?.querySelector<HTMLElement>(`[data-version="${openId}"]`)
    row?.focus()
  }, [openId])

  // The open version shows itself in the list. A version minted by typing, or
  // saved from the form, is opened by the panel — and a row that is open but
  // scrolled out of the list is a version nobody can see was made.
  useEffect(() => {
    if (openId === null) return
    const row = list.current?.querySelector<HTMLElement>(`[data-version="${openId}"]`)
    row?.scrollIntoView({ block: 'nearest' })
  }, [openId, versions.length])

  const step = (direction: -1 | 1) => {
    const next = neighbour(versions, openId, direction)
    if (next !== null) onOpen(next.id)
  }

  const onKeyDown = (event: React.KeyboardEvent) => {
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        step(1)
        break
      case 'ArrowUp':
        event.preventDefault()
        step(-1)
        break
      case 'Home':
        event.preventDefault()
        if (versions[0] !== undefined) onOpen(versions[0].id)
        break
      case 'End':
        event.preventDefault()
        if (versions.at(-1) !== undefined) onOpen(versions.at(-1)!.id)
        break
      // Along the tree rather than the list: where the open version came
      // from, and the newest version that came from it.
      case 'ArrowLeft': {
        event.preventDefault()
        const parent = parentIn(versions, openId)
        if (parent !== null) onOpen(parent.id)
        break
      }
      case 'ArrowRight': {
        event.preventDefault()
        const child = childrenIn(versions, openId)[0]
        if (child !== undefined) onOpen(child.id)
        break
      }
      default:
        break
    }
  }

  const nameOf = (version: VersionSummary) =>
    version.label ?? t('versions.revision', { number: version.revision })

  if (versions.length === 0) return empty

  return (
    <ul
      ref={list}
      role="listbox"
      aria-label={t('versions.title')}
      onKeyDown={onKeyDown}
      className="flex flex-col gap-1"
    >
      {versions.map((version, index) => {
        const isOpen = version.id === openId
        const isCompared = version.id === comparedId
        const score = scores?.[version.id]
        const row = graph?.rows[index]
        const parent = parentIn(versions, version.id)
        const figures = stats?.[version.id]
        const day = formatDay(version.created_at)
        const size =
          figures === undefined
            ? t('versions.length', { count: version.length })
            : `${t('versions.words', { count: figures.words })} · ${t('versions.lines', { count: figures.lines })}`
        const change = figures?.change ?? null
        // The whole row in words, for the hover: the list is narrow, and its
        // second line gives up its tail before its head.
        const said = [
          nameOf(version),
          day,
          change === null ? null : `+${change.added} −${change.removed}`,
          size,
        ]
          .filter((part) => part !== null)
          .join(' · ')

        return (
          <li key={version.id} className="group flex items-center gap-1">
            {graph !== null && row !== undefined && (
              <GraphCell
                row={row}
                columns={graph.columns}
                current={version.is_current}
                title={
                  parent === null
                    ? undefined
                    : t('versions.writtenFrom', { name: `v${parent.revision}` })
                }
              />
            )}
            <RowButton
              role="option"
              selected={isOpen}
              aria-selected={isOpen}
              // In a listbox the open row is the selected option; `aria-current`
              // on top of it would announce the same fact twice.
              aria-current={undefined}
              data-version={version.id}
              // Roving focus: one stop for the whole history, and the arrows do
              // the rest. Tabbing past twenty revisions is not navigation.
              tabIndex={isOpen ? 0 : -1}
              onClick={() => onOpen(version.id)}
              // The version beside the open one keeps a quiet tint, so the pair
              // being compared reads as a pair in the list too.
              className={cn('flex-1', !isOpen && isCompared && 'bg-soft')}
              title={said}
              start={<span className="font-mono text-xs text-faint">v{version.revision}</span>}
              // In mono, as the mockup sets it: figures in a column line up.
              // First how many lines it moved since the version it is
              // compared with - the comparison's own count, and the figure a
              // history is read for - then how long it is, in words and lines
              // once the text has come, in characters until then. The list
              // is narrow, and a line that truncates loses its tail, so the
              // date of a named version comes last.
              description={
                <span className="font-mono">
                  {change !== null && (
                    <>
                      <span className="text-good">+{change.added}</span>{' '}
                      <span className="text-bad">−{change.removed}</span>
                      {' · '}
                    </>
                  )}
                  {size}
                  {version.label !== null && ` · ${day}`}
                </span>
              }
              end={
                <>
                  {version.is_current && (
                    <span className="rounded bg-accent-soft px-1 text-2xs font-semibold uppercase tracking-caption text-accent-2">
                      {t('versions.current')}
                    </span>
                  )}
                  {/* The score of the version that was judged, beside the version
                      it judged. Reading the list was the one place the two could
                      not be seen together: the history said which drafts exist
                      and the Score tab said how they did, and matching one to the
                      other meant remembering a revision number across a tab. */}
                  {score !== undefined && (
                    <span
                      className="rounded bg-soft px-1 font-mono text-2xs font-semibold text-text"
                      title={t('versions.scored', { score })}
                    >
                      {score}
                    </span>
                  )}
                </>
              }
            >
              {/* The revision's number is at the start already; an unnamed
                  one is told apart by when it was written. */}
              {version.label ?? day}
            </RowButton>

            {/* Only shown for versions other than the open one: comparing a
                draft with itself is not a thing anyone means to do.

                Kept out of the menu below, unlike the other three. Comparing is
                how the list is READ - a glance between two drafts, often several
                in a row - while the others change what the history holds, and a
                row that carried all four of them left no width for the name of
                the version. */}
            {!isOpen && (
              <Button
                variant={isCompared ? 'soft' : 'icon'}
                size="icon-sm"
                onClick={() => onCompare(version.id)}
                title={isCompared ? t('versions.stopComparing') : t('versions.compare')}
                aria-label={isCompared ? t('versions.stopComparing') : t('versions.compare')}
              >
                <span aria-hidden className="font-mono text-xs">
                  ±
                </span>
              </Button>
            )}

            {/* The three that change the history, behind one button. Four
                controls per row pushed every label into an ellipsis at the
                width the list is given - and "make current" and "delete" sat
                one pixel apart, which is not where a delete belongs. */}
            <RowMenu
              label={nameOf(version)}
              actions={[
                // A scored version never changes - revising it means starting
                // the next revision from it, which then remembers it as its
                // parent (ADR 0002, ADR 0055).
                ...(commentary
                  ? []
                  : [
                      {
                        key: 'derive',
                        label: t('versions.deriveFrom', { name: `v${version.revision}` }),
                        onSelect: () => onDeriveFrom(version.id),
                      },
                    ]),
                ...(version.is_current || commentary
                  ? []
                  : [
                      {
                        key: 'current',
                        label: t('versions.makeCurrent'),
                        onSelect: () => onMakeCurrent(version.id),
                      },
                    ]),
                {
                  key: 'delete',
                  label: t('versions.delete'),
                  danger: true,
                  onSelect: () => onDelete(version.id),
                },
              ]}
            />
          </li>
        )
      })}
    </ul>
  )
}
