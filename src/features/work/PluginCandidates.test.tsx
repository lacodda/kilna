import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'
import { render, screen } from '@testing-library/react'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend } from '@/test/backend'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import { PluginCandidates, secondsOf } from '@/features/work/PluginCandidates'

/*
 * The takes a generator offered, each held against the length the work is
 * meant to play (v0.158, FR-21 21.1 p.2).
 */

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('the takes to choose from', () => {
  it('reads a length as it is written', () => {
    expect(secondsOf('3:05')).toBe(185)
    expect(secondsOf('185')).toBe(185)
    expect(secondsOf('185 s')).toBe(185)
    expect(secondsOf(170)).toBe(170)
    expect(secondsOf('about three minutes')).toBeNull()
    expect(secondsOf(undefined)).toBeNull()
  })

  it('says how far each take strays from the length the work is meant to play', async () => {
    const workspace = studio()
    const song = workspace.works.find((one) => one.id === IDS.song)!
    song.meta = { ...song.meta, duration: '3:00' }
    mockBackend(answersFor(workspace))
    render(
      <Providers client={createQueryClient()}>
        <MemoryRouter>
          <PluginCandidates
            workId={IDS.song}
            candidates={[
              { label: 'Take 1', files: [], meta: {}, seconds: 184, note: null },
              { label: 'Take 2', files: [], meta: {}, seconds: 120, note: null },
            ]}
            onClose={() => undefined}
          />
        </MemoryRouter>
      </Providers>,
    )

    expect(await screen.findByText('184 s · +4 s from 3:00')).toBeInTheDocument()
    expect(screen.getByText('120 s · -60 s from 3:00')).toBeInTheDocument()
  })
})
