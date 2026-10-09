import { describe, expect, it } from 'vitest'
import type {
  Going,
  MetaField,
  ProfileConfig,
  ScheduledRelease,
  Score,
  VersionRole,
  VersionSummary,
  WidgetPlacement,
} from '@/lib/api/types'
import {
  DEFAULT_PLACEMENT,
  WIDGETS,
  applies,
  boardOf,
  currentIn,
  doorsOf,
  fieldsOf,
  leadOf,
  nextStage,
  previewOf,
  previousOf,
  publicationFact,
  releaseTone,
  styleRoleOf,
  takesAn,
  textRoleOf,
  trailOf,
  wordOf,
  type Placed,
  type WidgetFacts,
} from '@/lib/overview'

const place = (id: string, size: 's' | 'm' | 'l', position: number): WidgetPlacement => ({
  id,
  size,
  position,
})

describe('boardOf', () => {
  it("reads no overview as the owner's choice: the lead layout and the shipped placement", () => {
    for (const overview of [undefined, null]) {
      const board = boardOf({ overview })
      expect(board.layout).toBe('lead')
      expect(board.widgets.map((w) => w.id)).toEqual(DEFAULT_PLACEMENT.map((p) => p.id))
    }
  })

  it('ships every widget of the catalogue once', () => {
    const ids = DEFAULT_PLACEMENT.map((p) => p.id)
    expect([...ids].sort()).toEqual([...WIDGETS].sort())
    expect(new Set(ids).size).toBe(ids.length)
  })

  it('keeps a chosen layout with the shipped placement when the list is empty', () => {
    const board = boardOf({ overview: { layout: 'mosaic', widgets: [] } })
    expect(board.layout).toBe('mosaic')
    expect(board.widgets).toHaveLength(DEFAULT_PLACEMENT.length)
  })

  it('orders by position, skips what this build does not know, and draws a widget once', () => {
    const board = boardOf({
      overview: {
        layout: 'grid',
        widgets: [
          place('stage', 's', 2),
          place('sparkles', 'l', 0),
          place('text', 'l', 1),
          place('stage', 'm', 3),
          place('score', 'm', 1),
        ],
      },
    })
    // Ties keep the order they are listed in: text before score.
    expect(board.widgets).toEqual([
      { id: 'text', size: 'l' },
      { id: 'score', size: 'm' },
      { id: 'stage', size: 's' },
    ])
  })

  it('falls back to the lead layout for a name it does not know', () => {
    const overview = { layout: 'carousel', widgets: [] } as unknown as ProfileConfig['overview']
    expect(boardOf({ overview }).layout).toBe('lead')
  })
})

describe('applies', () => {
  const none: WidgetFacts = {
    scored: false,
    text: false,
    style: false,
    prose: false,
    releases: false,
    publications: false,
    scenes: false,
    cover: false,
    findings: false,
    repeats: false,
  }

  it('draws only what the work always has when the kind names nothing', () => {
    const drawn = WIDGETS.filter((id) => applies(id, none))
    expect(drawn.sort()).toEqual(['fields', 'links', 'recent', 'stage'])
  })

  it('draws the cover where a picture can be set or already is', () => {
    expect(applies('cover', none)).toBe(false)
    expect(applies('cover', { ...none, cover: true })).toBe(true)
  })

  it('draws no storyboard on a song and one on a clip', () => {
    expect(applies('storyboard', none)).toBe(false)
    expect(applies('storyboard', { ...none, scenes: true })).toBe(true)
  })

  it('draws the score, its axes and its trend together, off the kind having axes', () => {
    for (const id of ['score', 'axes', 'trend'] as const) {
      expect(applies(id, none)).toBe(false)
      expect(applies(id, { ...none, scored: true })).toBe(true)
    }
  })

  it('draws the findings only while there is something to say', () => {
    expect(applies('findings', none)).toBe(false)
    expect(applies('findings', { ...none, findings: true })).toBe(true)
  })

  it("draws the guard's findings only while the song has one", () => {
    expect(applies('repeats', none)).toBe(false)
    expect(applies('repeats', { ...none, repeats: true })).toBe(true)
  })

  it("draws a song's publications where a clip draws its release", () => {
    const song = { ...none, publications: true }
    const clip = { ...none, releases: true }
    expect([applies('publications', song), applies('release', song)]).toEqual([true, false])
    expect([applies('publications', clip), applies('release', clip)]).toEqual([false, true])
  })
})

describe('the shipped placement', () => {
  it("leads a song's board with its publications, across the lead column", () => {
    const board = boardOf({ overview: null })
    const ids = board.widgets.map((widget) => widget.id)
    expect(ids.indexOf('publications')).toBe(ids.indexOf('findings') + 1)
    expect(board.widgets.find((widget) => widget.id === 'publications')?.size).toBe('l')
  })

  it("pairs the guard's findings with what needs attention, first in the lead column", () => {
    const board = boardOf({ overview: null })
    const ids = board.widgets.map((widget) => widget.id)
    expect(ids.slice(0, 2)).toEqual(['repeats', 'findings'])
    const { lead } = leadOf(board.widgets.slice(0, 2))
    expect(lead).toEqual([
      {
        kind: 'pair',
        widgets: [
          { id: 'repeats', size: 'm' },
          { id: 'findings', size: 'm' },
        ],
      },
    ])
  })

  it("leads a publication's board with its release, where a song's leads with its publications", () => {
    const board = boardOf({ overview: null })
    const ids = board.widgets.map((widget) => widget.id)
    expect(ids.indexOf('release')).toBe(ids.indexOf('publications') + 1)
    expect(board.widgets.find((widget) => widget.id === 'release')?.size).toBe('l')
  })
})

