import { createContext, useContext } from 'react'
import type { CardView } from '@/lib/api/types'
import type { LensChoice } from '@/lib/canon'

/**
 * Whether an open card shows its layers and its states (v0.141, FR-18 18.2):
 * the letters A, B, C beside each fact and the word for how settled it is.
 * Shown only where they say something - a card holding a fact or a relation
 * that is not public, or a fact still a draft or a live zone - or while the
 * person is looking at how a task reads the card. A card of public, settled
 * facts reads as a plain list.
 *
 * Everywhere outside an open card - the timeline, the proposals - the marks
 * stay, as they always were: the default is to show them.
 */
export const Detailed = createContext(true)

export function useDetailed(): boolean {
  return useContext(Detailed)
}

export function isDetailed(view: CardView, lens: LensChoice): boolean {
  return (
    lens !== 'all' ||
    view.facts.some(
      ({ fact }) => fact.layer !== 'public' || fact.status === 'draft' || fact.status === 'open',
    ) ||
    view.relations.some((relation) => relation.link.layer !== 'public')
  )
}
