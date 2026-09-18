import { describe, expect, it } from 'vitest'
import { stem } from '@/lib/stem'

// The stemmer is judged on one thing: forms of one word meet, and different
// words do not. The stems themselves are keys, not words, so the tests say
// "these agree" rather than "this is the stem".
const agree = (...words: string[]) => {
  const stems = new Set(words.map(stem))
  expect(stems, words.join(' / ')).toHaveProperty('size', 1)
}
const differ = (a: string, b: string) => {
  expect(stem(a), `${a} vs ${b}`).not.toBe(stem(b))
}

describe('Russian', () => {
  it('meets the cases of a noun', () => {
    agree('лестница', 'лестницы', 'лестнице', 'лестницу', 'лестницей', 'лестниц')
    agree('окно', 'окна', 'окну', 'окном', 'окне')
    agree('дорога', 'дороги', 'дорогу', 'дорогой', 'дорогами')
  })

  it('meets the forms of a verb', () => {
    agree('читает', 'читают', 'читаем', 'читала', 'читать')
    agree('бежала', 'бежали', 'бежал', 'бежать')
    agree('строит', 'строят', 'строил', 'строить')
  })

  it('meets the forms of an adjective', () => {
    agree('тёплый', 'тёплая', 'тёплое', 'тёплые', 'тёплого', 'тёплыми')
  })

  it('does not merge different words', () => {
    differ('лестница', 'лес')
    differ('дорога', 'дорого')
    differ('окно', 'око')
  })

  it('treats ё as е', () => {
    agree('ещё', 'еще')
  })
})

describe('English', () => {
  it('meets plurals and verb forms', () => {
    agree('ladder', 'ladders')
    agree('love', 'loves', 'loved', 'loving')
    agree('hope', 'hopes', 'hoped', 'hoping')
    agree('city', 'cities')
  })

  it('keeps short words whole', () => {
    expect(stem('sing')).toBe('sing')
    expect(stem('is')).toBe('is')
  })

  it('does not merge different words', () => {
    differ('sing', 'sin')
    differ('night', 'nine')
  })
})

it('lowercases and leaves other scripts whole', () => {
  expect(stem('Лестница')).toBe(stem('лестница'))
  expect(stem('東京')).toBe('東京')
})
