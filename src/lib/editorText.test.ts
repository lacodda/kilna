// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { initEditorText, useEditorText } from '@/lib/editorText'
import { textMetrics } from '@/features/work/tabs/versions/metrics'

/*
 * The editor's text (v0.163): a size for every body and a face for a sung
 * one, on <html>; a style prompt stays monospace.
 */

afterEach(() => {
  localStorage.clear()
  document.documentElement.removeAttribute('data-editor-font')
  document.documentElement.removeAttribute('data-editor-size')
})

describe('the editor text', () => {
  it('sets nothing for the defaults and keeps a choice for the next start', () => {
    initEditorText()
    const root = document.documentElement
    expect(root.hasAttribute('data-editor-font')).toBe(false)
    expect(root.hasAttribute('data-editor-size')).toBe(false)

    const { result } = renderHook(() => useEditorText())
    act(() => result.current.setFont('sans'))
    act(() => result.current.setSize('large'))
    expect(root.getAttribute('data-editor-font')).toBe('sans')
    expect(root.getAttribute('data-editor-size')).toBe('large')
    expect(result.current.font).toBe('sans')

    root.removeAttribute('data-editor-font')
    root.removeAttribute('data-editor-size')
    initEditorText()
    expect(root.getAttribute('data-editor-font')).toBe('sans')
    expect(root.getAttribute('data-editor-size')).toBe('large')
  })

  it('sets a sung text in the chosen face and a style prompt in monospace', () => {
    expect(textMetrics(false, true)).toContain('editor-font')
    expect(textMetrics(false, true)).not.toContain('font-mono')
    expect(textMetrics(false)).toContain('font-mono')
    expect(textMetrics(true, true)).not.toContain('editor-font')
    for (const one of [textMetrics(false), textMetrics(true), textMetrics(false, true)]) {
      expect(one).toContain('editor-size')
    }
  })
})
