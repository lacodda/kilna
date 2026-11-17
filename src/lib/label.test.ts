import { describe, expect, it } from 'vitest'
import i18n from '@/i18n'
import { isLabelMap, relabel, relabelOptional, resolveLabel } from '@/lib/label'

describe('resolveLabel', () => {
  it('says a map in the language asked, then English, then whatever it holds', () => {
    expect(resolveLabel({ en: 'Hook', ru: 'Крючок' }, 'ru')).toBe('Крючок')
    expect(resolveLabel({ en: 'Hook', ru: 'Крючок' }, 'ru-RU')).toBe('Крючок')
    expect(resolveLabel({ en: 'Hook', ru: 'Крючок' }, 'de')).toBe('Hook')
    expect(resolveLabel({ ru: 'Крючок' }, 'de')).toBe('Крючок')
  })

  it('keeps a typed word exactly as typed, and says nothing for nothing', () => {
    expect(resolveLabel('Хук', 'en')).toBe('Хук')
    expect(resolveLabel(undefined, 'en')).toBe('')
    expect(resolveLabel(null, 'en')).toBe('')
  })

  it('tells a language map from other objects', () => {
    expect(isLabelMap({ en: 'a', ru: 'б' })).toBe(true)
    expect(isLabelMap('a')).toBe(false)
    expect(isLabelMap({})).toBe(false)
    expect(isLabelMap({ en: 1 })).toBe(false)
    expect(isLabelMap(['a'])).toBe(false)
  })
})

// The class this guards: a profile word handed to `t()` as a value, where a
// map used to come out as "[object Object]". Every call site gets the fix
// from the i18n instance, so the instance is what is tested.
describe('i18n interpolation', () => {
  it('resolves a label map handed to t() in the language of the sentence', async () => {
    const english = i18n.getFixedT('en')
    const russian = i18n.getFixedT('ru')
    const tier = { en: 'Strong', ru: 'Сильная' }
    expect(english('editor.producesVersion', { role: tier })).toContain('Strong')
    expect(english('editor.producesVersion', { role: tier })).not.toContain('[object Object]')
    expect(russian('editor.producesVersion', { role: tier })).toContain('Сильная')
  })

  it('leaves ordinary values alone', () => {
    const english = i18n.getFixedT('en')
    expect(english('editor.producesVersion', { role: 'Lyrics' })).toContain('Lyrics')
  })
})

describe('relabel', () => {
  it('rewrites a word in the window language and keeps the others', () => {
    expect(relabel({ en: 'Song', ru: 'Песня' }, 'Трек', 'ru')).toEqual({ en: 'Song', ru: 'Трек' })
    expect(relabel({ en: 'Song', ru: 'Песня' }, 'Трек', 'ru-RU')).toEqual({
      en: 'Song',
      ru: 'Трек',
    })
    // A language the word did not have yet is added, not written over English.
    expect(relabel({ en: 'Song' }, 'Песня', 'ru')).toEqual({ en: 'Song', ru: 'Песня' })
  })

  it('keeps a typed word one string', () => {
    expect(relabel('Track', 'Трек', 'ru')).toBe('Трек')
    expect(relabel(undefined, 'Трек', 'ru')).toBe('Трек')
  })

  it('lets an optional word go in one language, and altogether with its last', () => {
    expect(relabelOptional({ en: 'A hint', ru: 'Подсказка' }, '', 'ru')).toEqual({ en: 'A hint' })
    expect(relabelOptional({ ru: 'Подсказка' }, '', 'ru')).toBeUndefined()
    expect(relabelOptional('A hint', '', 'en')).toBeUndefined()
    expect(relabelOptional(null, 'A hint', 'en')).toBe('A hint')
  })
})
