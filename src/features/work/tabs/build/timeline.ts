import type { JsonValue, Meta } from '@/lib/api/types'

/*
 * A work's simple video as a timeline (v0.149, FR-15): what of the track
 * plays, how loud, how it fades, how long the picture stands before the
 * sound and after it. Kept in the work's fields under `simple_video`, in the
 * shape the simple-video plugin renders by - the plugin's README holds it -
 * and read here the way the plugin reads it, so the preview is the render.
 */

/** The key the timeline is kept under in a work's fields. */
export const TIMELINE_KEY = 'simple_video'

/** A timeline as kept: every value optional, a missing one the settings'. */
export type Timeline = {
  trim_start?: number
  trim_end?: number
  gain_db?: number
  audio_fade_in?: number
  audio_fade_out?: number
  video_fade_in?: number
  video_fade_out?: number
  lead_in?: number
  tail?: number
}

/** The fades the renderer uses when the timeline names none. */
export type Defaults = { fade_in: number; fade_out: number }

/** How far the gain goes, as the renderer holds it. */
export const GAIN_RANGE = { min: -24, max: 18 } as const

const KEYS = [
  'trim_start',
  'trim_end',
  'gain_db',
  'audio_fade_in',
  'audio_fade_out',
  'video_fade_in',
  'video_fade_out',
  'lead_in',
  'tail',
] as const

/** The timeline a work's fields keep: each value a number, the rest left out. */
export function timelineOf(meta: Meta): Timeline {
  const kept = meta[TIMELINE_KEY]
  if (kept === null || typeof kept !== 'object' || Array.isArray(kept)) return {}
  const out: Timeline = {}
  for (const key of KEYS) {
    const value = kept[key]
    if (typeof value === 'number' && Number.isFinite(value)) out[key] = value
  }
  return out
}

/** A timeline as the fields keep it: only what was set. */
export function storedOf(timeline: Timeline): JsonValue {
  const out: Record<string, JsonValue> = {}
  for (const key of KEYS) {
    const value = timeline[key]
    if (value !== undefined) out[key] = Math.round(value * 1000) / 1000
  }
  return out
}

/** The fades a plugin's settings give, the renderer's own when unset. */
export function defaultsOf(settings: Readonly<Record<string, JsonValue>>): Defaults {
  const number = (key: string, or: number) => {
    const value = settings[key]
    return typeof value === 'number' && value >= 0 ? value : or
  }
  return { fade_in: number('fade_in', 0), fade_out: number('fade_out', 6) }
}

/** What a render does, every value settled - as the plugin's `Plan`. */
export type Plan = {
  trimStart: number
  /** Seconds of the track that play. */
  sound: number
  gainDb: number
  audioFadeIn: number
  audioFadeOut: number
  videoFadeIn: number
  videoFadeOut: number
  leadIn: number
  tail: number
  /** The whole video: the picture before the sound, the sound, after it. */
  total: number
}

/** The plan a render of a track `length` seconds long follows. */
export function planOf(timeline: Timeline, length: number, defaults: Defaults): Plan {
  const atLeastZero = (value: number | undefined, or: number) => Math.max(0, value ?? or)
  const trimStart = Math.min(atLeastZero(timeline.trim_start, 0), length)
  const end =
    timeline.trim_end !== undefined && timeline.trim_end > trimStart
      ? Math.min(timeline.trim_end, length)
      : length
  const sound = Math.max(0, end - trimStart)
  const fit = (seconds: number) => Math.min(seconds, sound)
  const leadIn = atLeastZero(timeline.lead_in, 0)
  const tail = atLeastZero(timeline.tail, 0)
  return {
    trimStart,
    sound,
    gainDb: Math.min(GAIN_RANGE.max, Math.max(GAIN_RANGE.min, timeline.gain_db ?? 0)),
    audioFadeIn: fit(atLeastZero(timeline.audio_fade_in, defaults.fade_in)),
    audioFadeOut: fit(atLeastZero(timeline.audio_fade_out, defaults.fade_out)),
    videoFadeIn: atLeastZero(timeline.video_fade_in, defaults.fade_in),
    videoFadeOut: atLeastZero(timeline.video_fade_out, defaults.fade_out),
    leadIn,
    tail,
    total: leadIn + sound + tail,
  }
}

/** How bright the picture is at second `t` of the video, from 0 to 1. */
export function pictureAt(plan: Plan, t: number): number {
  if (t < 0 || t > plan.total) return 0
  const fadeIn = plan.videoFadeIn > 0 ? Math.min(1, t / plan.videoFadeIn) : 1
  const toEnd = plan.total - t
  const fadeOut = plan.videoFadeOut > 0 ? Math.min(1, toEnd / plan.videoFadeOut) : 1
  return Math.max(0, Math.min(fadeIn, fadeOut))
}

/** How loud the sound is at second `t` of the video, as a factor: the gain
 *  and the fades, silence before the sound and after it. */
export function soundAt(plan: Plan, t: number): number {
  const into = t - plan.leadIn
  if (into < 0 || into > plan.sound) return 0
  const gain = 10 ** (plan.gainDb / 20)
  const fadeIn = plan.audioFadeIn > 0 ? Math.min(1, into / plan.audioFadeIn) : 1
  const left = plan.sound - into
  const fadeOut = plan.audioFadeOut > 0 ? Math.min(1, left / plan.audioFadeOut) : 1
  return gain * Math.max(0, Math.min(fadeIn, fadeOut))
}

/** The loudest a channel gets, `count` buckets across: what a waveform is
 *  drawn from. */
export function peaksOf(channels: readonly Float32Array[], count: number): number[] {
  const length = channels[0]?.length ?? 0
  if (length === 0 || count <= 0) return []
  const size = Math.max(1, Math.floor(length / count))
  const out: number[] = []
  for (let bucket = 0; bucket < count; bucket++) {
    const from = bucket * size
    if (from >= length) break
    const to = Math.min(length, from + size)
    let peak = 0
    for (const channel of channels) {
      for (let at = from; at < to; at++) {
        const value = Math.abs(channel[at] ?? 0)
        if (value > peak) peak = value
      }
    }
    out.push(peak)
  }
  return out
}

/** Whether the track clips once the gain is put on: its loudest sample over
 *  full scale. */
export function clips(peak: number, gainDb: number): boolean {
  return peak * 10 ** (gainDb / 20) > 1
}
