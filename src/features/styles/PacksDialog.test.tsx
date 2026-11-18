import { describe, expect, it } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import type { ListedPack, OpenedPack, PackEntry } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { PacksDialog } from '@/features/styles/PacksDialog'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend } from '@/test/backend'
import en from '@/i18n/locales/en.json'

/*
 * The catalogue of style packs (v0.131, FR-06): narrowed by half, a pack
 * opened into its bricks, installed whole or one style at a time.
 */

function entry(id: string, half: string, title: string): PackEntry {
  return {
    id,
    version: '1.0.0',
    digest: 'x',
    title: { en: title, ru: title },
    description: { en: `${title}, described`, ru: title },
    half,
    category: half === 'sound' ? 'genres' : 'photo-trends',
    brick_count: 2,
    updated_at: '2026-11-18',
    manifest_url: `packs/${id}.json`,
  }
}

const film = entry('film-photo', 'picture', 'Film photography')
const lofi = entry('lofi-textures', 'sound', 'Lo-fi textures')
const listed: ListedPack[] = [
  { source: { kind: 'builtin' }, entry: film, standing: 'absent' },
  { source: { kind: 'builtin' }, entry: lofi, standing: 'installed' },
]
const opened: OpenedPack = {
  source: { kind: 'builtin' },
  entry: film,
  bricks: [
    [
      {
        key: 'warm',
        type: 'image-style',
        label: { en: 'Warm stock', ru: 'Тёплая' },
        family: 'film-photo',
        description: 'warm film',
      },
      'absent',
    ],
    [
      {
        key: 'grain',
        type: 'image-style',
        label: { en: 'Hard grain', ru: 'Зерно' },
        family: 'film-photo',
        description: 'hard grain',
      },
      'changed',
    ],
  ],
}

function mounted() {
  const backend = mockBackend({
    style_packs: () => ({
      packs: listed,
      failed: [],
      sources: [{ kind: 'builtin' }],
      offline: false,
    }),
    style_pack: () => opened,
    install_style_pack: () => ({ added: 1, renewed: 0, kept: 1, unfit: 0, previews: 0 }),
  })
  render(
    <Providers client={createQueryClient()}>
      <PacksDialog open onOpenChange={() => {}} />
    </Providers>,
  )
  return backend
}

describe('the catalogue of style packs', () => {
  it('lists the packs with where each stands, narrowed by half', async () => {
    mounted()
    const filmRow = (await screen.findByRole('heading', { name: /Film photography/ })).closest(
      'li',
    )!
    expect(within(filmRow).getByText(en.packs.standing.absent)).toBeVisible()
    fireEvent.click(screen.getByRole('radio', { name: en.packs.halves.sound }))
    expect(screen.queryByRole('heading', { name: /Film photography/ })).toBeNull()
    expect(screen.getByRole('heading', { name: /Lo-fi textures/ })).toBeVisible()
  })

  it('opens a pack and installs it whole or one style at a time', async () => {
    const backend = mounted()
    const filmRow = (await screen.findByRole('heading', { name: /Film photography/ })).closest(
      'li',
    )!
    fireEvent.click(within(filmRow).getByRole('button', { name: en.packs.open }))

    const warm = (await screen.findByText('Warm stock')).closest('li')!
    fireEvent.click(within(warm).getByRole('button', { name: en.packs.installOne }))
    await waitFor(() =>
      expect(backend.argsOf('install_style_pack')[0]).toEqual({
        source: { kind: 'builtin' },
        pack: 'film-photo',
        only: ['warm'],
      }),
    )
    // The owner's own words are not installed over.
    const grain = screen.getByText('Hard grain').closest('li')!
    expect(within(grain).getByRole('button', { name: en.packs.installOne })).toBeDisabled()

    fireEvent.click(screen.getByRole('button', { name: en.packs.installPack }))
    await waitFor(() => expect(backend.argsOf('install_style_pack')).toHaveLength(2))
    expect(backend.argsOf('install_style_pack')[1]).toMatchObject({ only: null })
  })
})
