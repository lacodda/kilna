import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act } from '@testing-library/react'
import type { StyleBrick } from '@/lib/api/types'
import type { StyleForm } from '@/lib/styleDraft'
import { useStyleDraft } from '@/features/styles/useStyleDraft'
import { mockBackend, type Backend } from '@/test/backend'
import { renderHookWithin } from '@/test/render'
import { NOW } from '@/test/workspace'

/*
 * A style that writes itself as it is typed into, and takes in what is
 * written to it from elsewhere - the assistant keeping a description, an
 * undo putting a name back - without taking the words out from under the
 * person typing.
 */

const BRICK: StyleBrick = {
  id: 'b1',
  profile_id: 'p1',
  type_key: 'look',
  name: 'Dusk',
  description: null,
  hint: null,
  status: 'draft',
  created_at: NOW,
  updated_at: NOW,
  reference_count: 1,
  label: null,
  family: null,
  when_to_use: null,
  explanation: null,
  colours: [],
  sample: null,
  set_key: null,
  origin: 'own',
}

const LATER = '2026-09-27T12:00:00.000Z'

const pause = () => act(() => vi.advanceTimersByTimeAsync(600))

let backend: Backend

function draft(holdKey: (form: StyleForm) => boolean = () => false) {
  return renderHookWithin(
    ({ brick }: { brick: StyleBrick }) => useStyleDraft(brick, { holdKey, failure: 'save' }),
    { brick: BRICK },
  )
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    update_style_brick: ({ id, patch }) => ({
      ...BRICK,
      id,
      ...(patch as object),
      updated_at: LATER,
    }),
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('a style open in its editor', () => {
  it('writes what changed a moment after the typing pauses', async () => {
    const { result } = draft()
    act(() => result.current.edit({ description: 'Low sun' }))
    await act(() => vi.advanceTimersByTimeAsync(500))
    expect(backend.calls).toEqual([])
    await pause()

    expect(backend.argsOf('update_style_brick')).toEqual([
      { id: 'b1', patch: { description: 'Low sun' } },
    ])
    expect(result.current.status).toBe('saved')
  })

  it('writes a picked type or status at once: it is a decision, not a keystroke', async () => {
    const { result } = draft()
    act(() => result.current.edit({ status: 'ready' }, true))
    await act(() => vi.advanceTimersByTimeAsync(0))
    expect(backend.argsOf('update_style_brick')).toEqual([{ id: 'b1', patch: { status: 'ready' } }])
  })

  it('shows a description kept elsewhere, and leaves a field being typed alone', async () => {
    const { result, rerender } = draft()
    act(() => result.current.edit({ hint: 'only the water' }))

    // The assistant's description was kept while the steer was being typed.
    rerender({
      brick: {
        ...BRICK,
        description: 'Low sun, long reflections.',
        status: 'ready',
        updated_at: LATER,
      },
    })
    expect(result.current.form.description).toBe('Low sun, long reflections.')
    expect(result.current.form.status).toBe('ready')
    expect(result.current.form.hint).toBe('only the water')

    // And the typing is written on its own, not with the old description.
    await pause()
    expect(backend.argsOf('update_style_brick')).toEqual([
      { id: 'b1', patch: { hint: 'only the water' } },
    ])
  })

  it('does not put back old words from an answer that left before a write', async () => {
    const { result, rerender } = draft()
    act(() => result.current.edit({ name: 'Dusk over water' }, true))
    await act(() => vi.advanceTimersByTimeAsync(0))
    rerender({ brick: { ...BRICK } })
    expect(result.current.form.name).toBe('Dusk over water')
  })

  it('holds back a name the dictionary would refuse, and still writes the rest', async () => {
    const { result } = draft((form) => form.name.trim() === '')
    act(() => result.current.edit({ name: '  ', description: 'Low sun' }))
    await pause()
    expect(backend.argsOf('update_style_brick')).toEqual([
      { id: 'b1', patch: { description: 'Low sun' } },
    ])
  })

  it('writes what is pending when it is closed, and nothing once it is deleted', async () => {
    const { result, unmount } = draft()
    act(() => result.current.edit({ description: 'Low sun' }))
    unmount()
    await act(() => vi.advanceTimersByTimeAsync(0))
    expect(backend.argsOf('update_style_brick')).toHaveLength(1)

    const other = draft()
    act(() => other.result.current.edit({ description: 'Teal' }))
    act(() => other.result.current.forget())
    other.unmount()
    await act(() => vi.advanceTimersByTimeAsync(1000))
    expect(backend.argsOf('update_style_brick')).toHaveLength(1)
  })
})
