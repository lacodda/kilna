// @vitest-environment jsdom
import { describe, expect, it } from 'vitest'
import { initSealedRoot } from '@/lib/sealRoot'

describe('the sealed page', () => {
  it('is put back when something scrolls it, and leaves a screen to scroll itself', () => {
    initSealedRoot()

    document.documentElement.scrollLeft = 573
    document.body.scrollTop = 40
    document.dispatchEvent(new Event('scroll'))
    expect(document.documentElement.scrollLeft).toBe(0)
    expect(document.body.scrollTop).toBe(0)

    const screen = document.createElement('div')
    document.body.append(screen)
    screen.scrollTop = 120
    screen.dispatchEvent(new Event('scroll'))
    expect(screen.scrollTop).toBe(120)
    screen.remove()
  })
})
