import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import type { Work } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Chip } from '@/components/ui/chip'
import { Loaded } from '@/components/Loaded'
import {
  Invite,
  InviteLink,
  useLook,
  Widget,
  WidgetSkeleton,
} from '@/features/work/tabs/overview/Widget'

/** How many works a group lists before it says how many more there are. */
const ROWS = { s: 3, m: 5, l: 5 } as const

/**
 * What the work was made from and what was made from it: the song behind a
 * clip, the clips of a song. Each is the way to that work; the Links tab is
 * where a link is made or taken away.
 */
export function LinksWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const { size } = useLook()
  const links = useQuery(queries.links(work.id))

  const to = `/works/${work.id}/links`
  const empty =
    links.data !== undefined && links.data.sources.length + links.data.derived.length === 0

  return (
    <Widget
      caption={t('card.tab.links')}
      to={to}
      tone={empty ? 'empty' : 'plain'}
      actions={empty ? <InviteLink to={to}>{t('overview.linkOne')}</InviteLink> : undefined}
    >
      <Loaded query={links} plain skeleton={<WidgetSkeleton />}>
        {({ sources, derived }) => {
          if (sources.length + derived.length === 0) {
            return <Invite>{t('overview.noLinks')}</Invite>
          }
          const limit = ROWS[size]
          return (
            <div className="flex min-w-0 flex-col gap-1.5">
              {sources.length > 0 && (
                <Group caption={t('links.sources')} more={sources.length - limit}>
                  {sources.slice(0, limit).map((link) => (
                    <WorkLine
                      key={link.id}
                      id={link.source_id}
                      title={link.source_title}
                      kind={link.source_kind}
                      // The source moved on since it was taken: the fact, for
                      // the Links tab to show what changed.
                      note={
                        link.drifted ? (
                          <Chip variant="warn">{t('overview.changedSince')}</Chip>
                        ) : undefined
                      }
                    />
                  ))}
                </Group>
              )}
              {derived.length > 0 && (
                <Group caption={t('links.derived')} more={derived.length - limit}>
                  {derived.slice(0, limit).map((made) => (
                    <WorkLine
                      key={made.link_id}
                      id={made.work_id}
                      title={made.title}
                      kind={made.kind}
                    />
                  ))}
                </Group>
              )}
            </div>
          )
        }}
      </Loaded>
    </Widget>
  )
}

function Group({
  caption,
  more,
  children,
}: {
  caption: string
  more: number
  children: ReactNode
}) {
  const { t } = useTranslation()
  return (
    <section className="flex min-w-0 flex-col">
      <h3 className="text-xs text-faint">{caption}</h3>
      <ul className="flex flex-col">{children}</ul>
      {more > 0 && (
        <span className="text-xs text-faint">{t('overview.more', { count: more })}</span>
      )}
    </section>
  )
}

/** One work at the other end of a link: what kind it is, and the way to it. */
function WorkLine({
  id,
  title,
  kind,
  note,
}: {
  id: string
  title: string
  kind: string
  note?: ReactNode
}) {
  const profile = useProfile()
  return (
    <li className="flex min-w-0 items-center gap-2 border-b border-line py-1 text-sm last:border-b-0">
      <Chip>{labelOf(profile.config.work_kinds, kind)}</Chip>
      <Link
        to={`/works/${id}/overview`}
        title={title}
        className="min-w-0 truncate text-text no-underline hover:underline"
      >
        {title}
      </Link>
      {note !== undefined && <span className="ml-auto shrink-0">{note}</span>}
    </li>
  )
}
