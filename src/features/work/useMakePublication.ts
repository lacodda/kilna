import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { deriveWork } from '@/lib/api/links'
import { startReleaseTask } from '@/lib/api/releases'
import type { Made, Work } from '@/lib/api/types'
import { humanError } from '@/lib/errors'
import { doorsOf, takesAn, wordOf } from '@/lib/overview'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say, toastManager } from '@/lib/toast'
import {
  hasDoors,
  publicationKinds,
  releaseActionOf,
  say as sayLabel,
  useProfile,
  vocabularyOf,
} from '@/lib/useProfile'

/** One kind of work that can be made from the source, as a menu offers it. */
interface Makeable {
  key: string
  /** What the entry says: "Make an audio", or "Make a video from this". */
  label: string
  /** Where works of the kind go out, and in what shape: "YouTube · 16:9".
   *  Empty for a kind with no door. */
  description: string
}

export interface MakePublication {
  /** What can be made from the source, in the profile's order. */
  kinds: Makeable[]
  make: (kind: string) => void
  /** The kind being made, while it is. */
  making: string | null
}

/** What the gesture came back with: the work, and whether the meta started. */
interface Outcome {
  made: Made
  /** The action that writes the release's meta, when one was started. */
  started: boolean
  /** Why it did not start, when it was asked to and could not. */
  refused: unknown
}

/**
 * Make a work from `source` in one gesture (ADR 0047): the new work named by
 * its kind in the window's language, linked, with a release through its
 * first door and no day - then the profile's release action set writing what
 * that release goes out under, and the new work opened on its cover, where
 * the next decision is.
 *
 * The overview's Make menu, the header's menu and the Links tab's buttons
 * all make a work this way, so the three cannot drift into three gestures
 * that each do part of it.
 *
 * The meta is the gesture's second half and not its condition: an assistant
 * that is not there, or a task already running, leaves the work made and
 * says why the meta did not start, in a toast that warns rather than fails.
 */
export function useMakePublication(source: Work): MakePublication {
  const { t, i18n } = useTranslation()
  const profile = useProfile()
  const navigate = useNavigate()
  const config = profile.config

  // A song speaks through its publications, so its menu lists exactly those
  // - "Make an audio". Anything else offers every other kind, said as made
  // from it - "Make a song from this": a video can be where a song starts.
  const doorless = !hasDoors(config, source.kind)
  const offered = (doorless ? publicationKinds(config) : config.work_kinds).filter(
    (kind) => kind.key !== source.kind,
  )

  const kinds: Makeable[] = offered.map((kind) => {
    const word = wordOf(sayLabel(kind.label), i18n.language)
    const said = { kind: word, context: takesAn(word) ? 'an' : undefined }
    return {
      key: kind.key,
      label: doorless ? t('publications.make', said) : t('publications.makeFrom', said),
      description: doorsOf(vocabularyOf(config, kind.key).release_kinds, sayLabel),
    }
  })

  const mutation = useAppMutation({
    mutationFn: async (kind: string): Promise<Outcome> => {
      const made = await deriveWork(source.id, kind, i18n.language)
      const action = releaseActionOf(config, kind)
      if (made.release_id === null || action === undefined) {
        return { made, started: false, refused: null }
      }
      try {
        await startReleaseTask(made.release_id, action.key)
        return { made, started: true, refused: null }
      } catch (cause) {
        return { made, started: false, refused: cause }
      }
    },
    failure: 'publications.makeFailed',
    refresh: [
      keys.works,
      keys.catalogue,
      keys.links,
      keys.releases,
      keys.calendar,
      keys.releaseQueue,
      keys.publications,
      keys.activeTasks,
      keys.allChats,
    ],
    onSuccess: ({ made, started, refused }) => {
      const vocabulary = vocabularyOf(config, made.work.kind)
      const door = vocabulary.release_kinds[0]
      const said =
        made.release_id === null || door === undefined
          ? t('publications.madeLinked')
          : t(started ? 'publications.madeWriting' : 'publications.madeRelease', {
              door: sayLabel(door.label),
            })
      // A toast with a second line, which the app's `say.ok` does not carry:
      // what was made is the headline, and what came with it is the line.
      toastManager.add({
        type: 'success',
        title: t('publications.made', { title: made.work.title }),
        description: said,
      })
      if (refused !== null) say.warn(t('publications.metaNotStarted'), humanError(refused))

      // Straight to where the next decision is: the cover, for a kind that
      // has one to write; the card itself otherwise.
      const cover = vocabulary.cover_blocks.length > 0
      void navigate(`/works/${made.work.id}${cover ? '/cover' : ''}`)
    },
  })

  return {
    kinds,
    make: (kind) => mutation.mutate(kind),
    making: mutation.isPending ? (mutation.variables ?? null) : null,
  }
}
