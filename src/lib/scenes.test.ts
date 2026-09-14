import { describe, expect, it } from 'vitest'
import type { Scene, SceneBlock } from '@/lib/api'
import { readinessOf } from '@/lib/scenes'

const BLOCKS: SceneBlock[] = [
  { key: 'still', label: 'Still' },
  { key: 'motion', label: 'Motion' },
]

const scene = (description: string, blocks: Record<string, string>): Scene =>
  ({
    id: 's',
    profile_id: 'p',
    work_id: 'w',
    position: 1,
    section: null,
    starts_at: null,
    ends_at: null,
    shot_type: null,
    description,
    blocks,
    created_at: '',
    updated_at: '',
  }) as Scene

describe('readinessOf', () => {
  it('calls an untouched scene empty', () => {
    expect(readinessOf(scene('', {}), BLOCKS)).toBe('empty')
  })

  it('calls a scene with everything the profile asks for ready', () => {
    expect(readinessOf(scene('a lighthouse', { still: 'a', motion: 'b' }), BLOCKS)).toBe('ready')
  })

  it('calls a half-written scene started, from either half', () => {
    expect(readinessOf(scene('a lighthouse', {}), BLOCKS)).toBe('started')
    expect(readinessOf(scene('', { still: 'a' }), BLOCKS)).toBe('started')
    expect(readinessOf(scene('a lighthouse', { still: 'a' }), BLOCKS)).toBe('started')
  })

  it('does not call a scene ready on its prompts alone', () => {
    // Every block written and no description is the shape a proposal leaves
    // behind: the prompts are there, but nobody has said what the scene is.
    expect(readinessOf(scene('', { still: 'a', motion: 'b' }), BLOCKS)).toBe('started')
  })

  it('does not count whitespace as written', () => {
    expect(readinessOf(scene('   ', { still: '  \n ' }), BLOCKS)).toBe('empty')
  })

  it('asks only for a description when the kind names no blocks', () => {
    expect(readinessOf(scene('a lighthouse', {}), [])).toBe('ready')
    expect(readinessOf(scene('', {}), [])).toBe('empty')
  })

  it('ignores a block the profile no longer names', () => {
    expect(readinessOf(scene('a lighthouse', { still: 'a', motion: 'b', old: 'c' }), BLOCKS)).toBe(
      'ready',
    )
  })
})
