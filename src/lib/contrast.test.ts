// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { initContrast, useContrast } from '@/lib/contrast'

/*
 * The window's contrast (v0.167): the theme's mode of more contrast, pinned
 * on or off by a class on <html> and left to the system otherwise.
 */

afterEach(() => {
  localStorage.clear()
  document.documentElement.className = ''
})

const pinned = () =>
  ['contrast-more', 'contrast-normal'].filter((name) =>
    document.documentElement.classList.contains(name),
  )

describe('the window contrast', () => {
  it("starts on the system's answer, with nothing pinned", () => {
    initContrast()
    expect(pinned()).toEqual([])
    expect(renderHook(() => useContrast()).result.current.contrast).toBe('system')
  })

  it('pins one answer at a time and keeps it for the next start', () => {
    const { result } = renderHook(() => useContrast())
    act(() => result.current.setContrast('more'))
    expect(pinned()).toEqual(['contrast-more'])

    act(() => result.current.setContrast('normal'))
    expect(pinned()).toEqual(['contrast-normal'])

    document.documentElement.className = ''
    initContrast()
    expect(pinned()).toEqual(['contrast-normal'])

    act(() => result.current.setContrast('system'))
    expect(pinned()).toEqual([])
  })
})
