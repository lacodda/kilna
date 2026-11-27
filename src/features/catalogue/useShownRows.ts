import { useQuery } from '@tanstack/react-query'
import type { ScheduledRelease, ScoredWork } from '@/lib/api/types'
import {
  MEDIA_GAPS,
  narrow,
  narrowByColumns,
  sortRows,
  withNextReleases,
  type CatalogueFilter,
  type CatalogueRow,
  type ColumnFilters,
  type MediaOf,
  type RepeatLevels,
  type Sort,
} from '@/lib/catalogue'
import { queries } from '@/lib/query/queries'
import { useDebounced } from '@/lib/useDebounced'
import { useProfile } from '@/lib/useProfile'
import { axesOf } from '@/lib/vibe'

interface Shown {
  /** Every work, each with its next release. */
  all: CatalogueRow[]
  /** What the table shows: narrowed by the box and the funnels, then sorted. */
  visible: CatalogueRow[]
  /** Every work's media, once a filter by them asked (v0.155). */
  media: MediaOf | undefined
}

/**
 * What the table shows, worked out once for the toolbar's counts and the
 * table's rows - two readings of one list, which must not disagree.
 *
 * The one thing the rows themselves cannot answer is which works say the
 * typed text somewhere inside them - in a lyric, a note, a craft field, a
 * reply from the assistant. That is asked of the index, debounced, because it
 * is a round trip and the box is typed in.
 */
export function useShownRows(
  rows: ScoredWork[] | undefined,
  releases: ScheduledRelease[] | undefined,
  filter: CatalogueFilter,
  columnFilters: ColumnFilters,
  sort: Sort,
  /** Every work's mark of the guard, for the slice by repeats. */
  repeats: RepeatLevels,
): Shown | undefined {
  const profile = useProfile()
  const text = filter.search?.trim() ?? ''
  const settled = useDebounced(text, 160)
  const matches = useQuery({
    ...queries.worksMatching(settled),
    enabled: settled !== '',
    // The corpus does not change while a word is being typed, and every
    // keystroke that ends in the same query should cost nothing.
    staleTime: 30_000,
  })
  // While the answer for the current word is still on its way, the previous
  // one is worse than none: it would narrow to the hits for `frost` while the
  // box reads `frostbite`. Titles keep narrowing in the meantime.
  const matching =
    settled === '' || settled !== text || matches.data === undefined ? undefined : matches.data

  // The planned slots ride with their works, for the next release's cell.
  const plans = useQuery(queries.workPlans())
  // A work's media are looked for on disk: asked only while a filter by
  // them is on (v0.155).
  const media = useQuery({
    ...queries.worksMedia(),
    enabled: filter.gap !== undefined && MEDIA_GAPS.includes(filter.gap),
  })

  if (rows === undefined) return undefined
  const all = withNextReleases(rows, releases, plans.data)
  // The funnels narrow what the query left: the two compose as AND, and the
  // count above the table reads both.
  const visible = sortRows(
    narrowByColumns(narrow(all, filter, matching, repeats, media.data), columnFilters),
    sort,
    axesOf(profile.config),
  )
  return { all, visible, media: media.data }
}
