import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act } from '@testing-library/react'
import type { NewVersion, Version } from '@/lib/api'
import { say } from '@/lib/toast'
import { useBodyEditing } from '@/lib/useBodyEditing'
import { mockBackend, type Args, type Backend } from '@/test/backend'
import { renderHookWithin } from '@/test/render'
import { NOW } from '@/test/workspace'

const OPEN: Version = {
  id: 'v1',
  work_id: 'w1',
  role: 'lyrics',
  revision: 1,
  label: null,
  body: 'first line',
  meta: {},
  parent_version_id: null,
  created_at: NOW,
}

/** Past the pause after the last keystroke, and every write it started. */
const pause = () => act(() => vi.advanceTimersByTimeAsync(600))

let backend: Backend
let minted: number

/** The backend's side of a new revision: numbered on from the open one. */
function mint({ workId, version }: Args) {
  minted += 1
  const asked = version as NewVersion
  return {
    ...OPEN,
    id: `v${minted + 1}`,
    work_id: workId as string,
    revision: minted + 1,
    body: asked.body,
    parent_version_id: asked.parent_version_id ?? null,
  }
}

function editing(onMinted = vi.fn()) {
  const view = renderHookWithin(() =>
    useBodyEditing({ workId: 'w1', role: 'lyrics', open: OPEN, onMinted, failure: 'save' }),
  )
  return { ...view, onMinted }
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] })
  vi.setSystemTime(new Date(NOW))
  minted = 0
  backend = mockBackend({ create_version: mint, update_version_body: () => null })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('text that saves itself', () => {
  it('waits for the typing to pause before it writes', async () => {
    const { result } = editing()
    act(() => result.current.setText('first line, changed'))
    await act(() => vi.advanceTimersByTimeAsync(500))
    expect(backend.calls).toEqual([])
    await pause()
    expect(backend.argsOf('create_version')).toHaveLength(1)
  })

  it('mints the next revision from the open one on the first change', async () => {
    const { result, onMinted } = editing()
    act(() => result.current.setText('first line, changed'))
    await pause()

    expect(backend.argsOf('create_version')).toEqual([
      {
        workId: 'w1',
        version: {
          role: 'lyrics',
          body: 'first line, changed',
          label: null,
          make_current: true,
          parent_version_id: 'v1',
        },
      },
    ])
    expect(onMinted).toHaveBeenCalledWith('v2')
    expect(result.current.status).toBe('saved')
  })

  it('puts the changes after it into the version it minted', async () => {
    const { result } = editing()
    act(() => result.current.setText('first line, changed'))
    await pause()
    act(() => result.current.setText('first line, changed again'))
    await pause()

    expect(backend.argsOf('create_version')).toHaveLength(1)
    expect(backend.argsOf('update_version_body')).toEqual([
      { id: 'v2', body: 'first line, changed again' },
    ])
  })

  it('puts a change typed while the version is being minted into it, not into a second one', async () => {
    let finish: (value: unknown) => void = () => {}
    backend.answer(
      'create_version',
      (args) => new Promise((resolve) => (finish = () => resolve(mint(args)))),
    )
    const { result } = editing()

    act(() => result.current.setText('one'))
    await pause()
    act(() => result.current.setText('one, two'))
    await pause()
    // The mint is still out; the second write waits behind it.
    expect(backend.argsOf('update_version_body')).toEqual([])
    await act(async () => finish(undefined))
    await pause()

    expect(backend.argsOf('create_version')).toHaveLength(1)
    expect(backend.argsOf('update_version_body')).toEqual([{ id: 'v2', body: 'one, two' }])
  })

  it('takes a version a score has frozen as the start of the next revision', async () => {
    const { result } = editing()
    act(() => result.current.setText('one'))
    await pause()
    backend.answer('update_version_body', () => {
      throw { kind: 'frozen', message: 'a score holds this text' }
    })
    act(() => result.current.setText('one, two'))
    await pause()

    const mints = backend.argsOf('create_version')
    expect(mints).toHaveLength(2)
    expect(mints[1]).toMatchObject({ version: { body: 'one, two', parent_version_id: 'v2' } })
  })

  it('writes what is pending when the text is left', async () => {
    const { result, unmount } = editing()
    act(() => result.current.setText('left before the pause'))
    unmount()
    await act(() => vi.advanceTimersByTimeAsync(0))

    expect(backend.argsOf('create_version')).toHaveLength(1)
  })

  it('says a failed write failed, and does not claim to have saved', async () => {
    const failed = vi.spyOn(say, 'failedTo').mockImplementation(() => '')
    backend.answer('create_version', () => {
      throw { kind: 'other', message: 'disk full' }
    })
    const { result } = editing()
    act(() => result.current.setText('first line, changed'))
    await pause()

    expect(failed).toHaveBeenCalledWith('save', expect.objectContaining({ message: 'disk full' }))
    expect(result.current.status).toBe('idle')
  })

  it('writes nothing when the text comes back to what is on disk', async () => {
    const { result } = editing()
    act(() => result.current.setText('first line, changed'))
    act(() => result.current.setText('first line'))
    await pause()
    expect(backend.calls).toEqual([])
  })
})
