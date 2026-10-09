import { describe, expect, it } from 'vitest'
import type { StyleBrick } from '@/lib/api/types'
import {
  adopt,
  explanationIn,
  formOf,
  nameProblem,
  patchOf,
  untitledName,
  withExplanation,
  type StyleForm,
} from '@/lib/styleDraft'

const BRICK: StyleBrick = {
  trial_id: null,
  id: 'b1',
  profile_id: 'p1',
  type_key: 'look',
  name: 'Dusk',
  description: null,
  hint: 'only the water',
  status: 'draft',
  created_at: '2026-09-01T00:00:00.000Z',
  updated_at: '2026-09-01T00:00:00.000Z',
  reference_count: 2,
  label: null,
  family: null,
  when_to_use: null,
  explanation: null,
  colours: [],
  sample: null,
  set_key: null,
  origin: 'own',
}

const stored = formOf(BRICK)
const typed = (change: Partial<StyleForm>): StyleForm => ({ ...stored, ...change })

describe('formOf', () => {
  it('spells a missing text as an empty box', () => {
    expect(formOf(BRICK)).toEqual({
      type_key: 'look',
      name: 'Dusk',
      description: '',
      hint: 'only the water',
      status: 'draft',
      when_to_use: '',
      family: '',
      sample: '',
      colours: [],
      explanation: '',
      explanationStored: null,
    })
  })

  it('names a set brick by its word in the window language', () => {
    const shipped = formOf({ ...BRICK, name: 'Paper', label: { en: 'Paper', ru: 'Бумага' } })
    expect(shipped.name).toBe('Paper')
  })
})

describe('the fields of v0.87', () => {
  it('writes when to take it, the family and the sample in the stored spelling', () => {
    expect(
      patchOf(typed({ when_to_use: ' Loud songs ', family: 'tattoo', sample: '' }), stored),
    ).toEqual({ when_to_use: 'Loud songs', family: 'tattoo' })
    expect(patchOf(typed({ family: '' }), { ...stored, family: 'tattoo' })).toEqual({
      family: null,
    })
  })

  it('holds a colour back until it is one, and writes it upper case', () => {
    expect(patchOf(typed({ colours: ['#1e9'] }), stored)).toBeNull()
    expect(patchOf(typed({ colours: ['#1e9e95'] }), stored)).toEqual({ colours: ['#1E9E95'] })
    expect(patchOf(typed({ colours: ['#1E9E95'] }), { ...stored, colours: ['#1e9e95'] })).toBeNull()
    expect(patchOf(typed({ colours: [''] }), { ...stored, colours: ['#1E9E95'] })).toEqual({
      colours: [],
    })
  })

  // v0.90.3: a ground or an accent may be a gradient, and a palette is
  // edited at last - one box per colour.
  it('writes the stops of a gradient in their order, and none while one is half typed', () => {
    const ground = { ...stored, colours: ['#FA8072'] }
    expect(patchOf({ ...ground, colours: ['#FA8072', '#f4a261'] }, ground)).toEqual({
      colours: ['#FA8072', '#F4A261'],
    })
    expect(patchOf({ ...ground, colours: ['#FA8072', '#f4a'] }, ground)).toBeNull()
    expect(patchOf({ ...ground, colours: ['#FA8072', ''] }, ground)).toBeNull()
    expect(
      patchOf(
        { ...ground, colours: ['#F4A261', '#FA8072'] },
        { ...ground, colours: ['#FA8072', '#F4A261'] },
      ),
    ).toEqual({
      colours: ['#F4A261', '#FA8072'],
    })
  })
})

describe('patchOf', () => {
  it('writes nothing when the boxes say what is stored', () => {
    expect(patchOf(stored, stored)).toBeNull()
  })

  it('writes only what changed, in the stored spelling', () => {
    expect(patchOf(typed({ description: '  Low sun \n', status: 'ready' }), stored)).toEqual({
      description: 'Low sun',
      status: 'ready',
    })
    expect(patchOf(typed({ name: ' Dusk over water ' }), stored)).toEqual({
      name: 'Dusk over water',
    })
  })

  it('clears a text emptied down to blanks, rather than storing the blanks', () => {
    expect(patchOf(typed({ hint: '   ' }), stored)).toEqual({ hint: null })
  })

  it('takes a trailing space for no change: it is on its way to a word', () => {
    expect(patchOf(typed({ name: 'Dusk ' }), stored)).toBeNull()
  })

  it('holds back the name and the type it was told to, and still writes the rest', () => {
    const form = typed({ name: '', description: 'Low sun' })
    expect(patchOf(form, stored, { keyHeld: true })).toEqual({ description: 'Low sun' })
    // The type is half of the key a name is unique under: written alone it
    // could collide as surely as the name.
    expect(
      patchOf(typed({ name: 'Taken', type_key: 'place' }), stored, { keyHeld: true }),
    ).toBeNull()
  })
})

