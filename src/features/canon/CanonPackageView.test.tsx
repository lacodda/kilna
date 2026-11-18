import { afterEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import type { CanonPackage } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend } from '@/test/backend'
import { CanonPackageView, itemsOf } from '@/features/canon/CanonPackageView'
import en from '@/i18n/locales/en.json'

/*
 * A proposal for the canon that brings a card its pictures and its
 * description (v0.89.2): each is an item of its own, read before it is kept -
 * the card, the role and the file of a picture, the words of a description -
 * and each can be left out.
 */

const PACK: CanonPackage = {
  cards: [],
  facts: [],
  links: [],
  pictures: [
    {
      path: 'C:\\refs\\wren\\wren-face.png',
      card: 'card-wren',
      card_title: 'Wren',
      role: 'portrait',
    },
  ],
  descriptions: [{ card: 'card-wren', card_title: 'Wren', text: 'A young woman with freckles.' }],
  dropped: [],
}

afterEach(() => {
  vi.restoreAllMocks()
})

describe('a proposal for the canon with pictures and descriptions', () => {
  it('names every picture and description as an item to keep', () => {
    expect(itemsOf(PACK)).toEqual(['picture:0', 'description:0'])
  })

  it('shows the card, the role and the file of a picture, and the words of a description', async () => {
    mockBackend({ canon_review: () => [] })
    const chosen = vi.fn()
    render(
      <Providers client={createQueryClient()}>
        <CanonPackageView
          messageId="m-1"
          pack={PACK}
          chosen={itemsOf(PACK)}
          onChosen={chosen}
          answered={false}
        />
      </Providers>,
    )

    expect(await screen.findByText(en.canon.pictures)).toBeInTheDocument()
    expect(screen.getByText(en.canon.descriptions)).toBeInTheDocument()
    expect(screen.getByText('wren-face.png')).toBeInTheDocument()
    expect(screen.getByText(new RegExp(en.canon.role.portrait))).toBeInTheDocument()
    expect(screen.getByText('A young woman with freckles.')).toBeInTheDocument()

    const boxes = screen.getAllByRole('checkbox')
    expect(boxes).toHaveLength(2)
    fireEvent.click(boxes[0]!)
    expect(chosen).toHaveBeenCalledWith(['description:0'])
  })
})

// v0.144, FR-18 18.5: what an agent may now propose - a card changed, a fact
// moved or taken away, a near-duplicate marked, a relation taken away.
describe('a proposal that changes the canon', () => {
  const CHANGES: CanonPackage = {
    cards: [],
    facts: [
      { change: 'move', fact_id: 'f-coat', card: 'card-wren', section: 'outfits' },
      { change: 'delete', fact_id: 'f-old', card: 'card-wren' },
      {
        change: 'add',
        card: 'card-wren',
        section: 'looks',
        body: 'freckles across her nose',
        similar_to: { fact_id: 'f-freckles', text: 'Freckles across the nose.', section: 'looks' },
      },
    ],
    links: [{ from: 'card-wren', to: 'card-mill', delete: true }],
    pictures: [],
    descriptions: [],
    card_updates: [{ card: 'card-wren', card_title: 'Wren', title: 'Wren Hale', note: 'A girl.' }],
    dropped: [],
  }

  it('names every change as an item, and shows what each does', async () => {
    expect(itemsOf(CHANGES)).toEqual(['fact:0', 'fact:1', 'fact:2', 'relation:0', 'update:0'])
    mockBackend({ canon_review: () => [] })
    render(
      <Providers client={createQueryClient()}>
        <CanonPackageView
          messageId="m-2"
          pack={CHANGES}
          chosen={itemsOf(CHANGES)}
          onChosen={vi.fn()}
          answered={false}
        />
      </Providers>,
    )
    expect(await screen.findByText(en.canon.cardUpdates)).toBeInTheDocument()
    expect(screen.getByText(/→ Wren Hale/)).toBeInTheDocument()
    expect(screen.getByText(/→ outfits/)).toBeInTheDocument()
    expect(
      screen.getByText(en.canon.similarTo.replace('{{fact}}', 'Freckles across the nose.')),
    ).toBeInTheDocument()
    expect(screen.getByText(en.canon.relationTakenAway)).toBeInTheDocument()
  })
})
