import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { NavLink, useLocation } from 'react-router'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import type { Tab, TabCount } from '@/features/work/tabs'
import { useEdges } from '@/lib/overflow'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

interface Props {
  workId: string
  /** The tabs this work draws, in order (`tabsOf`). */
  tabs: readonly Tab[]
  /** The number beside each tab that has something in it (`tabCounts`). */
  counts?: Partial<Record<Tab, TabCount>>
}

/**
 * The card's tabs, as links.
 *
 * Each tab is its own URL, so the back button walks between them and a tab can
 * be linked to from a note, a journal entry or a chat — the same reason the open
 * work became `/works/:id` in v0.11. That is also why this is not the design
 * system's Tabs: those switch panels inside one page, and a tab here is an
 * address.
 *
 * Twelve tabs do not fit every window, so the strip scrolls sideways - without
 * a scrollbar, which a strip this shallow cannot spare the height for - and
 * says so at the end that goes on: a fade over the last tab in view and a
 * chevron that scrolls by most of a strip. Until v0.80 the tabs past the edge
 * simply were not there for anyone who did not know to scroll.
 */
export function TabBar({ workId, tabs, counts = {} }: Props) {
  const { t } = useTranslation()
  const strip = useRef<HTMLElement>(null)
  const row = useRef<HTMLDivElement>(null)
  const edges = useEdges(strip, row)
  const { pathname } = useLocation()

  // The open tab is brought into view when the address changes: a card
  // opened on its History from a journal line would otherwise show a strip
  // with nothing lit, the lit tab being past the edge. Scrolled by hand
  // rather than with `scrollIntoView`, which also scrolls every clipping box
  // around the strip that can be, and clear of the fade at either end.
  useEffect(() => {
    const element = strip.current
    if (element === null) return
    const open = element.querySelector<HTMLElement>('[aria-current="page"]')
    if (open === null) return
    const bounds = element.getBoundingClientRect()
    const tab = open.getBoundingClientRect()
    if (tab.left < bounds.left) element.scrollLeft -= bounds.left - tab.left + FADE
    else if (tab.right > bounds.right) element.scrollLeft += tab.right - bounds.right + FADE
  }, [pathname])

  const page = (direction: 1 | -1) => {
    const element = strip.current
    if (element === null) return
    element.scrollTo({
      left: element.scrollLeft + direction * element.clientWidth * 0.8,
      behavior: 'smooth',
    })
  }

  return (
    <div className="relative flex min-w-0">
      {/* The vertical axis is clipped explicitly: `overflow-x-auto` alone
          leaves it at `auto`, and a strip one pixel taller than its box was
          enough for WebView2 to reserve a vertical scrollbar down the side of
          the tabs. */}
      <nav
        ref={strip}
        aria-label={t('card.tabs')}
        className="min-w-0 flex-1 overflow-x-auto overflow-y-hidden [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
        // A mouse wheel turns a strip that only scrolls sideways: nothing
        // around it scrolls down, so there is no other scroll to take from.
        onWheel={(event) => {
          if (Math.abs(event.deltaY) > Math.abs(event.deltaX)) {
            event.currentTarget.scrollLeft += event.deltaY
          }
        }}
      >
        {/* `w-max`: the row is as wide as its tabs, so the strip measures
            what it holds rather than what it shows. */}
        <div ref={row} className="flex w-max min-w-full gap-px px-2 pt-1.75">
          {tabs.map((tab) => {
            const count = counts[tab]
            return (
              <NavLink
                key={tab}
                to={`/works/${workId}/${tab}`}
                className={({ isActive }) =>
                  cn(
                    'flex shrink-0 items-center gap-1.5 rounded-t-sm border-b-2 border-transparent px-2.5 py-1.75 text-sm whitespace-nowrap text-dim no-underline transition-colors hover:text-text',
                    'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent',
                    isActive && 'border-accent font-semibold text-text',
                  )
                }
              >
                {t(`card.tab.${tab satisfies Tab}`)}
                {count !== undefined && (
                  <span
                    title={
                      count.waiting ? t('comments.waitingCount', { count: count.value }) : undefined
                    }
                    className={cn(
                      'rounded-full border px-1.25 font-mono text-2xs tabular-nums',
                      count.waiting
                        ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                        : 'border-line text-faint',
                    )}
                  >
                    {count.value}
                  </span>
                )}
              </NavLink>
            )
          })}
        </div>
      </nav>

      {edges.start && <Edge side="start" label={t('card.tabsBack')} onPage={() => page(-1)} />}
      {edges.end && <Edge side="end" label={t('card.tabsMore')} onPage={() => page(1)} />}
    </div>
  )
}

/** How far the fade at an end reaches into the strip, in pixels: `w-12`. */
const FADE = 48

/**
 * The sign that the strip goes on past one of its ends: a fade over the tabs
 * cut by the edge, and a chevron that scrolls toward them.
 *
 * The chevron is for the pointer. The keyboard walks the tabs themselves,
 * and the strip scrolls each into view as it takes focus - so the chevron is
 * kept out of the tab order and the reading, the way the design system keeps
 * a tab's close cross.
 */
function Edge({
  side,
  label,
  onPage,
}: {
  side: 'start' | 'end'
  label: string
  onPage: () => void
}) {
  const Chevron = side === 'start' ? ChevronLeft : ChevronRight
  return (
    <div
      className={cn(
        'pointer-events-none absolute inset-y-0 flex w-12 items-center',
        side === 'start' ? 'left-0 justify-start' : 'right-0 justify-end',
      )}
      // The header's own ground, solid under the chevron and thinning over
      // the tab it cuts. Written out rather than as gradient utilities, whose
      // `bg-linear-*` the token check reads as a colour the theme lacks.
      style={{
        background: `linear-gradient(to ${side === 'start' ? 'right' : 'left'}, var(--raise) 45%, transparent)`,
      }}
    >
      <Button
        variant="icon"
        size="icon-xs"
        tabIndex={-1}
        aria-hidden
        title={label}
        onClick={onPage}
        className="pointer-events-auto mt-1.75"
      >
        <Chevron aria-hidden />
      </Button>
    </div>
  )
}
