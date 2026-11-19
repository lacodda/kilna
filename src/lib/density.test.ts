// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { initDensity, useDensity } from '@/lib/density'

/*
 * The window's density (v0.163): one attribute on <html> that dowel's sizes
 * follow, kept across starts, and nothing set for the regular one.
 */

afterEach(() => {
  localStorage.clear()
  document.documentElement.removeAttribute('data-density')
})

describe('the window density', () => {
  it('starts regular, with nothing on the root', () => {
    initDensity()
    expect(document.documentElement.hasAttribute('data-density')).toBe(false)
    const { result } = renderHook(() => useDensity())
    expect(result.current.density).toBe('regular')
  })

  it('sets the theme attribute and keeps it for the next start', () => {
    const { result } = renderHook(() => useDensity())
    act(() => result.current.setDensity('compact'))
    expect(document.documentElement.getAttribute('data-density')).toBe('compact')
    expect(result.current.density).toBe('compact')

    document.documentElement.removeAttribute('data-density')
    initDensity()
    expect(document.documentElement.getAttribute('data-density')).toBe('compact')

    act(() => result.current.setDensity('regular'))
    expect(document.documentElement.hasAttribute('data-density')).toBe(false)
  })

  it('reads an unknown stored word as regular', () => {
    localStorage.setItem('kilna.density', 'roomy')
    initDensity()
    expect(document.documentElement.hasAttribute('data-density')).toBe(false)
  })
})
