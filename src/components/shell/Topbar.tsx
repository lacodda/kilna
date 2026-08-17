import { useLocation, useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Bell } from 'lucide-react'
import { unreadJournal } from '@/lib/api'
import { keys } from '@/lib/query'
import { Button } from '@/components/ui/Button'
import { cn } from '@/lib/utils'

interface Props {
  works: number
}

// Maps the first path segment to the nav key that names the screen.
function screenKey(pathname: string): string {
  const segment = pathname.split('/')[1]
  switch (segment) {
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
    default:
      return 'nav.works'
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
      size="iconSm"
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
            'text-center font-mono text-[9px] leading-[14px] text-on-accent',
          )}
        >
          {count > 9 ? '9+' : count}
        </span>
      )}
    </Button>
  )
}

export function Topbar({ works }: Props) {
  const { t } = useTranslation()
  const location = useLocation()

  return (
    <header className="flex items-center gap-2.5 border-b border-line px-4">
      <span className="text-[13px] font-semibold">{t(screenKey(location.pathname))}</span>
      <p className="ml-auto text-xs text-faint">
        {t('status.works')} {works}
      </p>
      <Unread />
    </header>
  )
}
