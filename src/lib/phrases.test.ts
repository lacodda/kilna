import { describe, expect, it } from 'vitest'
import type { ProfileConfig, StyleBrick, StyleType } from '@/lib/api/types'
import { brickMatches, halvesOf, insertPhrase, offeredBricks, PICTURE } from '@/lib/phrases'

const type = (key: string, form: StyleType['form'] = 'picture'): StyleType =>
  ({ key, label: key, form }) as StyleType

const config = {
  compose: [
    {
      key: 'sound',
      label: { en: 'Sound', ru: 'Звук' },
      role: 'style',
      kinds: ['song'],
      parts: [
        { type: 'groove', min: 1, max: 1 },
        { type: 'genre', min: 1 },
      ],
    },
  ],
} as unknown as ProfileConfig

describe('the halves of the dictionary', () => {
  it('put a composition’s types in its own half, in the order they are picked', () => {
    const types = [type('image-style'), type('genre', 'phrase'), type('groove', 'phrase')]
    const halves = halvesOf(config, types)
    expect(halves.map((half) => half.key)).toEqual([PICTURE, 'sound'])
    expect(halves[0]?.types.map((one) => one.key)).toEqual(['image-style'])
    expect(halves[1]?.types.map((one) => one.key)).toEqual(['groove', 'genre'])
  })

  it('leave out a half with nothing in it', () => {
    expect(halvesOf({ compose: [] } as unknown as ProfileConfig, [type('look')])).toHaveLength(1)
  })
})

describe('a phrase put in at the caret', () => {
  it('is a tag of the line: a separator before and after it where the line goes on', () => {
    expect(insertPhrase('dream pop, warm pads', 9, 'breakbeat')).toEqual({
      text: 'dream pop, breakbeat, warm pads',
      caret: 20,
    })
  })

  it('needs no separator at the start or right after one', () => {
    expect(insertPhrase('', 0, 'breakbeat').text).toBe('breakbeat')
    expect(insertPhrase('dream pop, ', 11, 'breakbeat').text).toBe('dream pop, breakbeat')
    expect(insertPhrase('dream pop', 9, 'breakbeat').text).toBe('dream pop, breakbeat')
  })
})

const brick = (id: string, phrase: string, change: Partial<StyleBrick> = {}): StyleBrick =>
  ({
    id,
    type_key: 'groove',
    name: phrase,
    description: phrase,
    status: 'ready',
    explanation: null,
    ...change,
  }) as StyleBrick

describe('the bricks offered', () => {
  it('are ready ones of the type, the house ones first', () => {
    const offered = offeredBricks(
      [
        brick('a', 'breakbeat'),
        brick('b', 'punchy drums'),
        brick('c', 'brushed drums', { status: 'draft' }),
        brick('d', 'dream pop', { type_key: 'genre' }),
      ],
      'groove',
      new Set(['b']),
    )
    expect(offered.map((one) => one.id)).toEqual(['b', 'a'])
  })

  it('are found by what they mean, in any language', () => {
    const one = brick('a', 'breakbeat', {
      explanation: { en: 'Chopped drums.', ru: 'Рубленые барабаны.' },
    })
    expect(brickMatches(one, 'рублен')).toBe(true)
    expect(brickMatches(one, 'BREAK')).toBe(true)
    expect(brickMatches(one, 'piano')).toBe(false)
  })
})
