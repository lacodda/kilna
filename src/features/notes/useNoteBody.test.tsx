import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act } from '@testing-library/react'
import type { Note } from '@/lib/api/types'
import { say } from '@/lib/toast'
import { useNoteBody } from '@/features/notes/useNoteBody'
import { mockBackend, type Backend } from '@/test/backend'
import { renderHookWithin } from '@/test/render'
import { NOW } from '@/test/workspace'

const NOTE: Note = {
  id: 'n1',
  profile_id: 'p1',
  work_id: null,
  kind: 'note',
  title: 'Tides',
  body: '- [ ] find a hook',
  tags: [],
  created_at: NOW,
  updated_at: NOW,
}

const pause = () => act(() => vi.advanceTimersByTimeAsync(600))

let backend: Backend

function body(settle = vi.fn()) {
  const view = renderHookWithin(() => useNoteBody(NOTE, settle, 'save'))
  return { ...view, settle }
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({ update_note: ({ id, patch }) => ({ ...NOTE, id, ...(patch as object) }) })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe("a note's body", () => {
  it('writes into the same note a moment after the typing pauses', async () => {
    const { result, settle } = body()
    act(() => result.current.setText('- [ ] find a hook\n- [ ] ask about the bridge'))
    await act(() => vi.advanceTimersByTimeAsync(500))
    expect(backend.calls).toEqual([])
    await pause()

    expect(backend.argsOf('update_note')).toEqual([
      { id: 'n1', patch: { body: '- [ ] find a hook\n- [ ] ask about the bridge' } },
    ])
    expect(settle).toHaveBeenCalledTimes(1)
    expect(result.current.status).toBe('saved')
  })

  it('writes a ticked box at once: it is a decision, not a keystroke', async () => {
    const { result } = body()
    act(() => result.current.setText('- [x] find a hook', true))
    await act(() => vi.advanceTimersByTimeAsync(0))
    expect(backend.argsOf('update_note')).toHaveLength(1)
  })

  it('keeps writes in order, so a later one is never overtaken by an earlier', async () => {
    const written: string[] = []
    let finish: () => void = () => {}
    backend.answer('update_note', ({ patch }) => {
      const text = (patch as { body: string }).body
      if (written.length === 0) {
        return new Promise((resolve) => (finish = () => resolve(written.push(text))))
      }
      written.push(text)
      return null
    })
    const { result } = body()

    act(() => result.current.setText('first', true))
    await act(() => vi.advanceTimersByTimeAsync(0))
    // The first write is out; the second is typed while it is.
    act(() => result.current.setText('second', true))
    await act(() => vi.advanceTimersByTimeAsync(0))
    expect(backend.argsOf('update_note')).toHaveLength(1)
    await act(async () => finish())
    await act(() => vi.advanceTimersByTimeAsync(0))

    expect(written).toEqual(['first', 'second'])
  })

  it('writes what is pending when the note is left', async () => {
    const { result, unmount } = body()
    act(() => result.current.setText('left before the pause'))
    unmount()
    await act(() => vi.advanceTimersByTimeAsync(0))
    expect(backend.argsOf('update_note')).toHaveLength(1)
  })

  it('says a failed write failed, and does not claim to have saved', async () => {
    const failed = vi.spyOn(say, 'failedTo').mockImplementation(() => '')
    backend.answer('update_note', () => {
      throw { kind: 'other', message: 'disk full' }
    })
    const { result, settle } = body()
    act(() => result.current.setText('changed'))
    await pause()

    expect(failed).toHaveBeenCalledWith('save', expect.objectContaining({ message: 'disk full' }))
    expect(settle).not.toHaveBeenCalled()
    expect(result.current.status).toBe('idle')
  })
})
