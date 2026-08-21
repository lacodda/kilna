import type { ScoredWork } from '@/lib/api'

export interface CatalogueFilter {
  search?: string
  status?: string
  kind?: string
}

/**
 * Narrow the catalogue in the app rather than in SQL.
 *
 * The whole catalogue is one query and a few hundred rows at most, so filtering
 * here costs nothing and buys two things the database cannot give as cheaply:
 * the unfiltered total is still in hand for a "showing N of M" count, and the
 * case folding is JavaScript's, which handles Cyrillic — SQLite's `LIKE` folds
 * ASCII only, the defect that cost v0.19 a fix in two places.
 */
export function narrow(rows: ScoredWork[], filter: CatalogueFilter): ScoredWork[] {
  const needle = filter.search?.trim().toLowerCase() ?? ''

  return rows.filter((row) => {
    if (filter.status !== undefined && row.status !== filter.status) return false
    if (filter.kind !== undefined && row.kind !== filter.kind) return false
    if (needle !== '' && !row.title.toLowerCase().includes(needle)) return false
    return true
  })
}

/** True when anything is narrowing the list — what an empty result has to explain. */
export function isNarrowed(filter: CatalogueFilter): boolean {
  return (
    (filter.search !== undefined && filter.search.trim() !== '') ||
    filter.status !== undefined ||
    filter.kind !== undefined
  )
}
