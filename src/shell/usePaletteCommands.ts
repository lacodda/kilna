import { useLocation, useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import {
  CalendarPlus,
  FilePlus,
  Keyboard,
  Languages,
  MessageSquare,
  Monitor,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  Star,
  Sun,
  Undo2,
  type LucideIcon,
} from 'lucide-react'
import { onRail } from '@/app/screens'
import { LANGUAGES, useLanguage } from '@/lib/language'
import { queries } from '@/lib/query/queries'
import { openWorkId } from '@/lib/route'
import { THEMES, useTheme, type Theme } from '@/lib/theme'
import { useAssistant } from '@/lib/useAssistant'
import { useProfile } from '@/lib/useProfile'
import { useUndo } from '@/lib/useUndo'

/** Something the palette can do, rather than something it found. */
export interface Command {
  /** Unique across the palette's commands; the highlight compares by it. */
  id: string
  label: string
  icon: LucideIcon
  /** The keys that do the same from anywhere, shown at the end of the row -
   *  a palette is also where shortcuts are learned. */
  keys?: string[]
  run: () => void
}

/** What the shell hands the palette: the parts of it that live elsewhere. */
export interface ShellCommands {
  /** Open the New work dialog. */
  newWork: () => void
  /** Open the list of shortcuts. */
  showKeys: () => void
  /** Fold the rail to icons, or unfold it. */
  toggleRail: () => void
  /** Whether the rail is folded now, which is what the toggle is called by. */
  compact: boolean
}

const THEME_ICONS: Record<Theme, LucideIcon> = {
  system: Monitor,
  light: Sun,
  dark: Moon,
}

/**
 * The palette's two lists of its own: what can be done, and where to go.
 *
 * The actions are the shell's, plus two for the work that is open - scoring it
 * and putting it on the calendar are the two things most often opened a card
 * for. The screens are the rail's own list (`app/screens.tsx`, through
 * `onRail`), so the palette never offers a door the rail does not have, each
 * with the `g` chord that reaches it without the palette.
 */
export function usePaletteCommands(shell: ShellCommands): {
  actions: Command[]
  screens: Command[]
} {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const { config } = useProfile()
  const assistant = useAssistant()
  const { undo } = useUndo()
  const { theme, setTheme } = useTheme()
  const { language, setLanguage } = useLanguage()

  // The trail in the title bar has asked for this already, so it is read from
  // the cache rather than fetched a second time.
  const workId = openWorkId(pathname)
  const work = useQuery({ ...queries.work(workId ?? ''), enabled: workId !== undefined })
  const title = work.data?.title

  const onWork: Command[] =
    workId === undefined || title === undefined
      ? []
      : [
          {
            id: 'score',
            label: t('search.action.score', { title }),
            icon: Star,
            run: () => navigate(`/works/${workId}/score`),
          },
          {
            id: 'release',
            label: t('search.action.release', { title }),
            icon: CalendarPlus,
            run: () => navigate(`/works/${workId}`),
          },
        ]

  const actions: Command[] = [
    { id: 'new', label: t('works.newTitle'), icon: FilePlus, run: shell.newWork },
    ...onWork,
    {
      id: 'assistant',
      label: t('assistant.open'),
      icon: MessageSquare,
      run: () => assistant.open(),
    },
    { id: 'undo', label: t('keys.action.undo'), icon: Undo2, keys: ['Mod', 'Z'], run: undo },
    // Every theme and language but the one in use, rather than the rail's
    // "next in turn": typed into a palette, "dark" should find the dark theme
    // whichever one happens to follow the current.
    ...THEMES.filter((other) => other !== theme).map((other): Command => ({
      id: `theme-${other}`,
      label: t(`search.action.theme.${other}`),
      icon: THEME_ICONS[other],
      run: () => setTheme(other),
    })),
    ...LANGUAGES.filter((other) => other !== language).map((other): Command => ({
      id: `language-${other}`,
      label: t(`search.action.language.${other}`),
      icon: Languages,
      run: () => setLanguage(other),
    })),
    {
      id: 'rail',
      label: t(shell.compact ? 'shell.expandMenu' : 'shell.collapseMenu'),
      icon: shell.compact ? PanelLeftOpen : PanelLeftClose,
      run: shell.toggleRail,
    },
    { id: 'keys', label: t('keys.title'), icon: Keyboard, keys: ['?'], run: shell.showKeys },
  ]

  const screens = onRail(import.meta.env.DEV, config).map((screen): Command => ({
    id: `screen-${screen.key}`,
    label: t(screen.nav),
    icon: screen.rail!.icon,
    keys: screen.jump === undefined ? undefined : ['G', screen.jump.toUpperCase()],
    run: () => navigate(`/${screen.key}`),
  }))

  return { actions, screens }
}