describe('leadOf', () => {
  const w = (id: Placed['id'], size: Placed['size']): Placed => ({ id, size })

  it("draws the owner's picture: the text across the lead, a pair under it, figures on the rail", () => {
    const { lead, rail } = leadOf([
      w('text', 'l'),
      w('score', 's'),
      w('stage', 's'),
      w('style', 'm'),
      w('fields', 'm'),
      w('links', 's'),
      w('recent', 's'),
    ])
    expect(lead).toEqual([
      { kind: 'one', widget: w('text', 'l') },
      { kind: 'pair', widgets: [w('style', 'm'), w('fields', 'm')] },
    ])
    expect(rail.map((x) => x.id)).toEqual(['score', 'stage', 'links', 'recent'])
  })

  it('gives a two-across widget with no partner the whole width', () => {
    const { lead } = leadOf([w('findings', 'm'), w('text', 'l'), w('hook', 'm')])
    expect(lead).toEqual([
      { kind: 'one', widget: w('findings', 'm') },
      { kind: 'one', widget: w('text', 'l') },
      { kind: 'one', widget: w('hook', 'm') },
    ])
  })

  it('pairs across a figure that went to the rail', () => {
    const { lead, rail } = leadOf([w('style', 'm'), w('score', 's'), w('fields', 'm')])
    expect(lead).toEqual([{ kind: 'pair', widgets: [w('style', 'm'), w('fields', 'm')] }])
    expect(rail).toEqual([w('score', 's')])
  })
})

describe('the roles the text widgets read', () => {
  const roles: VersionRole[] = [
    { key: 'review', label: 'Review', comments_on: 'lyrics' },
    { key: 'lyrics', label: 'Lyrics' },
    { key: 'style', label: 'Style prompt', counts_as_version: false },
  ]

  it('reads the text from the first draft role and the style from the one written about it', () => {
    expect(textRoleOf(roles)?.key).toBe('lyrics')
    expect(styleRoleOf(roles)?.key).toBe('style')
  })

  it('finds no style where the kind keeps none', () => {
    expect(styleRoleOf([{ key: 'plot', label: 'Plot' }])).toBeUndefined()
    expect(textRoleOf([{ key: 'style', label: 'Style', counts_as_version: false }])).toBeUndefined()
  })
})

function summary(over: Partial<VersionSummary> & Pick<VersionSummary, 'id'>): VersionSummary {
  return {
    trial_id: null,
    work_id: 'w1',
    role: 'lyrics',
    revision: 1,
    label: null,
    length: 10,
    parent_version_id: null,
    created_at: '2026-09-01T10:00:00Z',
    is_current: false,
    about_version_id: null,
    ...over,
  }
}

describe('currentIn and previousOf', () => {
  const first = summary({ id: 'v1', revision: 1 })
  const second = summary({
    id: 'v2',
    revision: 2,
    parent_version_id: 'v1',
    created_at: '2026-09-02T10:00:00Z',
  })
  const third = summary({ id: 'v3', revision: 3, created_at: '2026-09-03T10:00:00Z' })
  const style = summary({ id: 's1', role: 'style', is_current: true })

  it("shows the work's current version when it is in the role", () => {
    const versions = [first, { ...second, is_current: true }, third]
    expect(currentIn(versions, 'lyrics')?.id).toBe('v2')
  })

  it('shows the newest of the role when the current one is in another', () => {
    expect(currentIn([first, second, third, style], 'lyrics')?.id).toBe('v3')
    expect(currentIn([first, style], 'style')?.id).toBe('s1')
    expect(currentIn([first], 'style')).toBeUndefined()
  })

  it('compares with the version it was written from, else the revision before', () => {
    expect(previousOf([first, second, third], second)?.id).toBe('v1')
    expect(previousOf([first, second, third], third)?.id).toBe('v2')
    expect(previousOf([first, second, third, style], first)).toBeUndefined()
  })
})

describe('fieldsOf', () => {
  it('puts the paragraphs in the hook widget and the rest in the grid', () => {
    const fields: MetaField[] = [
      { key: 'bpm', label: 'BPM', type: 'number' },
      { key: 'premise', label: 'Premise', type: 'multiline' },
      { key: 'explicit', label: 'Explicit', type: 'boolean' },
    ]
    const { short, prose } = fieldsOf(fields)
    expect(short.map((f) => f.key)).toEqual(['bpm', 'explicit'])
    expect(prose.map((f) => f.key)).toEqual(['premise'])
  })
})

