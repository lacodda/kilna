import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { ChevronDown } from 'lucide-react'
import type { Publication, PublicationBasis, Work } from '@/lib/api/types'
import { coverImageFor } from '@/lib/cover'
import { formatDay } from '@/lib/format'
import { badgeVariantOf } from '@/lib/markIcon'
import { today } from '@/lib/month'
import { doorsOf, FACT_TONE, publicationFact } from '@/lib/overview'
import { queries } from '@/lib/query/queries'
import { useCovers } from '@/lib/useCovers'
import { labelOf, say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { RowButton } from '@/components/ui/list-row'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { Loaded } from '@/components/Loaded'
import { Invite, useLook, Widget, WidgetSkeleton } from '@/features/work/tabs/overview/Widget'
import { useMakePublication, type MakePublication } from '@/features/work/useMakePublication'

/** How many publications the widget lists before it says how many more. */
const ROWS = { s: 3, m: 6, l: 8 } as const

/**
 * What a song goes out as: its publications, each with where it stands, and
 * the way to make another (v0.86, ADR 0047).
 *
 * A song has no releases of its own - the clip, the audio and the shorts made
 * from it go out, each with its releases, files, cover and comments. So its
 * board leads with them: one row per publication - what it is, what it is
 * called, the fact of it with its day, where it goes out and what was said
 * under it - and under the list the two things the song itself is: where it
 * stands, and on which publication's fact; and how many comments are waiting
 * across all of them.
 *
 * "Make…" is the mockup's menu: one entry per kind that goes out, each with
 * where it goes out, and one gesture that makes, links, plans the release and
 * opens the cover (`useMakePublication`).
 */
export function PublicationsWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const { size } = useLook()
  const publications = useQuery(queries.publications(work.id))
  const make = useMakePublication(work)

  const empty = publications.data?.items.length === 0
  const menu = <MakeMenu make={make} />

  return (
    <Widget
      caption={t('publications.caption')}
      // The Links tab holds every link either way; the rows here each open
      // their own publication.
      to={`/works/${work.id}/links`}
      go={t('card.tab.links')}
      tone={empty ? 'empty' : 'plain'}
      aside={
        empty ? undefined : (
          <>
            <span className="hidden min-w-0 truncate text-xs text-faint @2xl:inline">
              {t('publications.hint')}
            </span>
            {menu}
          </>
        )
      }
      actions={empty ? menu : undefined}
    >
      <Loaded query={publications} plain skeleton={<WidgetSkeleton lines={3} />}>
        {({ items, basis }) => {
          if (items.length === 0) return <Invite>{t('publications.none')}</Invite>

          const shown = items.slice(0, ROWS[size])
          const more = items.length - shown.length
          return (
            <div className="flex min-w-0 flex-col gap-2">
              <ul className="-mx-1 flex flex-col divide-y divide-line">
                {shown.map((publication) => (
                  <PublicationRow key={publication.work_id} publication={publication} />
                ))}
              </ul>
              {more > 0 && (
                <span className="text-xs text-faint">{t('overview.more', { count: more })}</span>
              )}
              <div className="flex flex-col gap-0.5 border-t border-line pt-2 text-sm">
                <StandsOn basis={basis} />
                <CommentsLine workId={work.id} items={items} />
              </div>
            </div>
          )
        }}
      </Loaded>
    </Widget>
  )
}

/**
 * The "Make…" menu: each kind that can be made from the work, with where it
 * goes out underneath. One button whether the list is empty or not - the
 * invitation of an empty widget is the same gesture as the button beside a
 * full one.
 */
