import { describe, expect, it } from 'vitest'
import { coverFor, coverImageFor } from '@/lib/cover'

describe('coverImageFor', () => {
  it('is the work’s own gradient when there is no picture', () => {
    expect(coverImageFor('w1', undefined)).toBe(coverFor('w1'))
  })

  it('lays the picture over the gradient rather than replacing it', () => {
    const value = coverImageFor('w1', 'http://asset.localhost/c/media/a.png')
    expect(value).toContain('url("http://asset.localhost/c/media/a.png")')
    // A cover still loading, or one whose file went missing, must not leave
    // a white hole where the work's colour was.
    expect(value).toContain(coverFor('w1'))
  })

  it('escapes what would end the CSS string early', () => {
    // A quote would close the url() and leave the rest as rubbish CSS; a
    // backslash would escape whatever follows it.
    const value = coverImageFor('w1', 'http://asset.localhost/a"b\\c.png')
    expect(value).toContain('a\\"b\\\\c.png')
    expect(value.match(/url\(/g)).toHaveLength(1)
  })
})
