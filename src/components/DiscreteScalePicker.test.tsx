import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import { DiscreteScalePicker } from '@/components/DiscreteScalePicker'

/* A scale of named stops (v0.135): each a radio, the arrows walk them. */

const steps = [
  { value: 'none', label: 'No subject' },
  { value: 'full', label: 'Full figure' },
  { value: 'head', label: 'Head' },
] as const

describe('a scale of stops', () => {
  it('says the stop it stands on and takes another by a press or an arrow', () => {
    const onChange = vi.fn()
    render(<DiscreteScalePicker label="Shot" steps={steps} value="full" onChange={onChange} />)
    expect(screen.getByRole('radio', { name: 'Full figure' })).toHaveAttribute(
      'aria-checked',
      'true',
    )
    fireEvent.click(screen.getByRole('radio', { name: 'No subject' }))
    expect(onChange).toHaveBeenLastCalledWith('none')
    fireEvent.keyDown(screen.getByRole('radiogroup', { name: 'Shot' }), { key: 'ArrowRight' })
    expect(onChange).toHaveBeenLastCalledWith('head')
  })
})
