import { describe, expect, it } from 'vitest'
import {
  DEFAULT_TAB,
  DEFAULT_TAB_CHOICES,
  isTab,
  TABS,
  tabCounts,
  tabsOf,
} from '@/features/work/tabs'
import type { CardCounts } from '@/lib/api/types'
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

  it('know a tab by its name and nothing else', () => {
    expect(isTab('versions')).toBe(true)
    expect(isTab('lyrics')).toBe(false)
    expect(isTab(undefined)).toBe(false)
  })

  it('are each opened by the smoke test', () => {
    const opened = new Set(CARD_TABS.map(([, tab]) => tab))
    expect(TABS.filter((tab) => !opened.has(tab))).toEqual([])
  })

  describe('carry the numbers the card counted', () => {
    const none: CardCounts = {
      versions: 0,
      scores: 0,
      releases: 0,
      files: 0,
      sources: 0,
      derived: 0,
      notes: 0,
      comments: 0,
      comments_waiting: 0,
      scenes: 0,
      cuts: 0,
      cut_from: 0,
      history: 0,
    }

    it('beside the tab that lists them, and nowhere there is nothing', () => {
      expect(tabCounts(none)).toEqual({})
      const shown = tabCounts({ ...none, versions: 14, scores: 3, files: 7, history: 2 })
      expect(shown).toEqual({
        versions: { value: 14, waiting: false },
        score: { value: 3, waiting: false },
        files: { value: 7, waiting: false },
        history: { value: 2, waiting: false },
      })
      expect(shown.overview).toBeUndefined()
      expect(shown.assistant).toBeUndefined()
    })

    it('counting links made either way', () => {
      expect(tabCounts({ ...none, sources: 1 }).links?.value).toBe(1)
      expect(tabCounts({ ...none, sources: 1, derived: 2 }).links?.value).toBe(3)
    })

    it('saying how many comments wait when any do, and how many in all when none do', () => {
      expect(tabCounts({ ...none, comments: 5, comments_waiting: 2 }).comments).toEqual({
        value: 2,
        waiting: true,
      })
      expect(tabCounts({ ...none, comments: 5 }).comments).toEqual({ value: 5, waiting: false })
    })
  })
})
