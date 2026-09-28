import { describe, expect, it } from 'vitest'
import { sectionLines } from '@/lib/sections'

describe('sectionLines', () => {
  it('finds a header on a line of its own', () => {
    expect(sectionLines('[Verse]\nfirst line\nsecond line\n\n[Chorus]\nhook')).toEqual([0, 4])
  })

  it('takes a header that carries directions after a colon', () => {
    expect(sectionLines('[Intro: soft piano, spoken]\nwords')).toEqual([0])
  })

  it('allows space around the brackets', () => {
    expect(sectionLines('  [Bridge]  \nwords')).toEqual([0])
  })

  it('leaves a bracket inside a line alone', () => {
    // A stage direction in the middle of a sung line is part of the line.
    expect(sectionLines('I said [quietly] no\n[Chorus] and then words')).toEqual([])
  })

  it('does not take an empty pair of brackets', () => {
    expect(sectionLines('[]\nwords')).toEqual([])
  })

  it('reads CRLF line endings the same as LF', () => {
    // A text pasted from a Windows editor keeps its carriage returns, and the
    // header before one is still a header.
    expect(sectionLines('[Verse]\r\nwords\r\n[Chorus]\r\n')).toEqual([0, 2])
  })

  it('finds nothing in a text without sections', () => {
    expect(sectionLines('warm synth pop, slow build, airy female vocal')).toEqual([])
  })

  it('finds nothing in nothing', () => {
    expect(sectionLines('')).toEqual([])
  })
})
