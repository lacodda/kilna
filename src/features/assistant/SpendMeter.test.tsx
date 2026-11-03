import { describe, expect, it } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import type { Spend } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { SpendMeter } from '@/features/assistant/SpendMeter'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend } from '@/test/backend'

function mount(spend: Spend) {
  const backend = mockBackend({
    assistant_spend: () => spend,
    set_weekly_budget: ({ budget }) => budget ?? null,
  })
  render(
    <Providers client={createQueryClient()}>
      <SpendMeter />
    </Providers>,
  )
  return backend
}

describe('what the runs cost (v0.107)', () => {
  it('says the day and the week, and the week against its budget', async () => {
    const backend = mount({ today: 0.42, week: 3.1, budget: 10 })

    const meter = await screen.findByRole('button', { name: /Today \$0\.42/ })
    expect(meter).toHaveTextContent('Today $0.42 · This week $3.10 of $10.00')
    expect(meter).not.toHaveClass('text-bad')
    // The window's own midnight and Monday, as moments.
    const [asked] = backend.argsOf('assistant_spend')
    expect(asked?.dayFrom).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/)
    expect(String(asked?.weekFrom) <= String(asked?.dayFrom)).toBe(true)
  })

  it('turns red past the budget', async () => {
    mount({ today: 2, week: 12.4, budget: 10 })
    expect(await screen.findByRole('button', { name: /Today/ })).toHaveClass('text-bad')
  })

  it('sets a budget, and takes it away', async () => {
    const backend = mount({ today: 0, week: 0, budget: null })

    fireEvent.click(
      await screen.findByRole('button', { name: /Today \$0\.00 · This week \$0\.00$/ }),
    )
    // With no budget set there is nothing to take away.
    expect(await screen.findByRole('button', { name: 'No budget' })).toBeDisabled()
    const field = await screen.findByRole('textbox', { name: 'Weekly budget' })
    fireEvent.change(field, { target: { value: '0.5' } })
    fireEvent.blur(field)
    // A click, as a person makes it: a submit the browser's own checks would
    // refuse is refused here too.
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(backend.argsOf('set_weekly_budget')).toEqual([{ budget: 0.5 }]))
  })
})
