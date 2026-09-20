import { describe, expect, it } from 'vitest'
import { hrefOf, replaceWikiLinks, wikiLinks } from '@/lib/wikilink'

describe('wikiLinks', () => {
  it('reads a work and a version, with where each sat', () => {
    const body = 'see [[work:abc]] and [[version:def-9]] too'

    expect(wikiLinks(body)).toEqual([
      { target: 'work', id: 'abc', label: undefined, start: 4, end: 16 },
      { target: 'version', id: 'def-9', label: undefined, start: 21, end: 38 },
    ])
  })

  it('takes the label after a pipe', () => {
    expect(wikiLinks('[[work:abc|Harbour lights]]')[0] ?? {}).toMatchObject({
      id: 'abc',
      label: 'Harbour lights',
    })
  })

  it('treats an empty label as none, so nothing renders as a blank link', () => {
    expect(wikiLinks('[[work:abc|   ]]')[0]?.label).toBeUndefined()
  })

  it('ignores a kind it does not know rather than making a dead link', () => {
    expect(wikiLinks('[[note:abc]] [[scene:2]]')).toEqual([])
  })

  /* A person who writes a sentence in double brackets has not written a link,
     and turning it into a broken one would be worse than leaving it alone. */
  it('leaves prose in brackets alone', () => {
    expect(wikiLinks('[[see: the note about x]]')).toEqual([])
    expect(wikiLinks('[[work abc]]')).toEqual([])
    expect(wikiLinks('a [[ and a ]] apart')).toEqual([])
  })

  it('does not run across a line break', () => {
    expect(wikiLinks('[[work:abc\nand more]]')).toEqual([])
  })

  /* A module-level `g` regex carries `lastIndex` between calls, so the second
     reader of the same body would see half the links. */
  it('reads the same body the same way twice', () => {
    const body = '[[work:a]] and [[work:b]]'

    expect(wikiLinks(body)).toHaveLength(2)
    expect(wikiLinks(body)).toHaveLength(2)
  })
})

describe('replaceWikiLinks', () => {
  it('swaps the links and passes the rest through the text function', () => {
    const out = replaceWikiLinks(
      'before [[work:abc]] after',
      (link) => `<${link.id}>`,
      (between) => between.toUpperCase(),
    )

    expect(out).toBe('BEFORE <abc> AFTER')
  })

  it('escapes a body with no links at all', () => {
    expect(replaceWikiLinks('plain', () => '!', (between) => `[${between}]`)).toBe('[plain]')
  })

  it('handles two links with nothing between them', () => {
    expect(replaceWikiLinks('[[work:a]][[work:b]]', (link) => link.id)).toBe('ab')
  })
})

describe('hrefOf', () => {
  it('addresses a work by itself', () => {
    expect(hrefOf({ target: 'work', id: 'abc' })).toBe('/works/abc')
  })

  /* The card is addressed by the work, so a version needs the work it belongs
     to — guessing would open a card that does not exist. */
  it('addresses a version through the work it belongs to', () => {
    expect(hrefOf({ target: 'version', id: 'v1' }, () => 'w9')).toBe('/works/w9/versions')
  })

  it('gives nothing for a version nobody has looked up', () => {
    expect(hrefOf({ target: 'version', id: 'v1' })).toBeUndefined()
    expect(hrefOf({ target: 'version', id: 'v1' }, () => undefined)).toBeUndefined()
  })
})
