import { afterEach, describe, expect, it } from 'vitest'
import i18n from '@/i18n'
import type { MetaField, ProfileConfig } from '@/lib/api/types'
import { fieldText } from '@/lib/fieldValue'
import { fieldsFor } from '@/lib/useProfile'
import music from '@/test/fixtures/profiles/music.json'

/*
 * A field's value as it is read, against the shipped Studio profile: the
 * audio release's variant is a choice stored by key and read by its label.
 */
const studio = music.config as unknown as ProfileConfig
const field = (key: string): MetaField => studio.work_meta_fields.find((f) => f.key === key)!

afterEach(async () => {
  await i18n.changeLanguage('en')
})

describe('a field as it is read', () => {
  it("reads a choice by its answer's label, in the window's language", async () => {
    expect(fieldText(field('variant'), 'sped-up')).toBe('Sped up')
    await i18n.changeLanguage('ru')
    expect(fieldText(field('variant'), 'instrumental')).toBe('Инструментал')
  })

  it('shows a key the field no longer offers as it is, rather than nothing', () => {
    expect(fieldText(field('variant'), 'acoustic')).toBe('acoustic')
  })

  it('reads nothing as nothing, and every other value as itself', () => {
    expect(fieldText(field('variant'), undefined)).toBeNull()
    expect(fieldText(field('variant'), null)).toBeNull()
    expect(fieldText(field('variant'), '')).toBeNull()
    expect(fieldText(field('bpm'), 96)).toBe('96')
    expect(fieldText(field('key'), 'A minor')).toBe('A minor')
  })

  it("belongs to the kinds it names: the audio's variant is not a song's", () => {
    const keysOf = (kind: string) => fieldsFor(studio, kind).map((f) => f.key)
    expect(keysOf('audio')).toContain('variant')
    expect(keysOf('song')).not.toContain('variant')
    // A field naming no kinds is every kind's.
    expect(keysOf('song')).toContain('bpm')
    expect(keysOf('audio')).toContain('bpm')
  })
})
