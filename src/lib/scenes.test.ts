import { describe, expect, it } from 'vitest'
import type { Scene, SceneBlock, SceneFrame } from '@/lib/api'
import {
  chosenFrame,
  chosenVideo,
  framesByScene,
  ofKind,
  readinessOf,
  FRAME,
  VIDEO,
} from '@/lib/scenes'

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

const frame = (
  id: string,
  sceneId: string,
  isSelected = false,
  kind: string = FRAME,
): SceneFrame => ({
  id,
  scene_id: sceneId,
  asset_id: `a-${id}`,
  kind,
  position: 1,
  is_selected: isSelected,
  path: `/media/${id}.${kind === VIDEO ? 'mp4' : 'png'}`,
  original_name: `${id}.${kind === VIDEO ? 'mp4' : 'png'}`,
  created_at: '',
})

describe('readinessOf, with frames', () => {
  const full = scene('a lighthouse', { still: 'a', motion: 'b' })

  it('calls a filled-in scene with a chosen frame shot', () => {
    expect(readinessOf(full, BLOCKS, [frame('f1', 's', true)])).toBe('shot')
  })

  it('leaves a filled-in scene ready while the frames are only candidates', () => {
    // Four pictures and no verdict is the middle of the work: the scene is
    // still ready to draw from, and saying otherwise would be a step back.
    expect(readinessOf(full, BLOCKS, [frame('f1', 's'), frame('f2', 's')])).toBe('ready')
  })

  it('does not let a chosen frame stand in for the writing', () => {
    // A picture hung on a scene nobody described does not make it ready; the
    // step is past `ready`, not a way around it.
    expect(readinessOf(scene('', {}), BLOCKS, [frame('f1', 's', true)])).toBe('empty')
    expect(readinessOf(scene('a lighthouse', {}), BLOCKS, [frame('f1', 's', true)])).toBe('started')
  })
})

describe('framesByScene', () => {
  it('groups the frames of a board by the scene they were drawn for', () => {
    const grouped = framesByScene([frame('f1', 's1'), frame('f2', 's2'), frame('f3', 's1')])
    expect(grouped.get('s1')?.map((one) => one.id)).toEqual(['f1', 'f3'])
    expect(grouped.get('s2')?.map((one) => one.id)).toEqual(['f2'])
    expect(grouped.get('s3')).toBeUndefined()
  })
})

describe('chosenFrame', () => {
  it('finds the frame the video is cut from, and nothing when there is none', () => {
    expect(chosenFrame([frame('f1', 's'), frame('f2', 's', true)])?.id).toBe('f2')
    expect(chosenFrame([frame('f1', 's')])).toBeUndefined()
    expect(chosenFrame(undefined)).toBeUndefined()
  })
})

describe('a scene that holds clips as well as stills', () => {
  const full = scene('a lighthouse', { still: 'a', motion: 'b' })

  it('is shot by its chosen STILL, never by a chosen clip', () => {
    // The trap the kind exists to close: a clip chosen while the picture
    // never was is not a scene that has been shot, and calling it one would
    // let a board report itself finished with nothing drawn.
    const onlyClip = [frame('v1', 's1', true, VIDEO)]
    expect(readinessOf(full, BLOCKS, onlyClip)).toBe('ready')

    const both = [frame('f1', 's1', true), frame('v1', 's1', true, VIDEO)]
    expect(readinessOf(full, BLOCKS, both)).toBe('shot')
  })

  it('tells the chosen still from the chosen clip', () => {
    const material = [
      frame('f1', 's1'),
      frame('f2', 's1', true),
      frame('v1', 's1', true, VIDEO),
    ]
    expect(chosenFrame(material)?.id).toBe('f2')
    expect(chosenVideo(material)?.id).toBe('v1')
  })

  it('splits material by kind, keeping the order of each list', () => {
    const material = [
      frame('f1', 's1'),
      frame('v1', 's1', false, VIDEO),
      frame('f2', 's1'),
    ]
    expect(ofKind(material, FRAME).map((one) => one.id)).toEqual(['f1', 'f2'])
    expect(ofKind(material, VIDEO).map((one) => one.id)).toEqual(['v1'])
  })
})
