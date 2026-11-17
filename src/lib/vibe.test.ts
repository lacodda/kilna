import { describe, expect, it } from 'vitest'
import type { VibeAxes } from '@/lib/api/types'
import { axesOf, resolveVibe, vibeWords, withLine } from '@/lib/vibe'
import { studio } from '@/test/workspace'

/*
 * What a work feels like (FR-02): one, two or three lines said, the rest
 * standing at the neutral word while the mark is drawn, and no mark at all
 * when nothing is said.
 */

const axes = axesOf(studio().profile.config)!

describe("a work's vibe", () => {
  it('is said with the words of the profile: five moods, five characters, three weights', () => {
    expect([axes.moods.length, axes.characters.length, axes.complexities.length]).toEqual([5, 5, 3])
    expect(axes.complexities.map((one) => one.strokes)).toEqual([1, 2, 3])
    expect(new Set(axes.moods.map((one) => one.color)).size).toBe(5)
    expect(new Set(axes.characters.map((one) => one.glyph)).size).toBe(5)
  })

  it('has no mark while nothing is said', () => {
    expect(resolveVibe(axes, undefined)).toBeNull()
    expect(resolveVibe(axes, null)).toBeNull()
    expect(resolveVibe(axes, {})).toBeNull()
    // A profile with no lines draws nothing, whatever a work says.
    expect(resolveVibe(undefined, { mood: 'dark' })).toBeNull()
  })

  it('stands an unsaid line at the middle of it while another is said', () => {
    const one = resolveVibe(axes, { mood: 'melancholy' })!
    expect([one.mood.key, one.character.key, one.complexity.key]).toEqual([
      'melancholy',
      'steady',
      'medium',
    ])
    expect(one.said).toEqual(['mood'])
    // Only what was said is said aloud: the neutral words are how the mark
    // is drawn, not what the person chose.
    expect(vibeWords(one, 'en')).toBe('Melancholy')

    const all = resolveVibe(axes, { mood: 'dark', character: 'edgy', complexity: 'complex' })!
    expect(vibeWords(all, 'en')).toBe('Dark · Raw · Complex')
    expect(vibeWords(all, 'ru')).toBe('Мрачная · Надрывная · Сложная')
    expect(all.complexity.strokes).toBe(3)
  })

  it('stands an unsaid line where the profile names its neutral', () => {
    const named: VibeAxes = { ...axes, neutral: { character: 'calm' } }
    const one = resolveVibe(named, { mood: 'warm' })!
    expect([one.character.key, one.complexity.key]).toEqual(['calm', 'medium'])
  })

  it('reads a word the profile no longer has as unsaid', () => {
    expect(resolveVibe(axes, { mood: 'sunny' })).toBeNull()
    const kept = resolveVibe(axes, { mood: 'sunny', character: 'drive' })!
    expect(kept.said).toEqual(['character'])
    expect(kept.mood.key).toBe('contemplative')
  })

  it('is changed a line at a time, and gone with its last word', () => {
    expect(withLine(undefined, 'mood', 'dark')).toEqual({ mood: 'dark' })
    expect(withLine({ mood: 'dark' }, 'character', 'calm')).toEqual({
      mood: 'dark',
      character: 'calm',
    })
    expect(withLine({ mood: 'dark', character: 'calm' }, 'mood', undefined)).toEqual({
      character: 'calm',
    })
    expect(withLine({ character: 'calm' }, 'character', undefined)).toBeNull()
  })
})
