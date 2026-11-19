import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend } from '@/test/backend'
import { ReleaseNumbers } from '@/features/releases/ReleaseNumbers'

/*
 * How a release did, as YouTube Studio's export said (v0.160, FR-22 22.1
 * p.2) - and nothing at all when it was never read.
 */

function mount(meta: Record<string, unknown>) {
  mockBackend({})
  return render(
    <Providers client={createQueryClient()}>
      <ReleaseNumbers meta={meta} />
    </Providers>,
  )
}

describe('a release’s numbers', () => {
  it('says the numbers the export gave, and when it was read', () => {
    mount({
      analytics: {
        views: 1200,
        ctr: 6.24,
        avg_view_pct: 45.4,
        likes: 40,
        read_at: '2026-11-19T10:00:00.000Z',
        from: 'youtube_studio',
      },
    })
    expect(screen.getByText(/1,200 views · CTR 6\.2% · 45% watched · 40 likes/)).toBeInTheDocument()
    expect(screen.getByText(/read /)).toBeInTheDocument()
  })

  it('says nothing for a release never read', () => {
    const { container } = mount({ title: 'Dawn' })
    expect(container.textContent).toBe('')
  })
})
