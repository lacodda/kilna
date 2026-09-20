import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { X } from 'lucide-react'
import {
  createLink,
  deleteLink,
  deriveWork,
  listLinks,
  listWorks,
  type Link,
  type Work,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { labelOf, useProfile, vocabularyOf } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Panel } from '@/components/ui/panel'
import { Skeleton } from '@/components/ui/Skeleton'

interface Props {
  work: Work
}

/** The queries a link changes: both cards, and the feed. */
const REFRESHED = [keys.links, keys.journal] as const

/**
 * What this work was made from, and what was made from it.
 *
 * A video from a song, a video from an article: the link says so in both
 * cards, and remembers the version of the source it was taken at. When the
 * source moves on, the card says *changed since* and points at the diff —
 * the fact, not a verdict; nothing in the work is touched on that account
 * (decision of 2026-09-10, the same rule a stale score follows). Making a
 * work from this one, or naming a source for it, both live here: "in the
 * song you see the clip, in the clip you see the song, and either can start
 * the other".
 */
export function LinksTab({ work }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const navigate = useNavigate()

  const links = useQuery({
    queryKey: keys.linksFor(work.id),
    queryFn: () => listLinks(work.id),
  })

  const refresh = () => {
    for (const key of REFRESHED) void client.invalidateQueries({ queryKey: key })
  }

  const remove = useMutation({
    mutationFn: (id: string) => deleteLink(id),
    onSuccess: () => {
      refresh()
      say.ok(t('links.removed'))
    },
    onError: (cause) => say.failed(cause),
  })

  const derive = useMutation({
    mutationFn: (kind: string) => deriveWork(work.id, kind),
    onSuccess: (created) => {
      refresh()
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.catalogue })
      say.ok(t('links.made', { title: created.title }))
      // Straight into the new work: making one is the start of working on it.
      void navigate(`/works/${created.id}/links`)
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

  // The other kinds of the profile: a song becomes a video, not another song.
  // With one kind there is nothing to make from this, and the row is gone.
  const otherKinds = profile.config.work_kinds.filter((kind) => kind.key !== work.kind)

  return (
    <div className="flex flex-col gap-4">
      <Panel className="flex flex-col gap-3 p-4">
        <h3 className="text-sm font-semibold">{t('links.sources')}</h3>

        {links.isPending && <Skeleton className="h-12 w-full" />}
        {links.isError && (
          <p role="alert" className="text-sm text-bad">
            {t('toast.loadFailed')}
          </p>
        )}

        {links.data !== undefined && links.data.sources.length === 0 && (
          <p className="text-sm text-dim">{t('links.noSources')}</p>
        )}

        {links.data !== undefined && links.data.sources.length > 0 && (
          <ul className="flex flex-col gap-2">
            {links.data.sources.map((link) => (
              <SourceRow
                key={link.id}
                link={link}
                onRemove={() => remove.mutate(link.id)}
                removing={remove.isPending}
              />
            ))}
          </ul>
        )}

        <SourcePicker
          work={work}
          taken={new Set(links.data?.sources.map((link) => link.source_id) ?? [])}
          onLinked={refresh}
        />
      </Panel>

      <Panel className="flex flex-col gap-3 p-4">
        <h3 className="text-sm font-semibold">{t('links.derived')}</h3>

        {links.data !== undefined && links.data.derived.length === 0 && (
          <p className="text-sm text-dim">{t('links.noDerived')}</p>
        )}

        {links.data !== undefined && links.data.derived.length > 0 && (
          <ul className="flex flex-col gap-2">
            {links.data.derived.map((entry) => {
              const vocabulary = vocabularyOf(profile.config, entry.kind)
              return (
                <li key={entry.link_id}>
                  <button
                    type="button"
                    onClick={() => void navigate(`/works/${entry.work_id}`)}
                    className="flex w-full cursor-pointer items-center gap-2 rounded-xl border border-line px-3 py-2 text-left transition-colors hover:bg-soft"
                  >
                    <span className="min-w-0 flex-1 truncate text-sm font-medium">{entry.title}</span>
                    <span className="text-xs text-dim">
                      {labelOf(profile.config.work_kinds, entry.kind)}
                      {' · '}
                      {labelOf(vocabulary.statuses, entry.status)}
                    </span>
                  </button>
                </li>
              )
            })}
          </ul>
        )}

        {otherKinds.length > 0 && (
          <div className="flex flex-wrap items-center gap-2">
            {otherKinds.map((kind) => (
              <Button
                key={kind.key}
                size="sm"
                disabled={derive.isPending}
                onClick={() => derive.mutate(kind.key)}
              >
                {t('links.makeFromThis', { kind: kind.label })}
              </Button>
            ))}
          </div>
        )}
      </Panel>
    </div>
  )
}

/**
 * One source: what it is, where it stands, the version this work was taken
 * at — and, when the source has moved on since, the mark and the way to the
 * diff. Both revisions are shown, because "changed since r2" is what a
 * person needs to decide whether to look.
 */
function SourceRow({
  link,
  onRemove,
  removing,
}: {
  link: Link
  onRemove: () => void
  removing: boolean
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  const navigate = useNavigate()
  const vocabulary = vocabularyOf(profile.config, link.source_kind)

  // The diff of the source's own versions: the one this was taken at against
  // the one it is on now. Opens on the versions tab with both selected.
  const diff =
    link.source_version_id !== null && link.source_current_version_id !== null
      ? `/works/${link.source_id}/versions?version=${link.source_current_version_id}&compare=${link.source_version_id}`
      : `/works/${link.source_id}/versions`

  return (
    <li
      className={cn(
        'flex flex-col gap-1.5 rounded-xl border px-3 py-2',
        link.drifted ? 'border-warn/50' : 'border-line',
      )}
    >
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => void navigate(`/works/${link.source_id}`)}
          className="min-w-0 flex-1 cursor-pointer truncate text-left text-sm font-medium hover:underline"
        >
          {link.source_title}
        </button>
        <span className="text-xs text-dim">
          {labelOf(profile.config.work_kinds, link.source_kind)}
          {' · '}
          {labelOf(vocabulary.statuses, link.source_status)}
        </span>
        <Button
          variant="danger"
          size="icon-sm"
          title={t('links.remove')}
          aria-label={t('links.remove')}
          disabled={removing}
          onClick={onRemove}
        >
          <X aria-hidden className="size-3.5" />
        </Button>
      </div>

      <p className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-dim">
        <span>{t(`links.role.${link.role}`, { defaultValue: link.role })}</span>
        {link.taken_revision !== null && (
          <span>{t('links.takenAt', { revision: link.taken_revision })}</span>
        )}
        {link.drifted ? (
          <>
            <span className="text-warn">
              {link.current_revision !== null && link.current_revision !== link.taken_revision
                ? t('links.driftedTo', { revision: link.current_revision })
                : t('links.driftedInPlace')}
            </span>
            <button
              type="button"
              onClick={() => void navigate(diff)}
              className="cursor-pointer text-accent-2 underline decoration-dotted underline-offset-2"
            >
              {t('links.seeDiff')}
            </button>
          </>
        ) : (
          link.source_version_id !== null && <span>{t('links.unchanged')}</span>
        )}
      </p>
    </li>
  )
}

/**
 * Name a source by typing its title: a work made from nothing gains one
 * here, a work made from one gains a second. The whole catalogue is already
 * in hand; the matches are its titles, a few at a time.
 */
function SourcePicker({
  work,
  taken,
  onLinked,
}: {
  work: Work
  taken: ReadonlySet<string>
  onLinked: () => void
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [query, setQuery] = useState('')

  const works = useQuery({
    queryKey: keys.works,
    queryFn: () => listWorks(),
    enabled: query.trim() !== '',
  })

  const link = useMutation({
    mutationFn: (sourceId: string) => createLink({ work_id: work.id, source_id: sourceId }),
    onSuccess: (created) => {
      setQuery('')
      onLinked()
      say.ok(t('links.linked', { title: created.source_title }))
    },
    onError: (cause) => say.failed(cause),
  })

  const needle = query.trim().toLowerCase()
  const matches =
    needle === ''
      ? []
      : (works.data ?? [])
          .filter(
            (candidate) =>
              candidate.id !== work.id &&
              !taken.has(candidate.id) &&
              candidate.title.toLowerCase().includes(needle),
          )
          .slice(0, 8)

  return (
    <div className="flex flex-col gap-1.5">
      <Input
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder={t('links.pickPlaceholder')}
        aria-label={t('links.pickPlaceholder')}
      />
      {needle !== '' && (
        <ul className="flex flex-col gap-1" aria-label={t('links.matches')}>
          {matches.length === 0 && works.data !== undefined && (
            <li className="px-2 py-1 text-xs text-faint">{t('links.noMatch')}</li>
          )}
          {matches.map((candidate) => (
            <li key={candidate.id}>
              <button
                type="button"
                disabled={link.isPending}
                onClick={() => link.mutate(candidate.id)}
                className="flex w-full cursor-pointer items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-sm transition-colors hover:bg-soft"
              >
                <span className="min-w-0 flex-1 truncate">{candidate.title}</span>
                <span className="text-xs text-faint">
                  {labelOf(profile.config.work_kinds, candidate.kind)}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
