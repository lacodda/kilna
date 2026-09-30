import { useState, type ReactNode } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQueries, useQuery } from '@tanstack/react-query'
import { Check, X } from 'lucide-react'
import type { ScheduledRelease, Work } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { hasDoors, labelOf, say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { useReleaseMeta } from '@/features/work/tabs/releases/releaseMeta'

/**
 * Where the writing of a publication's release meta stands: running, done
 * with what it wrote, or waiting for the person on the fields they started.
 * Drawn at the top of a publication's Cover and Frame tabs, where "Make…"
 * lands (v0.86).
 *
 * A slim bar, and only while there is something to say: "Make…" starts the
 * meta in the background and opens the cover, and the person who is choosing
 * a picture should still see that the words are being written - and, when
 * they come back with some of them waiting, where to take them. Idle, it
 * draws nothing. Its own questions throughout: it stands on tabs that know
 * nothing about releases.
 */
export function ReleaseMetaStatus({ work }: { work: Work }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { config } = useProfile()
  const doors = hasDoors(config, work.kind)
  const kinds = vocabularyOf(config, work.kind).release_kinds

  // The release whose run finished while this bar was watching, until the
  // person closes the line or opens the release. Held here, not asked: a
  // finished run leaves nothing to ask about when all it did was fill.
  const [written, setWritten] = useState<string | null>(null)

  const { action, writing } = useReleaseMeta(work.kind, (releaseId, finished) => {
    if (finished) setWritten(releaseId)
  })
  const wanted = doors && action !== undefined

  const releases = useQuery({ ...queries.releasesForWork(work.id), enabled: wanted })
  const mine: ScheduledRelease[] = wanted ? (releases.data ?? []) : []
  const proposals = useQueries({
    queries: mine.map((release) => queries.releaseProposals(release.id)),
  })
  const fields = useQuery({
    ...queries.releaseFields(written ?? ''),
    enabled: wanted && written !== null,
  })

  if (!wanted) return null

  const kindOf = (release: ScheduledRelease) => labelOf(kinds, release.kind)
  const open = (releaseId: string) => {
    setWritten(null)
    void navigate(`/works/${work.id}/releases?release=${releaseId}`)
  }

  const running = mine.find((release) => writing(release.id))
  if (running !== undefined) {
    return (
      <Bar>
        <span aria-hidden className="size-2 shrink-0 animate-pulse rounded-full bg-info" />
        <span>{t('releases.metaStatus.writing', { kind: kindOf(running) })}</span>
      </Bar>
    )
  }

  // What waits, counted in fields rather than proposals: "three fields wait"
  // is the size of the job, "one proposal" says nothing about it.
  const waiting = mine
    .map((release, index) => ({
      release,
      count: (proposals[index]?.data ?? []).reduce((sum, one) => sum + one.fields.length, 0),
    }))
    .filter((entry) => entry.count > 0)
  const first = waiting[0]
  if (first !== undefined) {
    const count = waiting.reduce((sum, entry) => sum + entry.count, 0)
    return (
      <Bar>
        <span>
          {waiting.length === 1
            ? t('releases.metaStatus.waiting', { count, kind: kindOf(first.release) })
            : t('releases.metaStatus.waitingMany', { count })}
        </span>
        <Button
          size="xs"
          variant="ghost"
          className="ml-auto"
          onClick={() => open(first.release.id)}
        >
          {t('releases.metaStatus.open')}
        </Button>
      </Bar>
    )
  }

  const done = mine.find((release) => release.id === written)
  if (done !== undefined) {
    const filled = (fields.data ?? [])
      .filter((field) => field.value.trim() !== '')
      .map((field) => sayLabel(field.label))
    return (
      <Bar tone="good">
        <Check aria-hidden className="size-3.5 shrink-0" />
        <span>
          {filled.length === 0
            ? t('releases.metaStatus.written', { kind: kindOf(done) })
            : t('releases.metaStatus.writtenFields', {
                kind: kindOf(done),
                fields: filled.join(', '),
              })}
        </span>
        <Button size="xs" variant="ghost" className="ml-auto" onClick={() => open(done.id)}>
          {t('releases.metaStatus.open')}
        </Button>
        <Button
          size="icon-xs"
          variant="icon"
          title={t('releases.metaStatus.close')}
          aria-label={t('releases.metaStatus.close')}
          onClick={() => setWritten(null)}
        >
          <X aria-hidden />
        </Button>
      </Bar>
    )
  }

  return null
}

/** The mockup's job bar: one line on the info tint, over the tab's content. */
function Bar({ tone = 'info', children }: { tone?: 'info' | 'good'; children: ReactNode }) {
  return (
    <div
      role="status"
      className={cn(
        'flex shrink-0 flex-wrap items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs',
        tone === 'info' ? 'bg-info-soft text-info' : 'bg-good-soft text-good',
      )}
    >
      {children}
    </div>
  )
}
