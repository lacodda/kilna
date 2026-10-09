import { describe, expect, it } from 'vitest'
import type { Asset, WorkFolder } from '@/lib/api/types'
import { clock } from '@/components/AudioPlayer'
import { soundsOf } from '@/features/work/player'

const ASSET: Asset = {
  trial_id: null,
  id: 'a1',
  profile_id: 'p',
  work_id: 'w',
  release_id: null,
  kind: 'attachment',
  path: 'C:/ws/media/a1.mp3',
  label: null,
  original_name: 'demo.mp3',
  style_brick_id: null,
  note_id: null,
  canon_fact_id: null,
  created_at: '2026-09-15T10:00:00Z',
}

const FOLDER: WorkFolder = {
  state: 'found',
  files: [
    {
      path: 'D:/s/mix/v1/final.wav',
      relative: 'mix/v1/final.wav',
      size: 1,
      modified: '2026-10-01T10:00:00Z',
    },
    { path: 'D:/s/still.png', relative: 'still.png', size: 1, modified: '2026-10-02T10:00:00Z' },
    { path: 'D:/s/take.flac', relative: 'take.flac', size: 1 },
  ],
  truncated: false,
}

describe("the card's sounds", () => {
  it('are the sounds attached and in the folder, newest first, each saying where it lies', () => {
    expect(soundsOf([ASSET], FOLDER, 'attached')).toEqual([
      { path: 'D:/s/mix/v1/final.wav', name: 'final.wav', where: 'mix/v1' },
      { path: 'C:/ws/media/a1.mp3', name: 'demo.mp3', where: 'attached' },
      // A file whose system gave no time goes last.
      { path: 'D:/s/take.flac', name: 'take.flac', where: '' },
    ])
    expect(soundsOf(undefined, undefined, 'attached')).toEqual([])
  })

  it('tell the time the way a clock does', () => {
    expect(clock(0)).toBe('0:00')
    expect(clock(187.9)).toBe('3:07')
    expect(clock(3765)).toBe('1:02:45')
    expect(clock(-4)).toBe('0:00')
  })
})
