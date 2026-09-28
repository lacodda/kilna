import { describe, expect, it } from 'vitest'
import type {
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
  fieldsOf,
  leadOf,
  nextStage,
  previewOf,
  previousOf,
  releaseTone,
  styleRoleOf,
  textRoleOf,
  trailOf,
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
    scenes: false,
    findings: false,
  }

  it('draws only what the work always has when the kind names nothing', () => {
    const drawn = WIDGETS.filter((id) => applies(id, none))
    expect(drawn.sort()).toEqual(['cover', 'fields', 'links', 'recent', 'stage'])
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
      w('releases', 's'),
      w('recent', 's'),
    ])
    expect(lead).toEqual([
      { kind: 'one', widget: w('text', 'l') },
      { kind: 'pair', widgets: [w('style', 'm'), w('fields', 'm')] },
    ])
    expect(rail.map((x) => x.id)).toEqual(['score', 'stage', 'releases', 'recent'])
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
