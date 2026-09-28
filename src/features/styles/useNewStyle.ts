import { useTranslation } from 'react-i18next'
import { attachAsset } from '@/lib/api/assets'
import { createStyleBrick, pasteStyleReference } from '@/lib/api/styles'
import type { StyleBrick } from '@/lib/api/types'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { untitledName } from '@/lib/styleDraft'
import { say } from '@/lib/toast'
import type { PastedPicture } from '@/features/styles/pastedPicture'

/** What a new style is made from: nothing yet, files on disk, or a paste. */
type Source = { paths: string[] } | { pasted: PastedPicture } | null

interface Options {
  /** The type it is made in: the one being looked at, so it does not vanish
   * from the list it was made in, or the profile's first. */
  typeKey: string
  /** Every style of the dictionary, for a name its type does not have yet. */
  bricks: readonly StyleBrick[]
  /** It exists: open it. */
  onMade: (brick: StyleBrick) => void
}

/**
 * Making a style, in one gesture.
 *
 * Until v0.79 a style took three: a dialog for its type and name, a save,
 * and a second opening before a reference could be hung on it. Now the
 * gesture that brings the pictures - dropped on the dashed card, pasted, or
 * picked - makes the style around them and opens it, named "Untitled style"
 * and ready to be named; "New style" does the same with no pictures. A style
 * is made when it is asked for, the way a note is, so the open editor always
 * has a style to write into.
 */
export function useNewStyle({ typeKey, bricks, onMade }: Options) {
  const { t } = useTranslation()

  return useAppMutation({
    mutationFn: async (source: Source) => {
      const taken = bricks.filter((one) => one.type_key === typeKey).map((one) => one.name)
      const brick = await createStyleBrick({
        type_key: typeKey,
        name: untitledName(t('styles.untitled'), taken),
      })
      // The pictures hang on the style, so they follow it. One that fails
      // leaves the style made and says so, rather than taking the style
      // back with it: the pictures that did arrive are worth keeping.
      try {
        if (source !== null && 'paths' in source) {
          for (const path of source.paths) await attachAsset(path, { style_brick_id: brick.id })
        }
        if (source !== null && 'pasted' in source) {
          await pasteStyleReference(brick.id, source.pasted.bytes, source.pasted.name)
        }
      } catch (cause) {
        say.failedTo(t('styles.referenceFailed'), cause)
      }
      return brick
    },
    failure: 'styles.makeFailed',
    refresh: refresh.style,
    onSuccess: onMade,
  })
}
