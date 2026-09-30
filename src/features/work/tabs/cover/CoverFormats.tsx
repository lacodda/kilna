import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Work } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Chip } from '@/components/ui/chip'
import { SectionLabel } from '@/components/ui/panel'
import { coverFormatsOf } from '@/features/work/tabs/cover/cover'

/**
 * The shapes this work's covers take: one per door of its kind that shows a
 * picture - "YouTube · 16:9", "Streaming · 1:1".
 *
 * Lit where the work already goes out through that door, dimmed where it
 * does not yet: the audio made from a song gets its one YouTube release on
 * the way in, and the square cover waits until a streaming release is added.
 * A strip of facts rather than a switch - the shape follows the release, and
 * adding a release is done where releases are (the mockup's format picker
 * belongs to the cover constructor of v0.88).
 *
 * A kind whose doors show no picture draws nothing.
 */
export function CoverFormats({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const formats = coverFormatsOf(profile.config, work.kind)
  const releases = useQuery({ ...queries.releasesForWork(work.id), enabled: formats.length > 0 })

  if (formats.length === 0) return null
  const doors = new Set((releases.data ?? []).map((release) => release.kind))

  return (
    <section aria-label={t('cover.formats')} className="flex min-w-0 flex-col gap-1.5">
      <div className="flex flex-wrap items-center gap-2">
        <SectionLabel>{t('cover.formats')}</SectionLabel>
        <ul className="flex flex-wrap items-center gap-2">
          {formats.map(({ door, format }) => {
            const held = doors.has(door.key)
            return (
              <li key={door.key} className="flex items-center gap-1.5">
                <Chip
                  variant={held ? 'accent' : 'dashed'}
                  className={held ? undefined : 'opacity-70'}
                >
                  {`${sayLabel(door.label)} · ${format}`}
                </Chip>
                {/* Said in words as well as drawn dim: a dashed border alone
                    is a riddle to anyone who does not know the convention.
                    Only once the releases are in, so it is not said of a door
                    the answer is about to light. */}
                {releases.isSuccess && !held && (
                  <span className="text-xs text-faint">{t('cover.noRelease')}</span>
                )}
              </li>
            )
          })}
        </ul>
      </div>
      <p className="text-xs text-faint">{t('cover.formatsHint')}</p>
    </section>
  )
}
