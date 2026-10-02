import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { deriveWork } from '@/lib/api/links'
import { startCoverTask } from '@/lib/api/ideas'
import { startReleaseTask } from '@/lib/api/releases'
import type { Made, Work } from '@/lib/api/types'
import { humanError } from '@/lib/errors'
import { coverActionOf, ideasOnMake } from '@/lib/ideas'
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

/** One place a made work can go out, as a menu offers it. */
interface Place {
  key: string
  /** The door and the shape its cover takes: "Streaming · 1:1". */
  label: string
}

/** One kind of work that can be made from the source, as a menu offers it. */
interface Makeable {
  key: string
  /** What the entry says: "Make an audio", or "Make a video from this". */
  label: string
  /** Where works of the kind go out, and in what shape: "YouTube · 16:9".
   *  Empty for a kind with no door. */
  description: string
  /** Each place it can go out, to choose from when there are several: a
   *  publication goes out once (ADR 0051), so where is decided when it is
   *  made - and may change until it goes. */
  places: Place[]
}

export interface MakePublication {
  /** What can be made from the source, in the profile's order. */
  kinds: Makeable[]
  /** Make one of `kind`, going out through `door`, or the kind's first. */
  make: (kind: string, door?: string) => void
  /** The kind being made, while it is. */
  making: string | null
}

/** What the gesture came back with: the work, and whether the meta and the
 *  cover's ideas started. */
interface Outcome {
  made: Made
  /** The action that writes the release's meta, when one was started. */
  started: boolean
  /** Why it did not start, when it was asked to and could not. */
  refused: unknown
  /** How many ideas for the cover were asked for; 0 when none were. */
  ideas: number
  /** Why they were not, when they were to be and could not. */
  ideasRefused: unknown
}

/**
 * Make a work from `source` in one gesture (ADR 0047): the new work named by
 * its kind in the window's language, linked, with a release through its
 * first door and no day - then the profile's release action set writing what
 * that release goes out under, the cover's ideas asked for beside it
 * (v0.89, ADR 0050), and the new work opened on its cover, where the next
 * decision is.
 *
 * The overview's Make menu, the header's menu and the Links tab's buttons
 * all make a work this way, so the three cannot drift into three gestures
 * that each do part of it.
 *
 * The meta and the ideas are the gesture's second half and not its
 * condition: an assistant that is not there, or a task already running,
 * leaves the work made and says why either did not start, in a toast that
 * warns rather than fails.
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
    const doors = vocabularyOf(config, kind.key).release_kinds
    return {
      key: kind.key,
      label: doorless ? t('publications.make', said) : t('publications.makeFrom', said),
      description: doorsOf(doors, sayLabel),
      places: doors.map((door) => ({ key: door.key, label: doorsOf([door], sayLabel) })),
    }
  })

  const mutation = useAppMutation({
    mutationFn: async ({ kind, door }: { kind: string; door?: string }): Promise<Outcome> => {
      const made = await deriveWork(source.id, kind, i18n.language, { door })
      const outcome: Outcome = { made, started: false, refused: null, ideas: 0, ideasRefused: null }
      const action = releaseActionOf(config, kind)
      if (made.release_id !== null && action !== undefined) {
        try {
          await startReleaseTask(made.release_id, action.key)
          outcome.started = true
        } catch (cause) {
          outcome.refused = cause
        }
      }
      // The cover's ideas start beside the meta (v0.89): the person lands on
      // the board while both are written. The number is the profile's - three
      // unless it says otherwise, none when it says 0.
      const ideas = coverActionOf(config, kind)
      const count = ideasOnMake(config)
      if (ideas !== undefined && count > 0 && vocabularyOf(config, kind).cover) {
        try {
          await startCoverTask(made.work.id, ideas.key, { count, refine: null, more: false })
          outcome.ideas = count
        } catch (cause) {
          outcome.ideasRefused = cause
        }
      }
      return outcome
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
    onSuccess: ({ made, started, refused, ideas, ideasRefused }) => {
      const vocabulary = vocabularyOf(config, made.work.kind)
      const door =
        vocabulary.release_kinds.find((one) => one.key === made.door) ?? vocabulary.release_kinds[0]
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
        description: ideas > 0 ? `${said} ${t('publications.madeIdeas', { count: ideas })}` : said,
      })
      if (refused !== null) say.warn(t('publications.metaNotStarted'), humanError(refused))
      if (ideasRefused !== null) {
        say.warn(t('publications.ideasNotStarted'), humanError(ideasRefused))
      }

      // Straight to where the next decision is: the cover, for a kind that
      // has one to write; the card itself otherwise.
      const cover = vocabulary.cover
      void navigate(`/works/${made.work.id}${cover ? '/cover' : ''}`)
    },
  })

  return {
    kinds,
    make: (kind, door) => mutation.mutate({ kind, door }),
    making: mutation.isPending ? (mutation.variables?.kind ?? null) : null,
  }
}
