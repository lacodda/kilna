import { describe, expect, it } from 'vitest'
import i18n from '@/i18n'
import { describeSkipped } from '@/lib/batch'

const skip = (title: string | null, key: string) => ({
  id: title ?? 'x',
  title,
  reason: { key, params: {} },
})

describe('describeSkipped', () => {
  it('says nothing when a batch reached everything', () => {
    expect(describeSkipped([])).toBeNull()
  })

  it('names each item passed over, with why', () => {
    const said = describeSkipped([
      skip('Winter road', 'skip.alreadyThere'),
      skip(null, 'error.notFound'),
    ])!
    expect(said.title).toBe(i18n.t('batch.skipped', { count: 2 }))
    expect(said.description).toContain('Winter road')
    expect(said.description).toContain(i18n.t('skip.alreadyThere'))
    expect(said.description).toContain(i18n.t('error.notFound'))
    expect(said.description).toContain(i18n.t('batch.untitled'))
  })

  it('counts the rest past the first few', () => {
    const many = Array.from({ length: 8 }, (_, i) => skip(`Song ${i}`, 'skip.alreadyThere'))
    const said = describeSkipped(many)!
    expect(said.description.split('\n')).toHaveLength(6)
    expect(said.description).toContain(i18n.t('batch.andMore', { count: 3 }))
    expect(said.description).not.toContain('Song 7')
  })
})
