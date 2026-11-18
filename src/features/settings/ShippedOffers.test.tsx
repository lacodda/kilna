import { describe, expect, it } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import type { ShippedOffer } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { ShippedOffers } from '@/features/settings/ShippedOffers'
import { changes, listOf, shown } from '@/lib/shippedOffer'
import { createQueryClient } from '@/lib/query/client'
import { ProfileContext } from '@/lib/useProfile'
import { mockBackend } from '@/test/backend'
import { answersFor, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * What kilna ships now beside the profile's entries (ADR 0087): an entry
 * that reads otherwise is shown with what differs, and is kept or taken by
 * a person - nothing waits, nothing is shown.
 */

const draft: ShippedOffer = {
  collection: 'work_kinds/song/statuses',
  key: 'draft',
  mine: { key: 'draft', label: 'Sketch', derive: 'manual' },
  standard: { key: 'draft', label: { en: 'Draft', ru: 'Черновик' }, derive: 'manual' },
}
const action: ShippedOffer = {
  collection: 'prompts',
  key: 'critique',
  mine: { key: 'critique', label: 'Critique', template: 'Read it.\nBe brief.' },
  standard: { key: 'critique', label: 'Critique', template: 'Read it.\nSay what works.' },
}

function mounted(offers: ShippedOffer[]) {
  const workspace = studio()
  const backend = mockBackend({
    ...answersFor(workspace),
    shipped_offers: () => offers,
    take_shipped: () => workspace.profile,
    keep_mine: () => null,
  })
  const view = render(
    <Providers client={createQueryClient()}>
      <ProfileContext.Provider value={workspace.profile}>
        <ShippedOffers />
      </ProfileContext.Provider>
    </Providers>,
  )
  return { backend, view, profileId: workspace.profile.id }
}

describe('what kilna ships now', () => {
  it('shows nothing while nothing waits', async () => {
    const { backend, view } = mounted([])
    await waitFor(() => expect(backend.argsOf('shipped_offers')).toHaveLength(1))
    expect(view.container).toBeEmptyDOMElement()
  })

  it('says which entry of which list differs, and how', async () => {
    mounted([draft, action])
    const offer = (await screen.findByRole('heading', { name: /^Sketch/ })).closest('li')!
    expect(within(offer).getByText(/of Song/)).toBeVisible()
    // A word differs: the owner's beside the standard.
    expect(within(offer).getByText('label')).toBeVisible()
    expect(within(offer).getByText('Draft / Черновик')).toBeVisible()
    expect(within(offer).queryByText('derive')).toBeNull()
    // A text of several lines is compared line by line.
    const critique = screen.getByRole('heading', { name: /^Critique/ }).closest('li')!
    expect(within(critique).getByText('Say what works.').closest('[data-change]')).toHaveAttribute(
      'data-change',
      'added',
    )
  })

  it('takes the standard for one entry, and keeps the owner’s for all', async () => {
    const { backend, profileId } = mounted([draft, action])
    const offer = (await screen.findByRole('heading', { name: /^Sketch/ })).closest('li')!
    fireEvent.click(within(offer).getByRole('button', { name: en.settings.shipped.take }))
    await waitFor(() =>
      expect(backend.argsOf('take_shipped')).toEqual([
        { id: profileId, offer: { collection: 'work_kinds/song/statuses', key: 'draft' } },
      ]),
    )

    fireEvent.click(
      screen.getByRole('button', {
        name: en.settings.shipped.keepAll.replace('{{count}}', '2'),
      }),
    )
    await waitFor(() =>
      expect(backend.argsOf('keep_mine')).toEqual([
        {
          id: profileId,
          offers: [
            { collection: 'work_kinds/song/statuses', key: 'draft' },
            { collection: 'prompts', key: 'critique' },
          ],
        },
      ]),
    )
  })
})

describe('reading an offer', () => {
  it('names only the fields that differ, a word in every language joined', () => {
    expect(changes(draft.mine, draft.standard)).toEqual([
      { field: 'label', mine: 'Sketch', standard: 'Draft / Черновик', long: false },
    ])
    expect(changes(action.mine, action.standard)[0]).toMatchObject({
      field: 'template',
      long: true,
    })
    // A field only the owner's entry has is a difference too.
    expect(changes({ key: 'a', hint: 'mine' }, { key: 'a' })).toEqual([
      { field: 'hint', mine: 'mine', standard: '', long: false },
    ])
    expect(shown(['a', 'b'])).toBe('a, b')
    expect(shown({ at: 3 })).toBe('{"at":3}')
  })

  it('names the list, and the kind a list inside a kind belongs to', () => {
    const config = studio().profile.config
    expect(listOf('prompts', config)).toEqual({ word: 'prompts' })
    expect(listOf('vibe/moods', config)).toEqual({ word: 'vibe_moods' })
    expect(listOf('work_kinds/song/statuses', config)).toEqual({ word: 'statuses', of: 'Song' })
    expect(listOf('note_kinds/gone/sections', config)).toEqual({ word: 'sections', of: 'gone' })
  })
})
