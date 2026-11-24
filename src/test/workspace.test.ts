import { describe, expect, it } from 'vitest'
import { studio } from '@/test/workspace'

describe('the fake studio', () => {
  it('hands every test a profile of its own', () => {
    const first = studio()
    first.profile.config.work_kinds = []
    first.profile.config.prompts.length = 0

    // What one test did to its studio is not there for the next.
    const second = studio()
    expect(second.profile.config.work_kinds.length).toBeGreaterThan(0)
    expect(second.profile.config.prompts.length).toBeGreaterThan(0)
  })
})
