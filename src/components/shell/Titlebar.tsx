import { useEffect, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { FileText, Plus, Search } from 'lucide-react'
import { getWork, listJournal, markJournalRead, unreadJournal, type JournalEntry } from '@/lib/api'
import { keys } from '@/lib/query'
import { openWorkId } from '@/lib/route'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { NotificationBell } from '@/components/ui/notification-bell'
import { CommandPalette } from '@/components/CommandPalette'
import { sentence, when } from '@/components/JournalFeed'
import { NewWorkDialog } from '@/components/shell/NewWorkDialog'
import { Mark } from '@/components/shell/Mark'
import { WindowButtons, useTitleBarGestures } from '@/components/ui/window-frame'
import { cn } from '@/lib/utils'

interface Props {
  works: number
}

// Maps the first path segment to the nav key that names the screen.
function screenKey(pathname: string): string {
  const segment = pathname.split('/')[1]
  switch (segment) {
    case 'dashboard':
      return 'nav.dashboard'
    case 'catalogue':
      return 'nav.catalogue'
    case 'calendar':
      return 'nav.calendar'
    case 'journal':
      return 'nav.journal'
    case 'trash':
      return 'nav.trash'
    case 'settings':
      return 'nav.data'
    case 'styleguide':
      return 'nav.styleguide'
    // Includes `/works/:id`: an open work belongs to the catalogue, which is
    // where its trail and its back link lead.
    default:
      return 'nav.catalogue'
  }
}

/**
 * Where you are, as a trail rather than a word.
 *
 * On an open card the screen's own name is not enough: "Works" says nothing
 * about which work, and the way back to the list is otherwise the browser's
 * back button alone. The first part is a link; the last is where you stand.
 */
function Breadcrumbs() {
  const { t } = useTranslation()
  const location = useLocation()
  // Read from the path rather than `useParams`: the title bar is a sibling of
  // `<Routes>`, not a descendant, so it matches no route and would always see
  // an empty params object.
  const workId = openWorkId(location.pathname)

  const work = useQuery({
    queryKey: keys.work(workId ?? ''),
    queryFn: () => getWork(workId ?? ''),
    enabled: workId !== undefined,
  })

  const screen = t(screenKey(location.pathname))
  if (workId === undefined) {
    return <span className="truncate text-[13px] font-semibold">{screen}</span>
  }

  return (
    <nav className="flex min-w-0 items-center gap-1.5 text-[13px]">
      <Link to="/catalogue" className="shrink-0 text-dim transition-colors hover:text-text">
        {screen}
      </Link>
      <span aria-hidden className="text-faint">
        ›
      </span>
      {/* Nothing while the title loads, rather than a placeholder that is
          replaced a moment later — the trail would jump under the cursor. */}
      <span className="truncate font-semibold">{work.data?.title ?? ''}</span>
    </nav>
  )
}

/** The mark and the name, where a system title bar would print them. */
function Brand() {
  const { t } = useTranslation()
  return (
    <div className="flex shrink-0 items-center gap-2 pr-3">
      <Mark className="size-[18px]" />
      <b className="text-[13px] font-semibold tracking-[0.02em]">{t('app.name')}</b>
      <small className="font-mono text-[10px] text-faint">{__APP_VERSION__}</small>
    </div>
  )
}

/** How many entries the bell shows before sending you to the whole history. */
const RECENT = 6

/**
 * The bell, lit only by things that ask to be looked at.
 *
 * Ordinary edits are recorded but never counted: a bell that is always lit is a
 * bell nobody reads. Only warnings — a release that lost its slot — light it.
 * Pressing it no longer leaves the screen: the last few entries open under it,
 * the ones that need a look first, and the whole history is one more click.
 */
function Unread() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const [open, setOpen] = useState(false)

  const unread = useQuery({
    queryKey: keys.journalUnread,
    queryFn: unreadJournal,
    // The bell sits outside the routes and never remounts, so without this it
    // would only ever change when a mutation in this window invalidated it —
    // and an entry written by a background task, a plugin or a second window
    // would leave it reading zero for as long as the app stayed open.
    refetchInterval: 60_000,
  })
  const count = unread.data ?? 0

  // Fetched only while the panel is open: the feed is one page of up to two
  // hundred entries, and the bell is pressed far less often than it is seen.
  const entries = useQuery({
    queryKey: keys.journalFeed,
    queryFn: listJournal,
    enabled: open,
  })

  const markRead = useMutation({
    mutationFn: markJournalRead,
    onSuccess: () => void client.invalidateQueries({ queryKey: keys.journal }),
    onError: (cause) => say.failedTo(t('toast.loadFailed'), cause),
  })

  // What needs a look comes first, then the newest of the rest, and the two
  // together fill the short list - so an unread warning from last week is not
  // pushed out by six edits made this morning.
  const recent = (() => {
    const all = entries.data ?? []
    const warned = all.filter((entry) => entry.level === 'warn' && entry.read_at === null)
    const rest = all.filter((entry) => !warned.includes(entry))
    return [...warned, ...rest].slice(0, RECENT)
  })()

  return (
    <NotificationBell
      count={count}
      label={t('journal.open')}
      title={t('journal.recent')}
      open={open}
      onOpenChange={setOpen}
      markAllLabel={t('journal.markRead')}
      onMarkAll={() => markRead.mutate()}
      busy={markRead.isPending}
      seeAllLabel={t('journal.seeAll')}
      onSeeAll={() => navigate('/journal')}
      emptyLabel={t('journal.nothingRecent')}
    >
      {/* Rows only once they have arrived: until then the bell shows its
          empty line, which is truer than a list of nothing. */}
      {entries.isSuccess && recent.length > 0 && (
        <ul>
          {recent.map((entry) => (
            <RecentLine key={entry.id} entry={entry} />
          ))}
        </ul>
      )}
    </NotificationBell>
  )
}

