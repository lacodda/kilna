import type { Hit, HitKind } from '@/lib/api'

/** The card tab each kind of hit lives on, for the hits a card opens on. */
const TAB_FOR: Record<Exclude<HitKind, 'note'>, string> = {
  work: 'overview',
  version: 'versions',
  message: 'assistant',
}

/**
 * Where opening a hit goes.
 *
 * A note opens on the notes screen, where it can be edited and moved on,
 * whether or not it belongs to a work - that screen is the notes' own home
 * since v0.76, and a note on nothing in particular has no card to open on at
 * all. Everything else opens the work it belongs to on the tab where it lives.
 */
export function hrefOfHit(hit: Hit): string {
  if (hit.kind === 'note') return `/notes/${hit.entity_id}`
  return `/works/${hit.work_id ?? ''}/${TAB_FOR[hit.kind]}`
}
