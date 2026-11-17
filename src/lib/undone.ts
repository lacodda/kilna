import i18n from '@/i18n'
import type { Undoable } from '@/lib/api/types'

/**
 * What an undo took back, as the toast says it.
 *
 * One gesture is one undo (v0.114): keeping a package or making a clip from a
 * song records several operations, and they go back together. The sentence
 * names the last of them - the one the offer was made for - and says how many
 * went with it, so "Taken back: adding a scene" does not hide that the work
 * and its two other scenes went too.
 */
export function sayUndone(taken: Undoable): string {
  const what = i18n.t(taken.action, taken.params)
  return taken.also > 0
    ? i18n.t('undo.doneWith', { what, count: taken.also })
    : i18n.t('undo.done', { what })
}
