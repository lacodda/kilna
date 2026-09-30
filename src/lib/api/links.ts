import { invoke } from '@tauri-apps/api/core'
import type { Link, Links, Made, NewLink, Publications, ResolvedLink } from '@/lib/api/types'

// What a work was made from, and what was made from it.

export const listLinks = (workId: string) => invoke<Links>('list_links', { workId })
export const createLink = (link: NewLink) => invoke<Link>('create_link', { link })
export const deleteLink = (id: string) => invoke<void>('delete_link', { id })
/** Make a work of `kind` from another: its title from the kind's `made_title`
 *  in `locale` (the window's language) unless one is given, the overview fields
 *  its kind has, a donor link, and - for a kind that goes out - one release
 *  through its first door, with no day yet. */
export const deriveWork = (sourceId: string, kind: string, locale: string, title?: string) =>
  invoke<Made>('derive_work', { sourceId, kind, title: title ?? null, locale })
/** Everything made from a work, down the links, and the release its status
 *  stands on (v0.86): a song's publications. */
export const listPublications = (workId: string) =>
  invoke<Publications>('list_publications', { workId })
export const resolveLinks = (works: string[], versions: string[]) =>
  invoke<ResolvedLink[]>('resolve_links', { works, versions })
