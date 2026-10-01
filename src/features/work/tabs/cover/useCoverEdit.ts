import { useQueryClient } from '@tanstack/react-query'
import type { Cover, Work } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'

/**
 * Change a work's cover, a part at a time, sending the whole of it.
 *
 * Made over the cover in the cache, not the one a render was handed, and
 * written back into the cache before the save goes - the frame's way (see
 * FrameTab): two choices made in a row both land, rather than the second
 * carrying the first one's old value back, and the constructor shows a
 * choice at once. The whole cover travels so the log's `before` holds it
 * whole and an undo puts it back. A refusal reads the work again, so what is
 * shown is what is stored.
 */
export function useCoverEdit(work: Work) {
  const client = useQueryClient()
  const save = useAppMutation({
    mutationFn: (cover: Cover) => updateWork(work.id, { cover }),
    failure: 'cover.saveFailed',
    refresh: [keys.work(work.id), keys.pictures],
    onSuccess: (updated) => client.setQueryData(keys.work(work.id), updated),
  })

  const current = (): Cover =>
    client.getQueryData<Work | null>(keys.work(work.id))?.cover ?? work.cover

  const change = (patch: (cover: Cover) => Cover) => {
    const held = client.getQueryData<Work | null>(keys.work(work.id))
    const next = patch(current())
    if (held != null) client.setQueryData<Work>(keys.work(work.id), { ...held, cover: next })
    save.mutate(next, {
      onError: () => void client.invalidateQueries({ queryKey: keys.work(work.id) }),
    })
  }

  return { change, current, saving: save.isPending }
}

/**
 * Whether a cover holds anything at all - what keeps the Cover tab on a work
 * whose kind has none. The twin of `Cover::holds_anything` on the Rust side,
 * kept beside the one place the window asks.
 */
export function coverHoldsAnything(cover: Cover): boolean {
  const written = [cover.idea, cover.scene, cover.picture, cover.negative, cover.typography].some(
    (text) => text.trim() !== '',
  )
  const chosen =
    cover.hero !== null ||
    cover.framing !== null ||
    Object.values(cover.bricks).some((id) => id !== null) ||
    cover.accent !== null ||
    cover.mark.variant !== null ||
    Object.keys(cover.details).length > 0 ||
    cover.lettering.title !== null ||
    cover.lettering.apart ||
    Object.keys(cover.lettering.captions).length > 0
  return written || chosen
}
