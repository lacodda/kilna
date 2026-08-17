import { useTranslation } from 'react-i18next'
import { NavLink } from 'react-router'
import { TABS, type Tab } from '@/components/card/tabs'
import { cn } from '@/lib/utils'

interface Props {
  workId: string
  /** Shown beside the Releases tab; omitted when there are none. */
  releases?: number
}

/**
 * The card's tabs, as links.
 *
 * Each tab is its own URL, so the back button walks between them and a tab can
 * be linked to from a note, a journal entry or a chat — the same reason the open
 * work became `/works/:id` in v0.11.
 */
export function TabBar({ workId, releases = 0 }: Props) {
  const { t } = useTranslation()

  return (
    <nav className="flex gap-0.5 overflow-x-auto border-t border-line bg-raise px-2">
      {TABS.map((tab) => (
        <NavLink
          key={tab}
          to={`/works/${workId}/${tab}`}
          className={({ isActive }) =>
            cn(
              '-mb-px flex shrink-0 items-center gap-1.5 border-b-2 border-transparent px-[13px] py-[9px] text-[13px] text-dim transition-colors hover:text-text',
              isActive && 'border-accent font-semibold text-text',
            )
          }
        >
          {t(`card.tab.${tab satisfies Tab}`)}
          {tab === 'releases' && releases > 0 && (
            <span className="rounded-full border border-line px-1.5 text-[11px] text-faint">
              {releases}
            </span>
          )}
        </NavLink>
      ))}
    </nav>
  )
}
