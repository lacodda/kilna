import { useEffect, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Check, FileText, PanelLeftClose, PanelLeftOpen, Plus, Search, X } from 'lucide-react'
import {
  dismissProposal,
  getWork,
  listJournal,
  markJournalRead,
  pendingProposals,
  unreadJournal,
  type JournalEntry,
  type PendingProposal,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { openWorkId } from '@/lib/route'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { NotificationBell } from '@/components/ui/notification-bell'
import { CommandPalette } from '@/components/CommandPalette'
import { AssistantButton } from '@/components/assistant/AssistantDrawer'
import { sentence, when } from '@/components/JournalFeed'
import { NewWorkDialog } from '@/components/shell/NewWorkDialog'
import { Mark } from '@/components/shell/Mark'
import { WindowButtons, useTitleBarGestures } from '@/components/ui/window-frame'
import { cn } from '@/lib/utils'

interface Props {
  works: number
  /** Whether the main menu is folded to icons: the handle says which way it
   *  will go. */
  compact: boolean
  onToggleRail: () => void
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
    case 'styles':
      return 'nav.styles'
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

/** The mark and the name, where a system title bar would print them.
 *
 * On a narrow window the name and the version go and the mark stays: the
 * trail beside it is what says where you are, and it needs the room more. */
function Brand() {
  const { t } = useTranslation()
  return (
    <div className="flex shrink-0 items-center gap-2 pr-3 max-[900px]:pr-1">
      <Mark className="size-[18px]" />
      <b className="text-[13px] font-semibold tracking-[0.02em] max-[900px]:hidden">
        {t('app.name')}
      </b>
      <small className="font-mono text-[10px] text-faint max-[900px]:hidden">
        {__APP_VERSION__}
      </small>
    </div>
  )
}

/**
 * The handle that folds the main menu to icons and back.
 *
 * In the title bar rather than at the foot of the menu: it sits above the
 * rail it changes, where the menu button of every desktop application is,
 * and it stays in the same place whichever width the rail has. */
function RailHandle({ compact, onToggle }: { compact: boolean; onToggle: () => void }) {
  const { t } = useTranslation()
  const label = t(compact ? 'shell.expandMenu' : 'shell.collapseMenu')
  const Icon = compact ? PanelLeftOpen : PanelLeftClose
  return (
    <Button
      variant="icon"
      size="icon-sm"
      title={label}
      aria-label={label}
      aria-expanded={!compact}
      onClick={onToggle}
    >
      <Icon aria-hidden />
    </Button>
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

  // Proposals that arrived over MCP and have been neither applied nor turned
  // down. Always fetched, not only while the panel is open: this is what the
  // badge counts, and a count that only appeared once you looked would be no
  // notification at all.
  const proposals = useQuery({
    queryKey: keys.pendingProposals,
    queryFn: pendingProposals,
    refetchInterval: 30_000,
  })
  const waiting = proposals.data ?? []

  const dismiss = useMutation({
    mutationFn: dismissProposal,
    onSuccess: () => void client.invalidateQueries({ queryKey: keys.pendingProposals }),
    onError: (cause) => say.failedTo(t('assistant.dismissFailed'), cause),
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
      // A proposal is counted too: the work is done and waiting on an answer,
      // which is a stronger claim on attention than an unread line about
      // something that already happened.
      count={count + waiting.length}
      label={t('journal.open')}
      title={t('journal.recent')}
      open={open}
      onOpenChange={setOpen}
      // "Mark all seen" clears read marks on the journal and nothing else.
      // It is deliberately NOT offered while only proposals are waiting: a
      // proposal is answered by applying it or refusing it, and one button
      // that made a screenful of them disappear unanswered is the click
      // nobody means to make. Hidden rather than disabled, because a button
      // that does not act on what is on screen should not be on screen.
      markAllLabel={count > 0 ? t('journal.markRead') : undefined}
      onMarkAll={() => markRead.mutate()}
      busy={markRead.isPending}
      seeAllLabel={t('journal.seeAll')}
      onSeeAll={() => navigate('/journal')}
      emptyLabel={t('journal.nothingRecent')}
    >
      {/* What is waiting on an answer, above what has already happened: these
          are the only lines in the panel that ask for something. */}
      {waiting.length > 0 && (
        <section className="border-b border-line py-2">
          <h4 className="pb-1 text-[11px] font-semibold uppercase tracking-wide text-dim">
            {t('assistant.waitingOnYou', { count: waiting.length })}
          </h4>
          <ul>
            {waiting.map((proposal) => (
              <ProposalLine
                key={proposal.message_id}
                proposal={proposal}
                busy={dismiss.isPending}
                onOpen={() => {
                  setOpen(false)
                  navigate(
                    proposal.work_id === null
                      ? '/assistant'
                      : `/works/${proposal.work_id}/assistant`,
                  )
                }}
                onDismiss={() => dismiss.mutate(proposal.message_id)}
              />
            ))}
          </ul>
        </section>
      )}

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

/**
 * One proposal waiting for an answer.
 *
 * The line opens the chat it arrived in rather than applying anything from
 * here: applying is a decision made while looking at what is proposed, and a
 * bell is not where a version gets written. Turning one down does happen
 * here, because refusing needs nothing read that the line does not say.
 */
function ProposalLine({
  proposal,
  busy,
  onOpen,
  onDismiss,
}: {
  proposal: PendingProposal
  busy: boolean
  onOpen: () => void
  onDismiss: () => void
}) {
  const { t, i18n } = useTranslation()
  const what = t(`assistant.proposed.${proposal.kind}`, {
    defaultValue: t('assistant.proposed.other'),
  })

  return (
    <li className="flex items-center gap-1.5 py-1">
      <button
        type="button"
        onClick={onOpen}
        className="min-w-0 flex-1 rounded-sm px-1 py-1 text-left hover:bg-soft"
      >
        <span className="flex items-baseline gap-2">
          <span className="min-w-0 flex-1 truncate text-[13px] text-text">{what}</span>
          <time
            dateTime={proposal.created_at}
            className="shrink-0 text-[11px] tabular-nums text-faint"
          >
            {when(proposal.created_at, i18n.language)}
          </time>
        </span>
        {proposal.chat_title !== null && (
          <span className="block truncate text-[11.5px] text-dim">{proposal.chat_title}</span>
        )}
      </button>
      <Button
        variant="icon"
        size="icon-sm"
        onClick={onOpen}
        disabled={busy}
        title={t('assistant.openToApply')}
        aria-label={t('assistant.openToApply')}
      >
        <Check aria-hidden className="size-4" />
      </Button>
      <Button
        variant="icon"
        size="icon-sm"
        onClick={onDismiss}
        disabled={busy}
        title={t('assistant.dismiss')}
        aria-label={t('assistant.dismiss')}
      >
        <X aria-hidden className="size-3.5" />
      </Button>
    </li>
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
 * menu handle, the mark, the trail, the search in the middle, the count, the
 * New button, the assistant and the bell, then the window's own buttons - the strip scheda draws, with this
 * application's things in it. Everything not a control is a handle to drag
 * the window by.
 */
export function Titlebar({ works, compact, onToggleRail }: Props) {
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
      <div className="flex min-w-0 items-center gap-2 pl-2">
        <RailHandle compact={compact} onToggle={onToggleRail} />
        <Brand />
        <Breadcrumbs />
      </div>

      <button
        type="button"
        onClick={() => setSearching(true)}
        className="flex h-7 min-w-0 cursor-pointer items-center gap-2 rounded-md border border-line bg-raise px-2.5 text-xs text-faint transition-colors hover:border-line-2 hover:text-dim"
      >
        <Search aria-hidden className="size-3.5 shrink-0" />
        <span className="truncate">{t('search.placeholder')}</span>
        <kbd className="ml-auto rounded border border-line px-1.5 font-mono text-[10px]">
          {t('search.shortcut')}
        </kbd>
      </button>

      <div className="flex h-full min-w-0 items-center justify-end gap-2">
        <p className="text-xs whitespace-nowrap text-faint max-[900px]:hidden">
          {t('status.works')} {works}
        </p>
        <NewWork onCreated={(workId) => navigate(`/works/${workId}`)} />
        <AssistantButton />
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
