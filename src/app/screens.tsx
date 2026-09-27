import type { ReactNode } from 'react'
import {
  Calendar,
  Disc,
  FileText,
  History,
  LayoutDashboard,
  List,
  MessagesSquare,
  Palette,
  Settings,
  Shapes,
  Trash2,
  type LucideIcon,
} from 'lucide-react'
import type { ProfileConfig } from '@/lib/api/types'
import { styleTypesOf } from '@/lib/useProfile'
import { Catalogue } from '@/features/catalogue/Catalogue'
import { CalendarView } from '@/features/calendar/CalendarView'
import { CommentsView } from '@/features/comments/CommentsView'
import { DashboardView } from '@/features/dashboard/DashboardView'
import { JournalView } from '@/features/journal/JournalView'
import { NotesView } from '@/features/notes/NotesView'
import { SettingsView } from '@/features/settings/SettingsView'
import { Styleguide } from '@/features/styleguide/Styleguide'
import { StylesView } from '@/features/styles/StylesView'
import { TrashView } from '@/features/trash/TrashView'
import { WorksScreen } from '@/features/work/WorksScreen'

/** Open a work, on a tab when one is named. */
export type OpenWork = (workId: string, tab?: string) => void

export interface ScreenSpec {
  /** The first segment of its address. */
  key: string
  /** The route, as the router reads it. */
  path: string
  /** The word that names it: in the rail, the title bar and the shortcut sheet. */
  nav: string
  /**
   * Held against the window's height - something inside scrolls - or flowing,
   * as long as its content and scrolling itself. See `Screen`.
   */
  scroll: 'flow' | 'held'
  /** Where the rail shows it; absent for a screen reached only from another. */
  rail?: {
    group: 'work' | 'library' | 'foot'
    icon: LucideIcon
    /** Only where the craft has it. */
    when?: (config: ProfileConfig) => boolean
  }
  /**
   * The letter after `g` that jumps here. The initials of the English names,
   * which is what makes them memorable - and they stay put when the interface
   * is read in another language, because a shortcut that moved with the
   * translation would have to be relearned per language.
   */
  jump?: string
  /** Development builds only. */
  dev?: boolean
  render: (open: OpenWork) => ReactNode
}

/**
 * Every screen of the window, in the order the rail shows them.
 *
 * One list, read by the router, the rail, the title bar and the keyboard.
 * Until v0.77 each of the four kept its own, and a screen added to one and
 * forgotten in another was named "Catalogue" in the title bar (styles, notes
 * and comments, in turn) or reached by no shortcut the sheet admitted to.
 */
export const SCREENS: readonly ScreenSpec[] = [
  {
    // Where the app opens: the first question is what needs deciding, not
    // what exists.
    key: 'dashboard',
    path: '/dashboard',
    nav: 'nav.dashboard',
    scroll: 'flow',
    rail: { group: 'work', icon: LayoutDashboard },
    jump: 'd',
    render: (open) => <DashboardView onSelect={open} />,
  },
  {
    // The list of works. Held rather than growing with its rows: its table
    // scrolls both ways inside, so the sideways bar stays at the bottom of the
    // window. There is no separate Works entry: a second door to the same
    // things only made you choose between them.
    key: 'catalogue',
    path: '/catalogue',
    nav: 'nav.catalogue',
    scroll: 'held',
    rail: { group: 'work', icon: List },
    jump: 'c',
    render: (open) => <Catalogue onSelect={open} />,
  },
  {
    // An open work belongs to the catalogue, where its trail and back link
    // lead. Held: the card lays itself out against the window's height - its
    // header stands still and the open tab scrolls inside itself. The open
    // tab is part of the address, so the back button walks between tabs and
    // a tab can be linked to directly.
    key: 'works',
    path: '/works/:workId?/:tab?',
    nav: 'nav.catalogue',
    scroll: 'held',
    render: () => <WorksScreen />,
  },
  {
    // Held like the catalogue: the queue scrolls inside its own column and
    // the month it is being read against stays on screen.
    key: 'calendar',
    path: '/calendar',
    nav: 'nav.calendar',
    scroll: 'held',
    rail: { group: 'work', icon: Calendar },
    jump: 'k',
    render: (open) => <CalendarView onSelect={open} />,
  },
  {
    // The list and the open note each scroll inside their own column, and
    // the note's tags stay on the window's bottom edge. The open note is in
    // the address, so back walks between notes.
    key: 'notes',
    path: '/notes/:noteId?',
    nav: 'nav.notes',
    scroll: 'held',
    rail: { group: 'library', icon: FileText },
    jump: 'n',
    render: () => <NotesView />,
  },
  {
    // The inbox of the audience's comments, held the same way: the list and
    // the open comment scroll apart.
    key: 'comments',
    path: '/comments/:commentId?',
    nav: 'nav.comments',
    scroll: 'held',
    rail: { group: 'library', icon: MessagesSquare },
    render: () => <CommentsView />,
  },
  {
    // Only where the craft has a dictionary: a profile that names no style
    // types has none, and a door to an empty room is worse than none.
    key: 'styles',
    path: '/styles',
    nav: 'nav.styles',
    scroll: 'flow',
    rail: { group: 'library', icon: Shapes, when: (config) => styleTypesOf(config).length > 0 },
    render: () => <StylesView />,
  },
  {
    key: 'journal',
    path: '/journal',
    nav: 'nav.journal',
    scroll: 'flow',
    rail: { group: 'library', icon: History },
    jump: 'j',
    render: () => <JournalView />,
  },
  {
    key: 'trash',
    path: '/trash',
    nav: 'nav.trash',
    scroll: 'flow',
    rail: { group: 'library', icon: Trash2 },
    jump: 't',
    render: () => <TrashView />,
  },
  {
    // The living inventory of the design system, in development builds.
    key: 'styleguide',
    path: '/styleguide',
    nav: 'nav.styleguide',
    scroll: 'flow',
    rail: { group: 'foot', icon: Palette },
    dev: true,
    render: () => <Styleguide />,
  },
  {
    // The section is part of the address, like a card's tab: the rail's
    // Settings link lands on the first one. Held: only the section scrolls,
    // and the list of sections stands.
    key: 'settings',
    path: '/settings/:section?',
    nav: 'nav.data',
    scroll: 'held',
    rail: { group: 'foot', icon: Settings },
    jump: 's',
    render: () => <SettingsView />,
  },
]

/** A door in the rail to a screen still to come, with the version bringing it. */
export interface SoonSpec {
  nav: string
  icon: LucideIcon
  version: string
}

/** Screens promised by the plan, drawn as dimmed doors in the rail. */
export const SOON: readonly SoonSpec[] = [{ nav: 'nav.collections', icon: Disc, version: '0.91' }]

/** The screens this build draws: development ones only in development. */
export const drawn = (dev: boolean): readonly ScreenSpec[] =>
  SCREENS.filter((screen) => dev || screen.dev !== true)

/** Where `g` then a letter goes. */
export const JUMPS: Readonly<Record<string, string>> = Object.freeze(
  Object.fromEntries(
    SCREENS.filter((screen) => screen.jump !== undefined).map((screen) => [
      screen.jump!,
      `/${screen.key}`,
    ]),
  ),
)

/**
 * The screen at `pathname`, by the first segment of the address. An address
 * no screen has is sent to the dashboard by the router, so that is what it is.
 */
export function screenAt(pathname: string): ScreenSpec {
  const segment = pathname.split('/')[1] ?? ''
  return SCREENS.find((screen) => screen.key === segment) ?? SCREENS[0]!
}
