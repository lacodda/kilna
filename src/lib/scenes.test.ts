import { describe, expect, it } from 'vitest'
import type { Meta, Scene, SceneBlock, SceneFrame, Work } from '@/lib/api'
import {
  chosenFrame,
  chosenVideo,
  durationOf,
  framesByScene,
  ofKind,
  orderMoving,
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

describe('the order that moves a scene', () => {
  const board = (count: number): Scene[] =>
    Array.from({ length: count }, (_, index) => ({
      ...scene('', {}),
      id: `s${index + 1}`,
      position: index + 1,
    }))

  it('puts a scene at the number asked for, the rest closing up behind it', () => {
    // The last scene of four said to happen second: it lands second, and the
    // two it displaced move down rather than sharing its number.
    expect(orderMoving(board(4), 's4', 2)).toEqual(['s1', 's4', 's2', 's3'])
    // And back the other way, which is the same call.
    expect(orderMoving(board(4), 's1', 4)).toEqual(['s2', 's3', 's4', 's1'])
  })

  it('reads a number past either end as that end', () => {
    // A person who asks for a number off the board means the edge of it.
    // Refusing would be pedantry about arithmetic they did not care about.
    expect(orderMoving(board(3), 's2', 99)).toEqual(['s1', 's3', 's2'])
    expect(orderMoving(board(3), 's2', 0)).toEqual(['s2', 's1', 's3'])
    expect(orderMoving(board(3), 's2', -7)).toEqual(['s2', 's1', 's3'])
  })

  it('always names every scene exactly once — the list renumbering accepts', () => {
    // The backend refuses an order that forgets a scene or names one twice,
    // and it is right to. This is what keeps that refusal from ever being
    // the person's problem.
    const all = board(5)
    for (const to of [-1, 1, 3, 5, 9]) {
      for (const scene of all) {
        const moved = orderMoving(all, scene.id, to)
        expect([...moved].sort()).toEqual(all.map((one) => one.id).sort())
      }
    }
  })

  it('leaves a board alone when the scene is already where it is asked for', () => {
    expect(orderMoving(board(3), 's2', 2)).toEqual(['s1', 's2', 's3'])
  })
})

describe("a work's length", () => {
  const work = (duration: unknown): Work =>
    ({ meta: { duration } as unknown as Meta }) as Work

  it('reads a number of seconds', () => {
    expect(durationOf(work(225))).toBe(225)
    expect(durationOf(work(3.5))).toBe(3.5)
  })

  it('still reads a timecode, for a workspace written before the retype', () => {
    // The field shipped as text holding "3:45" and migration 0018 converted
    // it, but a person can type a timecode back into it — and it means the
    // same length. Read the way `scene::duration_of` reads it.
    expect(durationOf(work('3:45'))).toBe(225)
    expect(durationOf(work('225'))).toBe(225)
    expect(durationOf(work('1:00:00'))).toBe(3600)
  })

  it('calls anything unreadable NO length rather than a zero', () => {
    // A zero would leave the board judged against a length that is not
    // there; nothing leaves it unjudged, which is the honest answer.
    // `'0'` and `'0:00'` take the STRING path to a length of zero, which
    // the number path never reaches: a board judged against a length of
    // zero would report every scene as overrunning the work.
    for (const value of [null, undefined, '', '   ', 'soon', '3:', '0', '0:00', -5, 0, NaN]) {
      expect(durationOf(work(value))).toBeNull()
    }
  })
})
