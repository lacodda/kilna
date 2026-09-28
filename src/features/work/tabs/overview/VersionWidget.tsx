import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import type { VersionRole, VersionSummary, Work } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { currentIn, previewOf, previousOf } from '@/lib/overview'
import { queries } from '@/lib/query/queries'
import { say } from '@/lib/toast'
import { say as sayLabel } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { Loaded } from '@/components/Loaded'
import {
  Invite,
  InviteLink,
  useLook,
  Widget,
  WidgetSkeleton,
} from '@/features/work/tabs/overview/Widget'

/** How many lines of the text a widget reads, by how much of the board it takes. */
const LINES = { s: 4, m: 6, l: 14 } as const

interface Props {
  work: Work
  /** The role it reads: the text the work is written in, or its style prompt. */
  role: VersionRole
}

/**
 * The widget catalogue's text: the opening of the version that stands in one
 * role - the lyrics, the plot, the style prompt - with its revision and length,
 * and a copy of the whole of it one click away.
 *
 * The text lives on the Versions tab, and this goes there: clicking the
 * widget, or "Edit", opens that very version (the owner's rule - a click
 * leads to the owning tab, and an editor copied into a widget would give the
 * text two homes and no way to tell where it was last changed). Comparing
 * with the revision before is the one thing that earns a larger surface, and
 * the tab opens with the two side by side.
 */
export function VersionWidget({ work, role }: Props) {
  const { t } = useTranslation()
  const { presentation, size } = useLook()
  const versions = useQuery(queries.versions(work.id))

  const name = sayLabel(role.label)
  const all = versions.data ?? []
  const shown = currentIn(all, role.key)
  const open = `/works/${work.id}/versions`
  const to = shown === undefined ? `${open}?role=${role.key}` : `${open}?version=${shown.id}`
  const empty = versions.data !== undefined && shown === undefined

  // A row says the revision under the text (`Preview`); its caption column
  // is the role's name alone, as the mockup's bands and sheet write it.
  const caption =
    shown === undefined || presentation !== 'card'
      ? name
      : [
          name,
          t('versions.revision', { number: shown.revision }),
          // The length where the text has the room to be read: on a large
          // widget it says how much of it the preview is.
          size === 'l' ? t('versions.length', { count: shown.length }) : null,
        ]
          .filter((part) => part !== null)
          .join(' · ')

  const before = shown === undefined ? undefined : previousOf(all, shown)

  return (
    <Widget
      caption={caption}
      to={to}
      go={t('overview.toVersions')}
      tone={empty ? 'empty' : 'plain'}
      actions={
        empty ? (
          <InviteLink to={to}>{t('overview.write')}</InviteLink>
        ) : shown === undefined ? undefined : (
          <VersionActions
            work={work}
            version={shown}
            before={size === 'l' && presentation === 'card' ? before : undefined}
          />
        )
      }
    >
      <Loaded query={versions} plain skeleton={<WidgetSkeleton lines={3} />}>
        {() =>
          shown === undefined ? (
            <Invite>{t('overview.noText', { role: name })}</Invite>
          ) : (
            <Preview version={shown} lines={LINES[size]} />
          )
        }
      </Loaded>
    </Widget>
  )
}

/**
 * The opening of the text, in the column it is written in.
 *
 * As a card, the first lines as they break, fading out where the widget ends;
 * on a band, one line; on a line of the sheet, three, with the revision and
 * the day under them - the mockup's text in each layout.
 */
function Preview({ version, lines }: { version: VersionSummary; lines: number }) {
  const { t } = useTranslation()
  const { presentation } = useLook()
  const body = useQuery(queries.version(version.id))

  if (body.isPending) return <Skeleton className="h-12 w-full" />
  const text = body.data?.body ?? ''

  if (presentation === 'card') {
    const { text: opening, more } = previewOf(text, lines)
    return (
      <div className="relative min-h-0 flex-1 overflow-hidden">
        {/* Selectable: a line of the text is copied from here as often as the
            whole of it is, and a click that ends a selection does not carry
            the person off to the Versions tab (`Widget`). */}
        <p className="selectable font-mono text-xs leading-relaxed whitespace-pre-wrap text-dim">
          {opening}
        </p>
        {more && (
          // The text goes on past the widget: it thins into the panel rather
          // than stopping on a line that reads as the last one.
          <div
            aria-hidden
            className="pointer-events-none absolute inset-x-0 bottom-0 h-8"
            style={{ background: 'linear-gradient(transparent, var(--raise))' }}
          />
        )}
      </div>
    )
  }

  const oneLine = presentation === 'band'
  const { text: opening } = previewOf(text, oneLine ? 6 : 3)
  return (
    <div className="flex min-w-0 flex-col gap-0.5">
      <p
        className={cn(
          'selectable font-mono text-sm text-dim',
          oneLine ? 'truncate' : 'line-clamp-3 leading-relaxed whitespace-pre-wrap',
        )}
      >
        {oneLine ? opening.split('\n').join(' ') : opening}
      </p>
      <span className="font-mono text-xs text-faint tabular-nums">
        {t('versions.revision', { number: version.revision })} ·{' '}
        {t('versions.length', { count: version.length })} · {formatDay(version.created_at)}
      </span>
    </div>
  )
}

/**
 * What is done to the text from the board: copy the whole of it, open it to
 * edit, and - on the large widget - put it beside the revision it was written
 * from.
 */
function VersionActions({
  work,
  version,
  before,
}: {
  work: Work
  version: VersionSummary
  before: VersionSummary | undefined
}) {
  const { t } = useTranslation()
  const { presentation } = useLook()
  const body = useQuery(queries.version(version.id))
  const text = body.data?.body

  // The word only after the clipboard says yes - the rule from v0.28: saying a
  // copy succeeded when it did not is worse than saying nothing.
  const copy = () => {
    if (text === undefined) return
    navigator.clipboard.writeText(text).then(
      () => say.ok(t('work.copied')),
      (cause: unknown) => say.failedTo(t('work.copyFailed'), cause),
    )
  }

  const open = `/works/${work.id}/versions?version=${version.id}`
  return (
    <>
      <Button size="xs" disabled={text === undefined} onClick={copy}>
        {t('overview.copy')}
      </Button>
      {/* A row already ends in the way to the tab, which is the same place. */}
      {presentation === 'card' && (
        <Button size="xs" render={<Link to={open} />}>
          {t('overview.edit')}
        </Button>
      )}
      {before !== undefined && (
        <Button size="xs" render={<Link to={`${open}&compare=${before.id}`} />}>
          {t('overview.compareWith', { number: before.revision })}
        </Button>
      )}
    </>
  )
}
