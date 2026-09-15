import { describe, expect, it } from 'vitest'
import { formatDuration } from '@/lib/runs'

describe('formatDuration', () => {
  it('reads in seconds under a minute', () => {
    expect(formatDuration(4200)).toBe('4s')
    expect(formatDuration(59_400)).toBe('59s')
  })

  it('reads in minutes and seconds, zero-padded', () => {
    expect(formatDuration(72_000)).toBe('1m 12s')
    expect(formatDuration(605_000)).toBe('10m 05s')
  })

  it('reads in hours past the hour', () => {
    expect(formatDuration(3_900_000)).toBe('1h 05m')
  })

  it('says less than a second rather than none', () => {
    // A run that did happen must not read as instant.
    expect(formatDuration(400)).toBe('<1s')
  })

  it('has nothing to say about nothing', () => {
    expect(formatDuration(null)).toBeNull()
    expect(formatDuration(Number.NaN)).toBeNull()
    expect(formatDuration(-5)).toBeNull()
  })
})
