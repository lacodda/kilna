import { describe, expect, it } from 'vitest'
import { DEFAULT_TAB, DEFAULT_TAB_CHOICES, isHeld, isTab, TABS, tabsOf } from '@/features/work/tabs'
import { CARD_TABS } from '@/test/places'

describe("the card's tabs", () => {
  it('leave out a storyboard a kind has not got, and a splice a work was not cut from', () => {
    const song = tabsOf({ storyboard: false, splice: false })
    expect(song).not.toContain('scenes')
    expect(song).not.toContain('cuts')
    expect(tabsOf({ storyboard: true, splice: false })).toContain('scenes')
    expect(tabsOf({ storyboard: false, splice: true })).toContain('cuts')
    // In the list's order, whatever is left out.
    expect(tabsOf({ storyboard: true, splice: true })).toEqual([...TABS])
  })

  it('open on a tab every work has', () => {
    expect(DEFAULT_TAB_CHOICES).toContain(DEFAULT_TAB)
    expect(DEFAULT_TAB_CHOICES).not.toContain('scenes')
    expect(DEFAULT_TAB_CHOICES).not.toContain('cuts')
    expect(tabsOf({ storyboard: false, splice: false })).toEqual([...DEFAULT_TAB_CHOICES])
  })

  it('give the two-column tabs the card height', () => {
    expect(TABS.filter(isHeld)).toEqual(['versions', 'score', 'comments'])
  })

  it('know a tab by its name and nothing else', () => {
    expect(isTab('versions')).toBe(true)
    expect(isTab('lyrics')).toBe(false)
    expect(isTab(undefined)).toBe(false)
  })

  it('are each opened by the smoke test', () => {
    const opened = new Set(CARD_TABS.map(([, tab]) => tab))
    expect(TABS.filter((tab) => !opened.has(tab))).toEqual([])
  })
})
