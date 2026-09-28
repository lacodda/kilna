import { describe, expect, it } from 'vitest'
import type { StyleBrick } from '@/lib/api/types'
import { adopt, formOf, nameProblem, patchOf, untitledName, type StyleForm } from '@/lib/styleDraft'

const BRICK: StyleBrick = {
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
