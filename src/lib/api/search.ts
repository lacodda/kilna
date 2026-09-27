import { invoke } from '@tauri-apps/api/core'
import type { Hit } from '@/lib/api/types'

// Finding things by what they say.

export const search = (query: string) => invoke<Hit[]>('search', { query })
/** The works whose text answers a query, best match first — ids only.
 *
 * What the catalogue's box asks, as against the palette's: the rows are
 * already on the screen, so only the narrowing comes back. */
export const worksMatching = (query: string) => invoke<string[]>('works_matching', { query })