describe('adopt', () => {
  it('shows a value written elsewhere in a field nobody is typing into', () => {
    // The assistant's description was kept, and the brick left its draft.
    const arrived = typed({ description: 'Low sun, long reflections.', status: 'ready' })
    const { form, base } = adopt(stored, stored, arrived)
    expect(form).toEqual(arrived)
    expect(base).toEqual(arrived)
  })

  it('leaves a field being typed into alone, and measures the next write against what arrived', () => {
    const mine = typed({ description: 'Teal and amber' })
    const arrived = typed({ description: 'Low sun, long reflections.', status: 'ready' })
    const { form, base } = adopt(mine, stored, arrived)
    expect(form.description).toBe('Teal and amber')
    // The status nobody touched follows.
    expect(form.status).toBe('ready')
    expect(patchOf(form, base)).toEqual({ description: 'Teal and amber' })
  })

  it('keeps the box as typed when what came back means the same', () => {
    // "Low sun " was written as "Low sun"; the space stays under the caret.
    const box = typed({ description: 'Low sun ' })
    const { form, base } = adopt(box, box, typed({ description: 'Low sun' }))
    expect(form.description).toBe('Low sun ')
    expect(patchOf(form, base)).toBeNull()
  })

  it('takes back an undone name', () => {
    const renamed = typed({ name: 'Dusk over water' })
    const { form } = adopt(renamed, renamed, stored)
    expect(form.name).toBe('Dusk')
  })
})

describe('nameProblem', () => {
  it('asks for a name, and for one no other style of the type has', () => {
    expect(nameProblem('  ', [])).toBe('missing')
    expect(nameProblem(' Dusk ', ['Dusk', 'Dawn'])).toBe('taken')
    expect(nameProblem('dusk', ['Dusk'])).toBeNull()
    expect(nameProblem('Noon', ['Dusk'])).toBeNull()
  })
})

describe('untitledName', () => {
  it('numbers the untitled so two of one type never share a name', () => {
    expect(untitledName('Untitled style', ['Dusk'])).toBe('Untitled style')
    expect(untitledName('Untitled style', ['Untitled style'])).toBe('Untitled style 2')
    expect(untitledName('Untitled style', ['Untitled style', 'Untitled style 2'])).toBe(
      'Untitled style 3',
    )
  })
})

describe('the explanation', () => {
  it('is edited in one language and keeps the others a shipped brick carries', () => {
    const stored = { en: 'Grit between the phrases.', ru: 'Грязь между фразами.' }
    const shipped = { ...BRICK, explanation: stored }
    expect(explanationIn(stored, 'ru')).toBe('Грязь между фразами.')
    const base = formOf(shipped)
    const patch = patchOf({ ...base, explanation: 'Короткие всплески шума.' }, base)
    // The window speaks English in the tests: the English sentence is the
    // one rewritten, the Russian kept.
    expect(patch).toEqual({
      explanation: { en: 'Короткие всплески шума.', ru: 'Грязь между фразами.' },
    })
  })

  it('is never seeded with another language, and a blank one goes', () => {
    expect(explanationIn({ en: 'Grit.' }, 'ru')).toBe('')
    expect(withExplanation({ en: 'Grit.', ru: 'Грязь.' }, '  ', 'ru')).toEqual({ en: 'Grit.' })
    expect(withExplanation({ ru: 'Грязь.' }, '', 'ru')).toBeNull()
  })

  it('is one string when a person writes it for a brick of their own', () => {
    expect(withExplanation(null, ' Grit. ', 'ru')).toBe('Grit.')
    expect(withExplanation('Old words.', 'New words.', 'en')).toBe('New words.')
  })
})