describe('trailOf', () => {
  it('turns a newest-first history into the last few totals, oldest first', () => {
    const score = (id: string, total: number): Score => ({
      id,
      work_id: 'w1',
      version_id: null,
      axes: {},
      total,
      tier: null,
      note: null,
      rater: null,
      scored_at: '2026-09-01T10:00:00Z',
      revision: null,
    })
    const history = [score('c', 91), score('b', 86), score('a', 78), score('z', 60)]
    expect(trailOf(history, 3).map((s) => s.total)).toEqual([78, 86, 91])
    expect(trailOf([], 3)).toEqual([])
  })
})

describe('nextStage', () => {
  const config = {
    stages: [
      { key: 'idea', label: 'Idea', percent: 0 },
      { key: 'polish', label: 'Polishing', percent: 67 },
      { key: 'done', label: 'Finished', percent: 100 },
    ],
  } as ProfileConfig

  it('names the stop after the one reached, and none at the last or while unset', () => {
    expect(nextStage(config, 70)?.key).toBe('done')
    expect(nextStage(config, 0)?.key).toBe('polish')
    expect(nextStage(config, 100)).toBeUndefined()
    expect(nextStage(config, null)).toBeUndefined()
  })
})

describe('releaseTone', () => {
  const release = (over: Partial<ScheduledRelease>): ScheduledRelease =>
    ({
      status: 'planned',
      scheduled_at: null,
      released_at: null,
      readiness: { roles: [], scored: true, ready: true },
      ...over,
    }) as ScheduledRelease

  it('is good once out, information when booked and ready, a warning otherwise', () => {
    expect(releaseTone(release({ status: 'released' }))).toBe('good')
    expect(releaseTone(release({ scheduled_at: '2026-09-20' }))).toBe('info')
    expect(releaseTone(release({}))).toBe('warn')
    expect(
      releaseTone(
        release({
          scheduled_at: '2026-09-20',
          readiness: { roles: [], scored: false, ready: false },
        }),
      ),
    ).toBe('warn')
  })
})

describe('previewOf', () => {
  it('cuts at a line, and says whether there is more', () => {
    expect(previewOf('one\ntwo\nthree', 2)).toEqual({ text: 'one\ntwo', more: true })
    expect(previewOf('one\ntwo\n\n\n', 2)).toEqual({ text: 'one\ntwo', more: false })
  })
})

describe('publicationFact', () => {
  const TODAY = '2026-09-15'
  const going = (fields: Partial<Going>): { release: Going } => ({
    release: {
      id: 'r',
      kind: 'youtube',
      status: 'planned',
      scheduled_at: null,
      scheduled_time: null,
      released_at: null,
      url: null,
      ...fields,
    },
  })

  it('says it is out, with the day it went out', () => {
    const out = going({
      status: 'released',
      released_at: '2026-09-02T09:00:00Z',
      scheduled_at: '2026-09-02',
    })
    expect(publicationFact(out, TODAY)).toEqual({
      said: 'out',
      day: '2026-09-02T09:00:00Z',
      time: null,
    })
  })

  it('says it is booked for its day and hour, and late once that day has passed', () => {
    expect(
      publicationFact(going({ scheduled_at: '2026-09-22', scheduled_time: '18:00' }), TODAY),
    ).toEqual({ said: 'booked', day: '2026-09-22', time: '18:00' })
    expect(publicationFact(going({ scheduled_at: '2026-09-10' }), TODAY)).toEqual({
      said: 'late',
      day: '2026-09-10',
      time: null,
    })
    // Today is not late: the day has not passed yet.
    expect(publicationFact(going({ scheduled_at: TODAY }), TODAY).said).toBe('booked')
  })

  it('leaves the status word to say it when nothing is out or booked', () => {
    expect(publicationFact({ release: null }, TODAY)).toEqual({
      said: 'status',
      day: null,
      time: null,
    })
    expect(publicationFact(going({}), TODAY).said).toBe('status')
  })
})

describe('a kind as a word in a sentence', () => {
  it('lowers the first letter, and leaves an abbreviation alone', () => {
    expect(wordOf('Audio')).toBe('audio')
    expect(wordOf('YouTube Short')).toBe('YouTube Short')
    expect(wordOf('Short film')).toBe('short film')
    expect(wordOf('MV')).toBe('MV')
    expect(wordOf('Шортс', 'ru')).toBe('шортс')
    expect(wordOf('')).toBe('')
  })

  it('takes "an" before a vowel', () => {
    expect(takesAn('audio')).toBe(true)
    expect(takesAn('short')).toBe(false)
  })
})

describe('doorsOf', () => {
  const name = (label: unknown) => String(label)

  it('says each door with the shape its cover is drawn in', () => {
    const doors = [
      { label: 'YouTube', cover_format: '16:9' },
      { label: 'Streaming', cover_format: '1:1' },
    ]
    expect(doorsOf(doors, name)).toBe('YouTube · 16:9, Streaming · 1:1')
  })

  it('says a door with no shape by its name alone, and no door as nothing', () => {
    expect(doorsOf([{ label: 'Premiere', cover_format: null }], name)).toBe('Premiere')
    expect(doorsOf([], name)).toBe('')
  })
})
