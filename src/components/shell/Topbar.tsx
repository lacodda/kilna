import { useEffect, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Bell, Search } from 'lucide-react'
import { getWork, unreadJournal } from '@/lib/api'
import { keys } from '@/lib/query'
import { openWorkId } from '@/lib/route'
import { Button } from '@/components/ui/Button'
import { CommandPalette } from '@/components/CommandPalette'
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
 * The bell, lit only by things that ask to be looked at.
 *
 * Ordinary edits are recorded but never counted: a bell that is always lit is a
 * bell nobody reads. Only warnings — a release that lost its slot — light it.
 */
function Unread() {
  const { t } = useTranslation()
  const navigate = useNavigate()
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

  return (
    <Button
      variant="icon"
      size="icon-sm"
      className="relative"
      title={t('journal.open')}
      aria-label={t('journal.open')}
      onClick={() => navigate('/journal')}
    >
      <Bell aria-hidden />
      {count > 0 && (
        <span
          className={cn(
            'absolute -right-0.5 -top-0.5 min-w-3.5 rounded-full bg-warn px-1',
            'text-center font-mono text-[9px] leading-[14px] text-on-warn',
          )}
        >
          {count > 9 ? '9+' : count}
        </span>
      )}
    </Button>
  )
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
  // Read from the path rather than `useParams`: the topbar is a sibling of
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
    return <span className="text-[13px] font-semibold">{screen}</span>
  }

  return (
    <nav className="flex min-w-0 items-center gap-1.5 text-[13px]">
      <Link to="/catalogue" className="text-dim transition-colors hover:text-text">
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

export function Topbar({ works }: Props) {
  const { t } = useTranslation()
  const [searching, setSearching] = useState(false)

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
    <header className="flex items-center gap-2.5 border-b border-line px-4">
      <Breadcrumbs />

      <button
        type="button"
        onClick={() => setSearching(true)}
        className="ml-auto flex min-w-52 cursor-pointer items-center gap-2 rounded-[10px] border border-line px-2.5 py-1 text-xs text-faint transition-colors hover:border-line-2 hover:text-dim"
      >
        <Search aria-hidden className="size-3.5" />
        {t('search.placeholder')}
        <kbd className="ml-auto rounded border border-line px-1.5 font-mono text-[10px]">
          {t('search.shortcut')}
        </kbd>
      </button>

      <p className="text-xs text-faint">
        {t('status.works')} {works}
      </p>
      <Unread />

      <CommandPalette open={searching} onOpenChange={setSearching} />
    </header>
  )
}
