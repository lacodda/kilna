import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, within } from '@testing-library/react'
import type { StyleBrick } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { MovementPick } from '@/features/work/tabs/frame/MovementPick'
import { createQueryClient } from '@/lib/query/client'
import { ProfileContext } from '@/lib/useProfile'
import { mockBackend } from '@/test/backend'
import { answersFor, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The movements of a loop (v0.136): picked from the Motion branch, up to
 * five, a camera move marked while the camera holds still.
 */

function mounted(chosen: string[], onChange: (next: string[]) => void) {
  const workspace = studio()
  const movement = (id: string, name: string, extra: Partial<StyleBrick> = {}): StyleBrick => ({
    ...workspace.bricks[0]!,
    id,
    name,
    type_key: 'motion-camera',
    family: 'travel',
    description: `MOTION: ${name.toLowerCase()}.`,
    ...extra,
  })
  workspace.bricks.push(
    movement('m-push', 'Slow push-in', { contract: { provides: ['camera:moving'] } }),
    movement('m-rack', 'Rack focus'),
  )
  mockBackend({ ...answersFor(workspace) })
  render(
    <Providers client={createQueryClient()}>
      <ProfileContext.Provider value={workspace.profile}>
        <MovementPick chosen={chosen} stillCamera onChange={onChange} />
      </ProfileContext.Provider>
    </Providers>,
  )
}

describe('the movements of a loop', () => {
  it('adds a movement from the Motion branch and says a camera move', async () => {
    const onChange = vi.fn()
    mounted([], onChange)
    fireEvent.click(screen.getByRole('button', { name: en.frame.addMovement }))
    const push = (await within(document.body).findByText('Slow push-in')).closest('button')!
    expect(within(push).getByText(en.frame.movesCamera)).toBeVisible()
    fireEvent.click(push)
    expect(onChange).toHaveBeenCalledWith(['m-push'])
  })

  it('takes no more than five', () => {
    mounted(['a', 'b', 'c', 'd', 'e'], () => {})
    expect(screen.getByRole('button', { name: /Five movements/ })).toBeDisabled()
  })
})