function MakeMenu({ make }: { make: MakePublication }) {
  const { t } = useTranslation()
  if (make.kinds.length === 0) return null

  return (
    <Menu>
      <MenuTrigger render={<Button variant="primary" size="xs" disabled={make.making !== null} />}>
        {t('publications.makeMenu')}
        <ChevronDown aria-hidden className="size-3 opacity-70" />
      </MenuTrigger>
      <MenuPopup align="end" size="lg">
        {make.kinds.map((kind) => (
          <MenuItem
            key={kind.key}
            onClick={() => make.make(kind.key)}
            className="flex-col items-start gap-0"
          >
            <span className="text-text">{kind.label}</span>
            {kind.description !== '' && (
              <span className="text-xs text-faint">{kind.description}</span>
            )}
          </MenuItem>
        ))}
      </MenuPopup>
    </Menu>
  )
}

/** A work's cover at the size of a row, as the Links tab draws it: what tells
 *  two rows apart before their titles are read. */
function Thumb({ workId }: { workId: string }) {
  const cover = useCovers().get(workId)
  return (
    <span
      aria-hidden
      className="size-7 shrink-0 rounded-sm"
      style={{ background: coverImageFor(workId, cover) }}
    />
  )
}

/**
 * One publication: its cover, its title, and under it where it goes out, the
 * comments under it and - for a short cut from the clip rather than from the
 * song - what it was cut from; at the end its kind and the fact of it with
 * its day. The whole row opens it.
 */
function PublicationRow({ publication }: { publication: Publication }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const vocabulary = vocabularyOf(profile.config, publication.kind)
  const fact = publicationFact(publication, today())
  const status = vocabulary.statuses.find((known) => known.key === publication.status)

  const about = [
    doorsOf(vocabulary.release_kinds, sayLabel),
    publication.comments > 0 ? t('publications.commentCount', { count: publication.comments }) : '',
    publication.depth > 1 && publication.via !== null
      ? t('publications.via', { title: publication.via })
      : '',
  ]
    .filter((part) => part !== '')
    .join(' · ')

  return (
    <li>
      <RowButton
        render={<Link to={`/works/${publication.work_id}`} />}
        start={<Thumb workId={publication.work_id} />}
        description={about === '' ? undefined : about}
        className="px-1"
        end={
          <>
            <Chip>{labelOf(profile.config.work_kinds, publication.kind)}</Chip>
            {fact.said === 'status' ? (
              <Chip variant={badgeVariantOf(status?.colour)}>
                {status === undefined ? publication.status : sayLabel(status.label)}
              </Chip>
            ) : (
              <Chip variant={FACT_TONE[fact.said]}>
                {t(`publications.${fact.said}`, { day: formatDay(fact.day) })}
              </Chip>
            )}
          </>
        }
      >
        {publication.title}
      </RowButton>
    </li>
  )
}

/** What the song's status stands on: the publication that went out, or the
 *  one booked soonest - or that nothing has, yet. */
function StandsOn({ basis }: { basis: PublicationBasis | null }) {
  const { t } = useTranslation()
  if (basis === null) return <p className="text-dim">{t('publications.nothingOut')}</p>
  const said = { title: basis.title, day: formatDay(basis.day) }
  return (
    <p className="text-dim">
      {t(basis.released ? 'publications.outAs' : 'publications.bookedAs', said)}
    </p>
  )
}

/**
 * The comments under every publication, summed, and the way to the song's
 * Comments tab, which lists them. A reply is written under the publication
 * the comment was left on; this line only says how many there are, and how
 * many of those wait.
 */
function CommentsLine({ workId, items }: { workId: string; items: readonly Publication[] }) {
  const { t } = useTranslation()
  const comments = items.reduce((sum, item) => sum + item.comments, 0)
  const waiting = items.reduce((sum, item) => sum + item.comments_waiting, 0)

  if (comments === 0) return <p className="text-faint">{t('publications.noComments')}</p>
  return (
    <p>
      <Link to={`/works/${workId}/comments`} className="text-dim no-underline hover:underline">
        {t('publications.commentsUnder', { count: comments })}
        {waiting > 0 && ` · ${t('publications.commentsWaiting', { count: waiting })}`}
      </Link>
    </p>
  )
}
