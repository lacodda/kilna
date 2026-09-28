import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Link as RouterLink, useNavigate } from 'react-router'
import { ChevronRight, X } from 'lucide-react'
import { createLink, deleteLink, deriveWork } from '@/lib/api/links'
import type { Derived, Link, Work } from '@/lib/api/types'
import { coverImageFor } from '@/lib/cover'
import { formatDay } from '@/lib/format'
import { today } from '@/lib/month'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { useCovers } from '@/lib/useCovers'
import { labelOf, say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { ListRow, RowButton } from '@/components/ui/list-row'
import { Panel, SectionLabel } from '@/components/ui/panel'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'

interface Props {
  work: Work
}

/** The first role a link can have; the backend's `link::DONOR`. A source
 *  in that role is what the panel it sits in already says it is. */
const DONOR = 'donor'

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
 *
 * Two panels of rows, the mockup's: a caption across the top, one line per
 * work with its cover beside it, and what adds to the list standing in the
 * panel's foot. The rows were cards inside a panel until v0.80.
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
  // With one kind there is nothing to make from this, and the foot is gone.
  const otherKinds = profile.config.work_kinds.filter((kind) => kind.key !== work.kind)

  return (
    <Frame>
      <Scroll label={t('card.tab.links')} contentClassName="flex flex-col gap-2.5">
        <LinkPanel
          title={t('links.sources')}
          foot={
            <SourcePicker
              work={work}
              taken={new Set(links.data?.sources.map((link) => link.source_id) ?? [])}
            />
          }
        >
          <Loaded query={links} skeleton={<SkeletonList rows={1} />} plain>
            {(data) =>
              data.sources.length === 0 ? (
                <Nothing>{t('links.noSources')}</Nothing>
              ) : (
                <ul>
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
        </LinkPanel>

        <LinkPanel
          title={t('links.derived')}
          foot={
            otherKinds.length === 0
              ? undefined
              : otherKinds.map((kind) => (
                  <Button
                    key={kind.key}
                    size="sm"
                    disabled={derive.isPending}
                    onClick={() => derive.mutate(kind.key)}
                  >
                    {t('links.makeFromThis', { kind: sayLabel(kind.label) })}
                  </Button>
                ))
          }
        >
          {/* The sources' panel above says it when the links fail to load;
              one question failing is one message, not two. */}
          {links.data !== undefined &&
            (links.data.derived.length === 0 ? (
              <Nothing>{t('links.noDerived')}</Nothing>
            ) : (
              <ul className="divide-y divide-line">
                {links.data.derived.map((entry) => (
                  <DerivedRow key={entry.link_id} entry={entry} />
                ))}
              </ul>
            ))}
        </LinkPanel>
      </Scroll>
    </Frame>
  )
}

/** One of the tab's two panels: its caption, its rows, and what adds to them. */
function LinkPanel({
  title,
  foot,
  children,
}: {
  title: string
  foot?: ReactNode
  children: ReactNode
}) {
  return (
    // `overflow-hidden`: a row's hover tint stops at the panel's rounded
    // corners rather than bulging past them.
    <Panel className="shrink-0 overflow-hidden">
      <div className="border-b border-line px-3.25 py-2.5">
        <SectionLabel>{title}</SectionLabel>
      </div>
      {children}
      {foot === undefined ? null : (
        <div className="flex flex-wrap items-center gap-1.5 border-t border-line px-3.25 py-2.25">
          {foot}
        </div>
      )}
    </Panel>
  )
}

/** What a panel says when it has no rows: a line, where the rows would be. */
function Nothing({ children }: { children: ReactNode }) {
  return <p className="px-3.25 py-2.5 text-sm text-dim">{children}</p>
}

/** A work's cover at the size of a row: what tells two rows apart before
 *  their titles are read, as it does on the card's own band. */
function Thumb({ workId }: { workId: string }) {
  const cover = useCovers().get(workId)
  return (
    <span
      aria-hidden
      className="size-5.5 shrink-0 rounded-sm"
      style={{ background: coverImageFor(workId, cover) }}
    />
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

  // What the source is and where it stands, after its name. The role only
  // when it is not the plain one: every row of this panel is a source.
  const about = [
    link.role === DONOR ? null : t(`links.role.${link.role}`, { defaultValue: link.role }),
    labelOf(profile.config.work_kinds, link.source_kind),
    labelOf(vocabulary.statuses, link.source_status),
  ]
    .filter((part) => part !== null)
    .join(' · ')

  return (
    <ListRow
      render={<li />}
      start={<Thumb workId={link.source_id} />}
      end={
        <>
          {link.taken_revision !== null && (
            <Chip>{t('links.takenAt', { revision: link.taken_revision })}</Chip>
          )}
          {link.drifted ? (
            <>
              <Chip variant="warn">
                {link.current_revision !== null && link.current_revision !== link.taken_revision
                  ? t('links.driftedTo', { revision: link.current_revision })
                  : t('links.driftedInPlace')}
              </Chip>
              <Button size="sm" onClick={() => void navigate(diff)}>
                {t('links.seeDiff')}
              </Button>
            </>
          ) : (
            link.source_version_id !== null && <Chip variant="good">{t('links.unchanged')}</Chip>
          )}
          <Button
            variant="icon"
            size="icon-sm"
            title={t('links.remove')}
            aria-label={t('links.remove')}
            disabled={removing}
            onClick={onRemove}
            className="not-data-disabled:hover:text-bad"
          >
            <X aria-hidden />
          </Button>
        </>
      }
    >
      <RouterLink
        to={`/works/${link.source_id}`}
        className="font-semibold text-text no-underline hover:underline"
      >
        {link.source_title}
      </RouterLink>{' '}
      <span className="text-xs text-faint">· {about}</span>
    </ListRow>
  )
}

/**
 * A work made from this one: what it is, where it stands, and where its
 * releases are - the day it came out, or the day it comes out. "When did the
 * clip come out" is asked on the song's card, and was answered only on the
 * clip's own card or in the calendar until v0.80. The whole row opens it.
 */
function DerivedRow({ entry }: { entry: Derived }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const navigate = useNavigate()
  const vocabulary = vocabularyOf(profile.config, entry.kind)

  // A day already past is a release that did not go out when it was meant
  // to: still the next one, and worth the warning colour.
  const next = entry.next_scheduled_at
  const late = next !== null && next < today()

  return (
    <li>
      <RowButton
        onClick={() => void navigate(`/works/${entry.work_id}`)}
        start={<Thumb workId={entry.work_id} />}
        // The divided rows of a panel, as `ListRow` draws them. Its `py-row`
        // is not a padding tailwind-merge knows, and would sit beside the
        // button's own rather than replace it; `py-2.5` is the same 10px.
        className="rounded-none px-3 py-2.5"
        end={
          <>
            <span>
              {labelOf(profile.config.work_kinds, entry.kind)}
              {' · '}
              {labelOf(vocabulary.statuses, entry.status)}
            </span>
            {entry.released > 0 && entry.last_released_at !== null && (
              <Chip variant="good">
                {entry.released === 1
                  ? t('links.releasedOnce', { day: formatDay(entry.last_released_at) })
                  : t('links.releasedMany', {
                      released: entry.released,
                      day: formatDay(entry.last_released_at),
                    })}
              </Chip>
            )}
            {next !== null && (
              <Chip variant={late ? 'warn' : 'info'}>
                {t(late ? 'links.late' : 'links.scheduled', { day: formatDay(next) })}
              </Chip>
            )}
            <ChevronRight aria-hidden className="size-3.5" />
          </>
        }
      >
        {entry.title}
      </RowButton>
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
    <div className="flex min-w-0 flex-1 flex-col gap-1.5">
      <Input
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder={t('links.pickPlaceholder')}
        aria-label={t('links.pickPlaceholder')}
      />
      {needle !== '' && (
        <ul className="flex flex-col gap-0.5" aria-label={t('links.matches')}>
          {matches.length === 0 && works.data !== undefined && (
            <li className="px-2.5 py-1 text-xs text-faint">{t('links.noMatch')}</li>
          )}
          {matches.map((candidate) => (
            <li key={candidate.id}>
              <RowButton
                disabled={link.isPending}
                onClick={() => link.mutate(candidate.id)}
                start={<Thumb workId={candidate.id} />}
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
