import { invoke } from '@tauri-apps/api/core'
import type { Link, Links, NewLink, ResolvedLink, Work } from '@/lib/api/types'

// What a work was made from, and what was made from it.

export const listLinks = (workId: string) => invoke<Links>('list_links', { workId })
export const createLink = (link: NewLink) => invoke<Link>('create_link', { link })
export const deleteLink = (id: string) => invoke<void>('delete_link', { id })
/** Make a work of `kind` from another: title and overview fields copied once, a donor link. */
export const deriveWork = (sourceId: string, kind: string, title?: string) =>
  invoke<Work>('derive_work', { sourceId, kind, title: title ?? null })
export const resolveLinks = (works: string[], versions: string[]) =>
  invoke<ResolvedLink[]>('resolve_links', { works, versions })
