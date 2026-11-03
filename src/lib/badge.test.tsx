import { describe, expect, it } from 'vitest'
import i18n from '@/i18n'
import { badgeDisc, badgeText, trayLine } from '@/lib/badge'

const t = i18n.t.bind(i18n)

describe('the badge', () => {
  it('holds a digit, and past nine says so rather than squeezing two', () => {
    expect(badgeText(1)).toBe('1')
    expect(badgeText(9)).toBe('9')
    expect(badgeText(10)).toBe('9+')
    expect(badgeText(240)).toBe('9+')
  })

  it('draws nothing where there is no canvas, rather than throwing', async () => {
    // jsdom has none: the taskbar is simply left alone.
    await expect(badgeDisc(3)).resolves.toBeNull()
  })
})

describe("the tray's line", () => {
  it('says what is going and what waits', () => {
    expect(trayLine({ running: 1, queued: 0 }, t)).toBe('kilna: 1 task going')
    expect(trayLine({ running: 3, queued: 7 }, t)).toBe('kilna: 3 tasks going, 7 waiting')
  })

  it('says when everything has finished, which is why the tray is still there', () => {
    expect(trayLine({ running: 0, queued: 0 }, t)).toBe('kilna: everything has finished')
  })

  it('agrees with the number in Russian', async () => {
    await i18n.changeLanguage('ru')
    expect(trayLine({ running: 2, queued: 5 }, t)).toBe('kilna: идут 2 задания, 5 ждут')
  })
})
