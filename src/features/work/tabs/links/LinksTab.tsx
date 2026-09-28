import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { X } from 'lucide-react'
import { createLink, deleteLink, deriveWork } from '@/lib/api/links'
import type { Link, Work } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { labelOf, say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { Panel } from '@/components/ui/panel'
import { Skeleton } from '@/components/ui/skeleton'
import { Frame, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'

interface Props {
  work: Work
}

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
  const navigate = useNavigate()

  const links = useQuery(queries.links(work.id))

  const remove = useAppMutation({
    mutationFn: (id: string) => deleteLink(id),
    refresh: refresh.link,
    onSuccess: () => say.ok(t('links.removed')),
  })

  const derive = useAppMutation({
    mutationFn: (kind: string) => deriveWork(work.id, kind),
    failure: 'toast.workSaveFailed',
    refresh: [keys.links, keys.works, keys.catalogue],
    onSuccess: (created) => {
      say.ok(t('links.made', { title: created.title }))
      // Straight into the new work: making one is the start of working on it.
      void navigate(`/works/${created.id}/links`)
    },
  })

  // The other kinds of the profile: a song becomes a video, not another song.
  // With one kind there is nothing to make from this, and the row is gone.
  const otherKinds = profile.config.work_kinds.filter((kind) => kind.key !== work.kind)

  return (
    <Frame>
      <Scroll label={t('card.tab.links')} contentClassName="flex flex-col gap-4">
        <Panel className="flex flex-col gap-3 p-4">
          <h3 className="text-sm font-semibold">{t('links.sources')}</h3>

          <Loaded query={links} skeleton={<Skeleton className="h-12 w-full" />} plain>
            {(data) =>
              data.sources.length === 0 ? (
                <p className="text-sm text-dim">{t('links.noSources')}</p>
              ) : (
                <ul className="flex flex-col gap-2">
                  {data.sources.map((link) => (
                    <SourceRow
                      key={link.id}
                      link={link}
                      onRemove={() => remove.mutate(link.id)}
                      removing={remove.isPending}
                    />
                  ))}
                </ul>
              )
            }
          </Loaded>

          <SourcePicker
            work={work}
            taken={new Set(links.data?.sources.map((link) => link.source_id) ?? [])}
          />
        </Panel>

        <Panel className="flex flex-col gap-3 p-4">
          <h3 className="text-sm font-semibold">{t('links.derived')}</h3>

          {links.data !== undefined && links.data.derived.length === 0 && (
            <p className="text-sm text-dim">{t('links.noDerived')}</p>
          )}

          {links.data !== undefined && links.data.derived.length > 0 && (
            <ul className="flex flex-col gap-0.5">
              {links.data.derived.map((entry) => {
                const vocabulary = vocabularyOf(profile.config, entry.kind)
                return (
                  <li key={entry.link_id}>
                    <RowButton
                      onClick={() => void navigate(`/works/${entry.work_id}`)}
                      end={
                        <>
                          {labelOf(profile.config.work_kinds, entry.kind)}
                          {' · '}
                          {labelOf(vocabulary.statuses, entry.status)}
                        </>
                      }
                    >
                      {entry.title}
                    </RowButton>
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
                  {t('links.makeFromThis', { kind: sayLabel(kind.label) })}
                </Button>
              ))}
            </div>
          )}
        </Panel>
      </Scroll>
    </Frame>
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
      {/* `text-sm` here, because a link takes the size of the line it sits in. */}
      <div className="flex items-center gap-2 text-sm">
        <Button
          variant="link"
          onClick={() => void navigate(`/works/${link.source_id}`)}
          className="min-w-0 flex-1 justify-start"
        >
          <span className="truncate">{link.source_title}</span>
        </Button>
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
            <Button variant="link" onClick={() => void navigate(diff)}>
              {t('links.seeDiff')}
            </Button>
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
function SourcePicker({ work, taken }: { work: Work; taken: ReadonlySet<string> }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [query, setQuery] = useState('')

  const works = useQuery({ ...queries.works(), enabled: query.trim() !== '' })

  const link = useAppMutation({
    mutationFn: (sourceId: string) => createLink({ work_id: work.id, source_id: sourceId }),
    refresh: refresh.link,
    onSuccess: (created) => {
      setQuery('')
      say.ok(t('links.linked', { title: created.source_title }))
    },
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
              <RowButton
                disabled={link.isPending}
                onClick={() => link.mutate(candidate.id)}
                end={labelOf(profile.config.work_kinds, candidate.kind)}
              >
                {candidate.title}
              </RowButton>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
