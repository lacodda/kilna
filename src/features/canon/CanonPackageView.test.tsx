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
