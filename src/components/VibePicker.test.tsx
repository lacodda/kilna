import { describe, expect, it } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import type { ShownVibe, Vibe, Work } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { VibePicker } from '@/components/VibePicker'
import { createQueryClient } from '@/lib/query/client'
import { ProfileContext } from '@/lib/useProfile'
import { mockBackend } from '@/test/backend'
import { answersFor, IDS, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The mark of what a work feels like, and the three lines that set it
 * (FR-02): one line said draws a mark, the word that is on comes off, and a
 * work with none of its own shows whose it has taken.
 */

function mounted(own: Vibe | undefined, shown: ShownVibe | undefined) {
  const workspace = studio()
  const backend = mockBackend({
    ...answersFor(workspace),
    update_work: ({ id, patch }) => ({
      ...workspace.works.find((work) => work.id === id)!,
      ...(patch as Partial<Work>),
    }),
  })
  render(
    <Providers client={createQueryClient()}>
      <ProfileContext.Provider value={workspace.profile}>
        <VibePicker workId={IDS.song} own={own} shown={shown} />
      </ProfileContext.Provider>
    </Providers>,
  )
  return backend
}

describe('the vibe picker', () => {
  it('is a quiet way in while nothing is said, and draws a mark from one word', async () => {
    const backend = mounted(undefined, undefined)

    fireEvent.click(screen.getByRole('button', { name: en.vibe.set }))
    expect(await screen.findByText(en.vibe.none)).toBeVisible()
    // The three lines, each with the profile's own words.
    const moods = screen.getByRole('group', { name: en.vibe.line.mood })
    expect(within(moods).getAllByRole('button')).toHaveLength(5)
    expect(
      within(screen.getByRole('group', { name: en.vibe.line.complexity })).getAllByRole('button'),
    ).toHaveLength(3)
    // Nothing of its own to clear.
    expect(screen.getByRole('button', { name: en.vibe.clear })).toBeDisabled()

    fireEvent.click(within(moods).getByRole('button', { name: 'Melancholy' }))
    await waitFor(() =>
      expect(backend.argsOf('update_work')).toEqual([
        { id: IDS.song, patch: { vibe: { mood: 'melancholy' } } },
      ]),
    )
  })

  it('takes a word off when it is pressed again, and the mark with its last word', async () => {
    const own = { mood: 'dark', character: 'edgy' }
    const backend = mounted(own, { vibe: own })

    // The mark says the words that were said, and only those.
    fireEvent.click(screen.getByRole('button', { name: 'Dark · Raw' }))
    const moods = await screen.findByRole('group', { name: en.vibe.line.mood })
    expect(within(moods).getByRole('button', { name: 'Dark' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )

    fireEvent.click(within(moods).getByRole('button', { name: 'Dark' }))
    await waitFor(() =>
      expect(backend.argsOf('update_work').at(-1)).toEqual({
        id: IDS.song,
        patch: { vibe: { character: 'edgy' } },
      }),
    )

    fireEvent.click(screen.getByRole('button', { name: en.vibe.clear }))
    await waitFor(() =>
      expect(backend.argsOf('update_work').at(-1)).toEqual({
        id: IDS.song,
        patch: { vibe: null },
      }),
    )
  })

  it('shows whose vibe a work has taken, and makes it its own when one is said', async () => {
    const backend = mounted(undefined, { vibe: { mood: 'warm' }, from: 'Paper Lanterns' })

    const taken = en.vibe.takenFrom
      .replace('{{vibe}}', 'Bright')
      .replace('{{title}}', 'Paper Lanterns')
    fireEvent.click(screen.getByRole('button', { name: taken }))
    // Nothing of its own is pressed: the source's is shown, not set here.
    const moods = await screen.findByRole('group', { name: en.vibe.line.mood })
    expect(within(moods).getByRole('button', { name: 'Bright' })).toHaveAttribute(
      'aria-pressed',
      'false',
    )

    fireEvent.click(
      within(screen.getByRole('group', { name: en.vibe.line.character })).getByRole('button', {
        name: 'Drive',
      }),
    )
    await waitFor(() =>
      expect(backend.argsOf('update_work')).toEqual([
        { id: IDS.song, patch: { vibe: { character: 'drive' } } },
      ]),
    )
  })
})
