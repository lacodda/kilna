import type { Hit, HitKind } from '@/lib/api'

/** The card tab each kind of hit lives on, for the hits a card opens on. */
const TAB_FOR: Record<Exclude<HitKind, 'note' | 'comment'>, string> = {
  work: 'overview',
  version: 'versions',
  message: 'assistant',
}

/**
 * Where opening a hit goes.
 *
 * A note opens on the notes screen and a comment on the comments screen,
 * whether or not either belongs to a work - those screens are their homes
 * since v0.76, and one about nothing in particular has no card to open on at
 * all. Everything else opens the work it belongs to on the tab where it lives.
 */
export function hrefOfHit(hit: Hit): string {
  if (hit.kind === 'note') return `/notes/${hit.entity_id}`
  if (hit.kind === 'comment') return `/comments/${hit.entity_id}`
  return `/works/${hit.work_id ?? ''}/${TAB_FOR[hit.kind]}`
}
