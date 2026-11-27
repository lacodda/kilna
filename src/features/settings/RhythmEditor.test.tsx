import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import type { Rhythm } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { RhythmEditor } from '@/features/settings/RhythmEditor'
import { mockBackend } from '@/test/backend'
import { answersFor, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The two windows of a planned slot (v0.169, FR-11): how early it reminds
 * and how near its day a booking closes it - set in the window, where until
 * now they were only in the profile's file.
 */

function mounted(rhythm: Rhythm | null) {
  const workspace = studio()
  mockBackend(answersFor(workspace))
  const config = { ...workspace.profile.config, rhythm }
  const onChange = vi.fn<(next: Rhythm | null) => void>()
  render(
    <Providers client={createQueryClient()}>
      <RhythmEditor config={config} onChange={onChange} />
    </Providers>,
  )
  return onChange
}

const RHYTHM: Rhythm = { every_days: 3, default_time: null, monthly: [] }

describe('the windows of a planned slot in the rhythm', () => {
  it('stand at a week until they are set, and are told as whole days', () => {
    const onChange = mounted(RHYTHM)
    const lead = screen.getByRole('textbox', { name: en.editor.rhythmPlanLead })
    const near = screen.getByRole('textbox', { name: en.editor.rhythmPlanWindow })
    expect(lead).toHaveValue('7')
    expect(near).toHaveValue('7')

    fireEvent.change(lead, { target: { value: '10' } })
    fireEvent.blur(lead)
    expect(onChange).toHaveBeenLastCalledWith({ ...RHYTHM, plan_lead_days: 10 })

    fireEvent.change(near, { target: { value: '3' } })
    fireEvent.blur(near)
    expect(onChange).toHaveBeenLastCalledWith({ ...RHYTHM, plan_match_window_days: 3 })
  })

  it('are not offered while the profile keeps no rhythm', () => {
    mounted(null)
    expect(screen.queryByRole('textbox', { name: en.editor.rhythmPlanLead })).toBeNull()
  })
})
