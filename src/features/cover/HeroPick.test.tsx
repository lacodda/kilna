import { describe, expect, it } from 'vitest'
import { fireEvent, render, screen, within } from '@testing-library/react'
import type { CardSummary, CoverHero } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { HeroPick } from '@/features/cover/HeroPick'
import { createQueryClient } from '@/lib/query/client'
import { ProfileContext } from '@/lib/useProfile'
import { mockBackend } from '@/test/backend'
import { answersFor, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The hero of a cover from the canon (v0.133, FR-05 5.3): the public cards
 * first, every layer on asking, each inner one with its layer said.
 */

function card(id: string, title: string, layer: CardSummary['layer']): CardSummary {
  return {
    id,
    kind: 'character',
    title,
    aliases: [],
    layer,
    portrait: null,
    work_title: null,
  } as unknown as CardSummary
}

function mounted(onChange: (hero: CoverHero | null) => void = () => {}) {
  const workspace = studio()
  mockBackend({
    ...answersFor(workspace),
    list_cards: () => [card('c-open', 'Wren', 'public'), card('c-inner', 'The Warden', 'internal')],
  })
  render(
    <Providers client={createQueryClient()}>
      <ProfileContext.Provider value={workspace.profile}>
        <HeroPick hero={null} state={null} onChange={onChange} />
      </ProfileContext.Provider>
    </Providers>,
  )
}

describe('the hero picker', () => {
  it('offers the public heroes, and every layer on asking with the layer said', async () => {
    const picked: (CoverHero | null)[] = []
    mounted((hero) => picked.push(hero))
    fireEvent.click(screen.getByRole('button', { name: en.cover.hero.pick }))
    const list = await screen.findByRole('list', { name: en.cover.hero.cards })
    await within(list).findByText('Wren')
    expect(within(list).queryByText('The Warden')).toBeNull()

    fireEvent.click(screen.getByRole('switch', { name: en.cover.hero.everyLayer }))
    const inner = (await within(list).findByText('The Warden')).closest('button')!
    expect(within(inner).getByText(en.canon.layer.short.internal)).toBeVisible()
    fireEvent.click(inner)
    expect(picked).toEqual([{ card: 'c-inner', references: true }])
  })
})
