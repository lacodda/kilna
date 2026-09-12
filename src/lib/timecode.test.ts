import { describe, expect, it } from 'vitest'
import { formatSeconds, parseTimecode } from './timecode'

describe('parseTimecode', () => {
  it('reads seconds, minutes and hours', () => {
    expect(parseTimecode('83')).toEqual({ ok: true, seconds: 83 })
    expect(parseTimecode('1:23')).toEqual({ ok: true, seconds: 83 })
    expect(parseTimecode('0:04.5')).toEqual({ ok: true, seconds: 4.5 })
    expect(parseTimecode('1:00:00')).toEqual({ ok: true, seconds: 3600 })
    expect(parseTimecode('  2:05 ')).toEqual({ ok: true, seconds: 125 })
  })

  it('reads an empty field as unset', () => {
    expect(parseTimecode('')).toEqual({ ok: true, seconds: null })
    expect(parseTimecode('   ')).toEqual({ ok: true, seconds: null })
  })

  it('refuses what is not a timecode rather than guessing', () => {
    expect(parseTimecode('abc')).toEqual({ ok: false })
    expect(parseTimecode('-4')).toEqual({ ok: false })
    expect(parseTimecode('1:75')).toEqual({ ok: false })
    expect(parseTimecode('1.5:10')).toEqual({ ok: false })
    expect(parseTimecode('1:2:3:4')).toEqual({ ok: false })
  })
})

describe('formatSeconds', () => {
  it('writes m:ss, keeping a fraction only when there is one', () => {
    expect(formatSeconds(83)).toBe('1:23')
    expect(formatSeconds(4.5)).toBe('0:04.5')
    expect(formatSeconds(0)).toBe('0:00')
    expect(formatSeconds(3600)).toBe('60:00')
  })

  it('rounds to a tenth without reading 0:60', () => {
    expect(formatSeconds(59.96)).toBe('1:00')
    expect(formatSeconds(12.34)).toBe('0:12.3')
  })

  it('writes nothing for unset or nonsense', () => {
    expect(formatSeconds(null)).toBe('')
    expect(formatSeconds(-1)).toBe('')
    expect(formatSeconds(Number.NaN)).toBe('')
  })

  it('round-trips what parse read', () => {
    for (const text of ['1:23', '0:04.5', '60:00', '0:00']) {
      const parsed = parseTimecode(text)
      expect(parsed.ok && formatSeconds(parsed.seconds)).toBe(text)
    }
  })
})
