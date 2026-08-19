import { describe, expect, it } from 'vitest'
import { openWorkId } from './route'

describe('openWorkId', () => {
  it('finds the work in an open card', () => {
    expect(openWorkId('/works/abc')).toBe('abc')
  })

  it('finds it with a tab in the address too', () => {
    expect(openWorkId('/works/abc/score')).toBe('abc')
  })

  it('reads the list itself as nothing open', () => {
    expect(openWorkId('/works')).toBeUndefined()
    // The trailing slash is what a link written by hand produces.
    expect(openWorkId('/works/')).toBeUndefined()
  })

  it('does not mistake another screen for a work', () => {
    expect(openWorkId('/catalogue')).toBeUndefined()
    expect(openWorkId('/journal')).toBeUndefined()
    // A screen whose name merely starts the same way.
    expect(openWorkId('/workspaces/abc')).toBeUndefined()
  })
})
