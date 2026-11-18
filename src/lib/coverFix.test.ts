import { describe, expect, it } from 'vitest'
import type { Cover, WorkFrame } from '@/lib/api/types'
import { fixCover, fixFrame } from '@/lib/coverFix'

/*
 * The one change that lets a left-out brick in (v0.134): on the cover, and
 * on a frame's own parts over its cover.
 */

const cover = {
  hero: { card: 'c-wren', references: true },
  framing: {
    layout: 'masthead',
    column: 'centre',
    row: 'bottom',
    size: 'full',
    crop: 'bust',
    place: 'top',
  },
  staging: { subject: null, disc: false, ground: 'palette' },
  bricks: { style: 's-noir', typography: null, dressing: null, background: null, pose: 'p-arms' },
} as unknown as Cover

const frame = {
  framing: null,
  still_camera: true,
  motion_bricks: ['m-rain', 'm-push'],
} as unknown as WorkFrame

describe('fixing a cover', () => {
  it('shows more of the subject, or takes a brick out', () => {
    expect(fixCover(cover, { kind: 'crop', crop: 'waist' }).framing?.crop).toBe('waist')
    expect(
      fixCover(cover, { kind: 'remove', place: 'style', id: 's-noir' }).bricks.style,
    ).toBeNull()
    expect(fixCover(cover, { kind: 'remove', place: 'style', id: 's-noir' }).bricks.pose).toBe(
      'p-arms',
    )
  })

  it('makes a picture of a thing without its hero', () => {
    const fixed = fixCover(cover, { kind: 'subject', subject: 'object' })
    expect(fixed.hero).toBeNull()
    expect(fixed.staging.subject).toBe('object')
  })
})

describe('fixing a frame', () => {
  it('lets the camera move, or takes one movement out', () => {
    expect(fixFrame(frame, cover, { kind: 'movingCamera' })).toEqual({ still_camera: false })
    expect(fixFrame(frame, cover, { kind: 'remove', place: 'motion', id: 'm-push' })).toEqual({
      motion_bricks: ['m-rain'],
    })
  })

  it('shows more of the subject in its own framing, starting from the cover', () => {
    expect(fixFrame(frame, cover, { kind: 'crop', crop: 'waist' })).toMatchObject({
      framing: { layout: 'masthead', crop: 'waist' },
    })
  })
})
