import { describe, expect, it } from 'vitest'
import {
  type CardFacts,
  DEFAULT_TAB,
  DEFAULT_TAB_CHOICES,
  factsOf,
  isTab,
  NOTHING_COUNTED,
  type Tab,
  TABS,
  tabCounts,
  tabsOf,
} from '@/features/work/tabs'
import type { CardCounts, Work } from '@/lib/api/types'
import { CARD_TABS } from '@/test/places'
import { answersFor, IDS, studio } from '@/test/workspace'

/** Facts with nothing named and nothing held, and whatever a test sets. */
function facts(
  names: Partial<CardFacts['names']> = {},
  holds: Partial<CardFacts['holds']> = {},
): CardFacts {
  return {
    names: {
      roles: false,
      axes: false,
      storyboard: false,
      doors: false,
      cover: false,
      frame: false,
      ...names,
    },
    holds: {
      versions: false,
      scores: false,
      scenes: false,
      releases: false,
      files: false,
      cover: false,
      splice: false,
      ...holds,
    },
  }
}

describe("the card's tabs", () => {
  it('leave out a storyboard a kind has not got, and a splice a work was not cut from', () => {
    const bare = tabsOf(facts())
    expect(bare).not.toContain('scenes')
    expect(bare).not.toContain('cuts')
    expect(tabsOf(facts({ storyboard: true }))).toContain('scenes')
    expect(tabsOf(facts({}, { splice: true }))).toContain('cuts')
    // In the list's order, whatever is left out.
    const everything = facts(
      { roles: true, axes: true, storyboard: true, doors: true, cover: true, frame: true },
      { splice: true },
    )
    expect(tabsOf(everything)).toEqual([...TABS])
  })

  it('stand where the kind names what each is for', () => {
    const rules: [keyof CardFacts['names'], ...Tab[]][] = [
      ['roles', 'versions'],
      ['axes', 'score'],
      ['storyboard', 'scenes'],
      ['doors', 'releases', 'files'],
      ['cover', 'cover'],
      ['frame', 'frame'],
    ]
    for (const [name, ...tabs] of rules) {
      const drawn = tabsOf(facts({ [name]: true }))
      for (const tab of tabs) expect(drawn, `${name} draws ${tab}`).toContain(tab)
      expect(tabsOf(facts()).filter((tab) => tabs.includes(tab))).toEqual([])
    }
  })

  it('stand where the work holds rows only they show, whatever its kind names', () => {
    const rules: [keyof CardFacts['holds'], Tab][] = [
      ['versions', 'versions'],
      ['scores', 'score'],
      ['scenes', 'scenes'],
      ['releases', 'releases'],
      ['files', 'files'],
      ['cover', 'cover'],
      ['splice', 'cuts'],
    ]
    for (const [held, tab] of rules) {
      expect(tabsOf(facts({}, { [held]: true })), `holding ${held}`).toContain(tab)
    }
    // Nothing is held in a frame a kind does not name: the backend refuses one.
    expect(tabsOf(facts({}, { cover: true, releases: true }))).not.toContain('frame')
  })

  it("open on a tab a person lives in, never on one of a single kind's making", () => {
    expect(DEFAULT_TAB_CHOICES).toContain(DEFAULT_TAB)
    for (const tab of ['scenes', 'cuts', 'cover', 'frame'] as const) {
      expect(DEFAULT_TAB_CHOICES).not.toContain(tab)
    }
    // A habit kept across v0.86: a work without the tab opens on the overview.
    for (const tab of ['versions', 'score', 'releases', 'files'] as const) {
      expect(DEFAULT_TAB_CHOICES).toContain(tab)
    }
  })

  it('know a tab by its name and nothing else', () => {
    expect(isTab('versions')).toBe(true)
    expect(isTab('frame')).toBe(true)
    expect(isTab('lyrics')).toBe(false)
    expect(isTab(undefined)).toBe(false)
  })

  it('are each opened by the smoke test', () => {
    const opened = new Set(CARD_TABS.map(([, tab]) => tab))
    expect(TABS.filter((tab) => !opened.has(tab))).toEqual([])
  })

  describe('read off a work of the studio', () => {
    const workspace = studio()
    const config = workspace.profile.config
    const answers = answersFor(workspace)
    const workOf = (id: string) => workspace.works.find((one) => one.id === id)!
    const countsOf = (id: string) => answers.card_counts!({ workId: id }) as CardCounts
    const drawn = (id: string, counts: CardCounts = countsOf(id)) =>
      tabsOf(factsOf(config, workOf(id), counts))

    it('give a song its text and its score, and nothing it goes out with', () => {
      // A song goes out only as what is made from it (ADR 0047).
      expect(drawn(IDS.song)).toEqual([
        'overview',
        'versions',
        'score',
        'links',
        'notes',
        'comments',
        'assistant',
        'history',
      ])
    })

    it('give an audio release its cover, its frame and its doors, and no board or score', () => {
      const audio = drawn(IDS.audio)
      for (const tab of ['cover', 'frame', 'releases', 'files'] as const) {
        expect(audio).toContain(tab)
      }
      // Its kind names roles - a concept, a context - so its versions stand.
      expect(audio).toContain('versions')
      expect(audio).not.toContain('scenes')
      expect(audio).not.toContain('score')
      // Made from its song whole, under one frame: there is nothing to cut.
      expect(audio).not.toContain('cuts')
    })

    it('give a video its board and its cover, and no frame', () => {
      const video = drawn(IDS.video)
      for (const tab of ['versions', 'scenes', 'cover', 'cuts', 'score', 'releases'] as const) {
        expect(video).toContain(tab)
      }
      expect(video).not.toContain('frame')
    })

    it('keep a tab a song holds rows for, though its kind no longer names it', () => {
      const song = countsOf(IDS.song)
      expect(drawn(IDS.song, { ...song, releases: 1 })).toContain('releases')
      expect(drawn(IDS.song, { ...song, files: 2 })).toContain('files')
      expect(drawn(IDS.song, { ...song, scenes: 3 })).toContain('scenes')
      const covered: Work = { ...workOf(IDS.song), cover: { picture: 'a lantern' } }
      expect(tabsOf(factsOf(config, covered, song))).toContain('cover')
      // A block written as nothing is nothing held.
      const blank: Work = { ...workOf(IDS.song), cover: { picture: '  ' } }
      expect(tabsOf(factsOf(config, blank, song))).not.toContain('cover')
    })

    it('trust an address until the counts are in', () => {
      // No counts at all - `drawn` would count them.
      const unknown = tabsOf(factsOf(config, workOf(IDS.song)))
      for (const tab of ['releases', 'files', 'scenes', 'cuts'] as const) {
        expect(unknown).toContain(tab)
      }
      // What the kind and the work say is known already.
      expect(unknown).not.toContain('frame')
      expect(unknown).not.toContain('cover')
      // The bar is drawn from what is known.
      expect(drawn(IDS.song, NOTHING_COUNTED)).not.toContain('releases')
    })
  })

  describe('carry the numbers the card counted', () => {
    const none = NOTHING_COUNTED

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
