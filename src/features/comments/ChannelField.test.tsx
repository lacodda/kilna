import { useState } from 'react'
import { describe, expect, it } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import { ChannelField } from '@/features/comments/ChannelField'
import en from '@/i18n/locales/en.json'

/*
 * Where a comment was written (FR-19 19.2): any word, with the words already
 * used offered in a list under the field - not as a row of buttons beside it.
 */

function Held({ known }: { known: string[] }) {
  const [value, setValue] = useState('')
  return (
    <>
      <ChannelField value={value} onChange={setValue} known={known} />
      <output>{value}</output>
    </>
  )
}

const field = () => screen.getByRole('combobox', { name: en.comments.channel })

describe('the channel field', () => {
  it('keeps any word typed, in the list or not', () => {
    render(<Held known={['YouTube', 'Telegram']} />)

    fireEvent.change(field(), { target: { value: 'A forum nobody listed' } })

    expect(screen.getByRole('status')).toHaveTextContent('A forum nobody listed')
    expect(field()).toHaveValue('A forum nobody listed')
  })

  it('offers the channels already used when entered, narrowed by what is typed', async () => {
    render(<Held known={['YouTube', 'Telegram', 'YouTube Music']} />)

    // No buttons stand beside the field: the words are in its list.
    expect(screen.queryByRole('button', { name: 'Telegram' })).toBeNull()

    fireEvent.focus(field())
    expect((await screen.findAllByRole('option')).map((one) => one.textContent)).toEqual([
      'YouTube',
      'Telegram',
      'YouTube Music',
    ])

    fireEvent.change(field(), { target: { value: 'you' } })
    expect((await screen.findAllByRole('option')).map((one) => one.textContent)).toEqual([
      'YouTube',
      'YouTube Music',
    ])

    fireEvent.click(screen.getByRole('option', { name: 'YouTube Music' }))
    expect(screen.getByRole('status')).toHaveTextContent('YouTube Music')
  })
})