function RecentLine({ entry }: { entry: JournalEntry }) {
  const { t, i18n } = useTranslation()
  const needsALook = entry.level === 'warn' && entry.read_at === null
  return (
    <li className="flex items-baseline gap-2.5 border-b border-line py-2 last:border-b-0">
      <span
        aria-hidden
        className={cn(
          'mt-1.5 size-1.5 shrink-0 rounded-full',
          needsALook ? 'bg-warn' : 'bg-line-2',
        )}
      />
      <p className={cn('min-w-0 flex-1 text-[13px] text-dim', needsALook && 'text-text')}>
        {sentence(entry, t)}
      </p>
      <time dateTime={entry.created_at} className="shrink-0 text-[11px] tabular-nums text-faint">
        {when(entry.created_at, i18n.language)}
      </time>
    </li>
  )
}

/**
 * The New button: an icon beside the bell, and the dialog behind it.
 */
function NewWork({ onCreated }: { onCreated: (workId: string) => void }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [kind, setKind] = useState<string | null>(null)
  const kinds = profile.config.work_kinds

  return (
    <>
      {/* One press, straight to the dialog.

          This was a labelled button with a dropdown of the kinds, and it asked
          the question twice: the menu named Song, Instrumental, Video, Short -
          and then the dialog opened with a Kind field offering the same four,
          because a kind pressed by mistake must not cost the dialog. So the
          menu only delayed the box where the title is typed, which is the
          thing actually being added. The dialog opens on the first kind and
          the field inside changes it.

          A page with a plus over it, the way the bell beside it carries its
          count: the shape says what is made, the plus says a new one. */}
      <Button
        variant="icon"
        size="icon-sm"
        className="relative"
        title={t('shell.new')}
        aria-label={t('shell.new')}
        onClick={() => setKind(kinds[0]?.key ?? null)}
      >
        <FileText aria-hidden />
        <Plus
          aria-hidden
          className="absolute -right-0.5 -top-0.5 size-2.5 rounded-full bg-raise text-accent"
        />
      </Button>
      <NewWorkDialog kind={kind} onClose={() => setKind(null)} onCreated={onCreated} />
    </>
  )
}

/**
 * The window's title bar, and the application's.
 *
 * There is one bar rather than a system one over an application one: the
 * mark, the trail, the search in the middle, the count, the New button and
 * the bell, then the window's own buttons - the strip scheda draws, with this
 * application's things in it. Everything not a control is a handle to drag
 * the window by.
 */
export function Titlebar({ works }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [searching, setSearching] = useState(false)
  const gestures = useTitleBarGestures()

  // Ctrl+K anywhere, including from inside a text field: the palette is a way
  // out of wherever you are, not a control that belongs to one screen.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key.toLowerCase() === 'k' && (event.ctrlKey || event.metaKey)) {
        event.preventDefault()
        setSearching(true)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  return (
    <header
      className="grid h-full grid-cols-[minmax(0,1fr)_minmax(16rem,22rem)_minmax(0,1fr)] items-center border-b border-line bg-bg"
      {...gestures}
    >
      <div className="flex min-w-0 items-center gap-2 pl-3">
        <Brand />
        <Breadcrumbs />
      </div>

      <button
        type="button"
        onClick={() => setSearching(true)}
        className="flex h-7 min-w-0 cursor-pointer items-center gap-2 rounded-[10px] border border-line bg-raise px-2.5 text-xs text-faint transition-colors hover:border-line-2 hover:text-dim"
      >
        <Search aria-hidden className="size-3.5 shrink-0" />
        <span className="truncate">{t('search.placeholder')}</span>
        <kbd className="ml-auto rounded border border-line px-1.5 font-mono text-[10px]">
          {t('search.shortcut')}
        </kbd>
      </button>

      <div className="flex h-full min-w-0 items-center justify-end gap-2">
        <p className="hidden text-xs whitespace-nowrap text-faint lg:block">
          {t('status.works')} {works}
        </p>
        <NewWork onCreated={(workId) => navigate(`/works/${workId}`)} />
        <Unread />
        <span aria-hidden className="ml-1 h-4 w-px bg-line" />
        <WindowButtons
          labels={{
            minimize: t('shell.minimize'),
            maximize: t('shell.maximize'),
            restore: t('shell.restore'),
            close: t('shell.close'),
          }}
        />
      </div>

      <CommandPalette open={searching} onOpenChange={setSearching} />
    </header>
  )
}
