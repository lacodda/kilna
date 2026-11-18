import { describe, expect, it } from 'vitest'
import type { CardView } from '@/lib/api/types'
import { isDetailed } from './disclosure'

// v0.141, FR-18 18.2: a card shows its layers and states only where they say
// something, or while a task's reading is looked at.
describe('the detail an open card shows', () => {
  const view = (
    facts: { layer: string; status: string }[],
    relations: { layer: string }[] = [],
  ): CardView =>
    ({
      facts: facts.map((fact) => ({ fact, lenses: [] })),
      relations: relations.map((link) => ({ link })),
    }) as unknown as CardView

  it('keeps a card of public, settled facts plain', () => {
    expect(isDetailed(view([{ layer: 'public', status: 'canon' }]), 'all')).toBe(false)
    expect(isDetailed(view([]), 'all')).toBe(false)
  })

  it('shows them for anything not public, a draft, a live zone, or a lens', () => {
    expect(isDetailed(view([{ layer: 'internal', status: 'canon' }]), 'all')).toBe(true)
    expect(isDetailed(view([{ layer: 'public', status: 'draft' }]), 'all')).toBe(true)
    expect(isDetailed(view([{ layer: 'public', status: 'open' }]), 'all')).toBe(true)
    expect(isDetailed(view([], [{ layer: 'inWorks' }]), 'all')).toBe(true)
    expect(isDetailed(view([{ layer: 'public', status: 'canon' }]), 'cover')).toBe(true)
  })
})

// Every shape a section holds facts in has the words its "+" button says:
// the button's key is made at run time, out of sight of the locale check -
// the bans of the passport said "canon.add.bans" until v0.141.
describe('the way to add to a section', () => {
  it('is worded for every shape that holds facts', async () => {
    const en = (await import('@/i18n/locales/en.json')).default
    const ru = (await import('@/i18n/locales/ru.json')).default
    for (const shape of ['facts', 'slots', 'details', 'palette', 'marks', 'styles', 'bans']) {
      expect([shape, shape in en.canon.add, shape in ru.canon.add]).toEqual([shape, true, true])
    }
  })
})
