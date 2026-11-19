// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useGrowingWindow, WINDOW_STEP } from '@/lib/useGrowingWindow'

/*
 * A long list drawn a step at a time (v0.163): the first step, one more each
 * time the sentinel comes near, never past the end; everything at once where
 * nothing can observe.
 */

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('a growing window over a long list', () => {
  it('draws a step, and one more each time the sentinel is reached', () => {
    let fire: (() => void) | null = null
    class Observer {
      constructor(callback: (entries: { isIntersecting: boolean }[]) => void) {
        fire = () => callback([{ isIntersecting: true }])
      }
      observe() {}
      disconnect() {}
    }
    vi.stubGlobal('IntersectionObserver', Observer)

    const { result } = renderHook(() => useGrowingWindow(400))
    expect(result.current.shown).toBe(WINDOW_STEP)

    act(() => result.current.sentinel(document.createElement('tr')))
    act(() => fire?.())
    expect(result.current.shown).toBe(WINDOW_STEP * 2)
    act(() => fire?.())
    expect(result.current.shown).toBe(400)
  })

  it('draws every row where nothing can observe', () => {
    vi.stubGlobal('IntersectionObserver', undefined)
    const { result } = renderHook(() => useGrowingWindow(400))
    expect(result.current.shown).toBe(400)
  })
})
