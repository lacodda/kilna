import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import type { Diagnostics, UpdateCheck } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend, type Backend } from '@/test/backend'
import { answersFor, NOW, studio } from '@/test/workspace'
import { AboutSection } from '@/features/settings/AboutSection'
import { issueFor } from '@/components/ErrorBoundary'

/*
 * About kilna and what to send when something breaks (v0.162): the version,
 * a check for updates on a press, the diagnostics copied as one text, and a
 * crash's issue written without the person's home in it.
 */

const FOUND: Diagnostics = {
  version: '0.162.0',
  os: 'windows',
  arch: 'x86_64',
  webview: '131.0',
  cli: '2.1.306',
  errors: [],
}

let backend: Backend

function mount(update: UpdateCheck) {
  backend = mockBackend({
    ...answersFor(studio()),
    diagnostics: () => FOUND,
    diagnostics_text: () => 'kilna 0.162.0\nSystem: windows (x86_64)\n',
    check_update: () => update,
  })
  render(
    <Providers client={createQueryClient()}>
      <MemoryRouter>
        <AboutSection />
      </MemoryRouter>
    </Providers>,
  )
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  Object.assign(navigator, { clipboard: { writeText: vi.fn(() => Promise.resolve()) } })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('about kilna', () => {
  it('says the version and the machine, and copies the diagnostics on a press', async () => {
    mount({ current: '0.162.0', latest: '0.162.0', url: null, newer: false })
    expect(await screen.findByText('kilna 0.162.0')).toBeInTheDocument()
    expect(
      screen.getByText('windows (x86_64) · WebView 131.0 · Claude Code 2.1.306'),
    ).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Copy the diagnostics' }))
    await waitFor(() =>
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
        'kilna 0.162.0\nSystem: windows (x86_64)\n',
      ),
    )
  })

  it('offers a newer version to download when one is out', async () => {
    mount({
      current: '0.162.0',
      latest: '0.163.0',
      url: 'https://github.com/lacodda/kilna/releases/tag/v0.163.0',
      newer: true,
    })
    fireEvent.click(await screen.findByRole('button', { name: 'Check for updates' }))
    await waitFor(() => expect(backend.argsOf('check_update')).toHaveLength(1))
    expect(await screen.findByRole('button', { name: 'Download 0.163.0' })).toBeInTheDocument()
  })

  it('writes a crash’s issue with no home folder in it', () => {
    const error = new Error('cannot read C:\\Users\\someone\\media\\a.mp4')
    error.stack = 'Error: x\n    at Screen (C:/Users/someone/app/index.js:1:1)'
    const link = decodeURIComponent(issueFor(error))
    expect(link.startsWith('https://github.com/lacodda/kilna/issues/new?')).toBe(true)
    expect(link).not.toContain('someone')
    expect(link).toContain('~')
  })
})
