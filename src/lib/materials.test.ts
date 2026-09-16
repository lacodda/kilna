import { describe, expect, it } from 'vitest'
import type { Asset } from '@/lib/api'
import { groupMaterials, OTHER } from '@/lib/materials'

const asset = (id: string, kind: string | null): Asset =>
  ({ id, kind, path: `/media/${id}`, original_name: `${id}.png` }) as Asset

describe('the files of a work, grouped by what they are for', () => {
  it('shows the groups in the order the work happens in', () => {
    const groups = groupMaterials([
      asset('v1', 'video'),
      asset('r1', null),
      asset('c1', 'cover'),
      asset('f1', 'frame'),
    ])

    expect(groups.map((group) => group.kind)).toEqual(['cover', 'frame', 'video', OTHER])
  })

  it('leaves out a kind with nothing in it', () => {
    // An empty heading is furniture: a work with only a cover should look
    // like a work with only a cover.
    const groups = groupMaterials([asset('c1', 'cover')])

    expect(groups).toHaveLength(1)
    expect(groups[0]?.kind).toBe('cover')
  })

  it('keeps the order the files arrived in, within a group', () => {
    const groups = groupMaterials([asset('f1', 'frame'), asset('f2', 'frame')])

    expect(groups[0]?.assets.map((one) => one.id)).toEqual(['f1', 'f2'])
  })

  it('gathers a kind it does not know rather than dropping it', () => {
    // A kind invented by a later version must still reach the screen; the
    // alternative is a file that exists and is invisible.
    const groups = groupMaterials([asset('x1', 'storyboard')])

    expect(groups).toEqual([{ kind: OTHER, assets: [expect.objectContaining({ id: 'x1' })] }])
  })
})
