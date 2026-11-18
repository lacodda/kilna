import { beforeEach, describe, expect, it } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import { CoachMark, QUIET, resetCoachMarks } from '@/components/CoachMark'
import en from '@/i18n/locales/en.json'

/*
 * What a screen is for, said on its first visit on this machine (v0.129,
 * #112): once per screen, all off on request, and back from Settings.
 */

beforeEach(() => {
  localStorage.removeItem(QUIET)
})

describe('a first-visit hint', () => {
  it('says what the screen is for until it is dismissed, then never again there', () => {
    const first = render(<CoachMark screen="catalogue" />)
    expect(screen.getByText(en.coach.screens.catalogue)).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: en.coach.gotIt }))
    expect(screen.queryByText(en.coach.screens.catalogue)).toBeNull()
    first.unmount()

    render(<CoachMark screen="catalogue" />)
    expect(screen.queryByText(en.coach.screens.catalogue)).toBeNull()
    // Another screen still has its first visit.
    render(<CoachMark screen="calendar" />)
    expect(screen.getByText(en.coach.screens.calendar)).toBeVisible()
  })

  it('goes quiet everywhere on request, and comes back from Settings', () => {
    const view = render(<CoachMark screen="canon" />)
    fireEvent.click(screen.getByRole('button', { name: en.coach.off }))
    view.unmount()
    render(<CoachMark screen="notes" />)
    expect(screen.queryByText(en.coach.screens.notes)).toBeNull()

    resetCoachMarks()
    render(<CoachMark screen="register" />)
    expect(screen.getByText(en.coach.screens.register)).toBeVisible()
  })

  it('says nothing on a screen it has no hint for', () => {
    const view = render(<CoachMark screen="styleguide" />)
    expect(view.container).toBeEmptyDOMElement()
  })
})
