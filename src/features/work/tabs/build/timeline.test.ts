import { describe, expect, it } from 'vitest'
import {
  clips,
  defaultsOf,
  peaksOf,
  pictureAt,
  planOf,
  soundAt,
  storedOf,
  timelineOf,
} from './timeline'

// v0.149, FR-15: the timeline read as the simple-video plugin reads it, so
// the preview is what the render makes.
describe('the timeline of a simple video', () => {
  const defaults = { fade_in: 0, fade_out: 6 }

  it('renders by the settings where the work keeps none', () => {
    const plan = planOf({}, 200, defaults)
    expect(plan.sound).toBe(200)
    expect([plan.audioFadeOut, plan.videoFadeOut]).toEqual([6, 6])
    expect(plan.total).toBe(200)
  })

  it('trims, holds the picture around the sound and keeps the gain in bounds', () => {
    const plan = planOf(
      { trim_start: 2.5, trim_end: 102.5, gain_db: 40, lead_in: 1, tail: 2 },
      200,
      defaults,
    )
    expect([plan.trimStart, plan.sound, plan.total]).toEqual([2.5, 100, 103])
    expect(plan.gainDb).toBe(18)
  })

  it('reads what the fields keep and writes back only what was set', () => {
    const kept = timelineOf({ simple_video: { trim_start: 1.23456, tail: 'soon', lead_in: 2 } })
    expect(kept).toEqual({ trim_start: 1.23456, lead_in: 2 })
    expect(storedOf(kept)).toEqual({ trim_start: 1.235, lead_in: 2 })
    expect(timelineOf({})).toEqual({})
    expect(defaultsOf({ fade_out: 3, fade_in: -1 })).toEqual({ fade_in: 0, fade_out: 3 })
  })

  it('fades the picture and the sound where the render does', () => {
    const plan = planOf(
      { lead_in: 1, tail: 1, audio_fade_in: 1, audio_fade_out: 2, video_fade_in: 1 },
      10,
      defaults,
    )
    expect(pictureAt(plan, 0)).toBe(0)
    expect(pictureAt(plan, 0.5)).toBe(0.5)
    expect(pictureAt(plan, 6)).toBe(1)
    expect(pictureAt(plan, plan.total)).toBe(0)
    expect(soundAt(plan, 0.5)).toBe(0)
    expect(soundAt(plan, 1.5)).toBe(0.5)
    expect(soundAt(plan, 6)).toBe(1)
    expect(soundAt(plan, 10)).toBeCloseTo(0.5)
    expect(soundAt(plan, 11.5)).toBe(0)
  })

  it('draws a waveform from the loudest of each bucket and says when the gain clips', () => {
    const left = new Float32Array([0.1, -0.5, 0.2, 0.9])
    const right = new Float32Array([0.3, 0.1, -0.95, 0.2])
    expect(peaksOf([left, right], 2).map((p) => Math.round(p * 100))).toEqual([50, 95])
    expect(clips(0.9, 0)).toBe(false)
    expect(clips(0.9, 3)).toBe(true)
  })
})
