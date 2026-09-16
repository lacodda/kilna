import { describe, expect, it } from 'vitest'
import type { Scene, SceneFrame } from '@/lib/api'
import { montageFileName, montageList } from '@/lib/montage'
import { framesByScene, FRAME, VIDEO } from '@/lib/scenes'

const scene = (position: number, startsAt: number | null, endsAt: number | null): Scene => ({
  id: `s${position}`,
  profile_id: 'p',
  work_id: 'w',
  position,
  section: null,
  starts_at: startsAt,
  ends_at: endsAt,
  shot_type: null,
  description: '',
  blocks: {},
  created_at: '',
  updated_at: '',
})

const material = (
  id: string,
  sceneId: string,
  kind: string,
  isSelected: boolean,
): SceneFrame => ({
  id,
  scene_id: sceneId,
  asset_id: `a-${id}`,
  kind,
  position: 1,
  is_selected: isSelected,
  path: `/media/${id}.${kind === VIDEO ? 'mp4' : 'png'}`,
  original_name: null,
  created_at: '',
})

describe('the list a board is cut from', () => {
  it('writes a scene as its number, its span and the two files it uses', () => {
    const scenes = [scene(1, 0, 4)]
    const frames = framesByScene([
      material('f1', 's1', FRAME, false),
      material('f2', 's1', FRAME, true),
      material('v1', 's1', VIDEO, true),
    ])

    const [first = '', second = ''] = montageList(scenes, frames).split('\n')

    expect(first).toBe('01  0:00–0:04  /media/f2.png')
    // The clip sits under the still, indented to where the paths begin, so
    // the eye reads a block per scene rather than a column of look-alikes.
    expect(second).toBe(' '.repeat(first.indexOf('/media')) + '/media/v1.mp4')
  })

  it('names only what was CHOSEN, not every candidate', () => {
    // The point of the list: four pictures were drawn and one of them is the
    // cut. A list naming all four would be a list nobody could hand to an
    // editor.
    const frames = framesByScene([
      material('f1', 's1', FRAME, false),
      material('f2', 's1', FRAME, true),
      material('f3', 's1', FRAME, false),
    ])
    const text = montageList([scene(1, 0, 4)], frames)

    expect(text).toContain('/media/f2.png')
    expect(text).not.toContain('/media/f1.png')
    expect(text).not.toContain('/media/f3.png')
  })

  it('still gives a scene its line when nothing has been chosen yet', () => {
    // Silence would read as a gap that is not there; the list is also how a
    // person finds out what is still missing.
    const text = montageList([scene(3, null, null)], new Map())

    const [first = '', second = ''] = text.split('\n')

    expect(first).toBe('03  —  —')
    expect(second.trim()).toBe('—')
    // Aligned under the still it is missing beside.
    expect(second.indexOf('—')).toBe(first.lastIndexOf('—'))
  })

  it('keeps the order of the board rather than sorting again', () => {
    const scenes = [scene(2, 4, 9), scene(1, 0, 4)]
    const text = montageList(scenes, new Map())

    expect(text.indexOf('02')).toBeLessThan(text.indexOf('01'))
  })
})

describe('what the saved file is called', () => {
  it('is named for the work', () => {
    expect(montageFileName('Nadezhda')).toBe('Nadezhda — montage.txt')
  })

  it('drops the characters a file name may not carry', () => {
    expect(montageFileName('A/B: "C"?')).toBe('AB C — montage.txt')
  })

  it('falls back rather than producing a nameless file', () => {
    expect(montageFileName('  ')).toBe('montage — montage.txt')
  })
})
