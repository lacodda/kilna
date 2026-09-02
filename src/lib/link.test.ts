import { describe, expect, it } from 'vitest'
import { isWebLink, shortLink } from '@/lib/link'

describe('isWebLink', () => {
  it('accepts the two schemes a browser is for', () => {
    expect(isWebLink('https://example.invalid/a')).toBe(true)
    expect(isWebLink('http://example.invalid')).toBe(true)
  })

  it('ignores surrounding whitespace, which pasting leaves behind', () => {
    expect(isWebLink('  https://example.invalid/a  ')).toBe(true)
  })

  // The reason this function exists rather than a truthiness check: a release
  // url is typed by a person, and handing an arbitrary scheme to the shell lets
  // a note about a publication launch whatever is registered for it.
  it('refuses schemes that are not the web', () => {
    for (const url of [
      'file:///c:/windows/system32/calc.exe',
      'javascript:alert(1)',
      'mailto:someone@example.invalid',
      'data:text/html,<script>1</script>',
    ]) {
      expect(isWebLink(url), url).toBe(false)
    }
  })

  it('refuses what is not a url at all', () => {
    for (const url of ['', '   ', 'example.invalid', 'just some words']) {
      expect(isWebLink(url), JSON.stringify(url)).toBe(false)
    }
  })
})

describe('shortLink', () => {
  it('keeps the host when there is nothing else to say', () => {
    expect(shortLink('https://example.invalid')).toBe('example.invalid')
    expect(shortLink('https://example.invalid/')).toBe('example.invalid')
  })

  it('drops the www, which never tells two links apart', () => {
    expect(shortLink('https://www.example.invalid')).toBe('example.invalid')
  })

  it('keeps the path, which is what does tell them apart', () => {
    expect(shortLink('https://example.invalid/watch')).toBe('example.invalid/watch')
  })

  it('keeps the query, where some sites put the identity of the thing', () => {
    expect(shortLink('https://example.invalid/watch?v=abc')).toBe('example.invalid/watch?v=abc')
  })

  it('never exceeds the budget it is given, ellipsis included', () => {
    const long = `https://example.invalid/${'a'.repeat(200)}`

    const short = shortLink(long, 24)

    expect(short.length).toBeLessThanOrEqual(24)
    expect(short.endsWith('…')).toBe(true)
  })

  // Whatever was recorded is shown, even when it is not a url: hiding it would
  // leave the row claiming there is no link when the field is plainly not empty.
  it('shows text that is not a url as it was typed', () => {
    expect(shortLink('not a url')).toBe('not a url')
  })

  it('trims text that is not a url to the same budget', () => {
    expect(shortLink('x'.repeat(50), 10).length).toBeLessThanOrEqual(10)
  })
})
