import { useLocation, useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import {
  CalendarPlus,
  FilePlus,
  Layers,
  Languages,
  MessageSquare,
  Monitor,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  Star,
  Sun,
  type LucideIcon,
} from 'lucide-react'
import { onRail } from '@/app/screens'
import { useCommands, type Command } from '@/components/ui/commands'
import { LANGUAGES, useLanguage } from '@/lib/language'
import { queries } from '@/lib/query/queries'
import { openWorkId } from '@/lib/route'
import { THEMES, useTheme, type Theme } from '@/lib/theme'
import { useAssistant } from '@/lib/useAssistant'
import { useProfile } from '@/lib/useProfile'
import { useOpenSpace } from '@/features/spaces/useOpenSpace'

/** What the shell hands the commands: the parts of it that live elsewhere. */
export interface ShellCommands {
  /** Open the New work dialog. */
  newWork: () => void
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
 * The shell's commands that need the workspace: where to go, and what can be
 * done from anywhere.
 *
 * Declared on dowel's list (wish 4308), so the palette, the keyboard and the
 * sheet of shortcuts read one list. The screens are the rail's own list
 * (`app/screens.tsx`, through `onRail`), each with its `G` sequence, so
 * neither the palette nor the keyboard reaches a door the rail does not
 * have. Every other space is a place to go too (ADR 0077). The actions are
 * the shell's, plus two for the work that is open - scoring it and putting
 * it on the calendar are the two things most often opened a card for.
 */
export function useShellCommands(shell: ShellCommands): void {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const { config } = useProfile()
  const assistant = useAssistant()
  const { theme, setTheme } = useTheme()
  const { language, setLanguage } = useLanguage()
  const spaces = useQuery(queries.spaces())
  const openSpace = useOpenSpace()

  // The trail in the title bar has asked for this already, so it is read from
  // the cache rather than fetched a second time.
  const workId = openWorkId(pathname)
  const work = useQuery({ ...queries.work(workId ?? ''), enabled: workId !== undefined })
  const title = work.data?.title

  const actions = t('search.group.action')
  const screens = t('search.group.screen')

  const onWork: Command[] =
    workId === undefined || title === undefined
      ? []
      : [
          {
            id: 'score',
            label: t('search.action.score', { title }),
            group: actions,
            icon: <Star aria-hidden />,
            run: () => navigate(`/works/${workId}/score`),
          },
          {
            id: 'release',
            label: t('search.action.release', { title }),
            group: actions,
            icon: <CalendarPlus aria-hidden />,
            run: () => navigate(`/works/${workId}`),
          },
        ]

  const RailIcon = shell.compact ? PanelLeftOpen : PanelLeftClose

  useCommands([
    {
      id: 'new',
      label: t('works.newTitle'),
      group: actions,
      icon: <FilePlus aria-hidden />,
      run: shell.newWork,
    },
    ...onWork,
    {
      id: 'assistant',
      label: t('assistant.open'),
      group: actions,
      icon: <MessageSquare aria-hidden />,
      run: () => assistant.open(),
    },
    // Every theme and language but the one in use, rather than the rail's
    // "next in turn": typed into a palette, "dark" should find the dark theme
    // whichever one happens to follow the current.
    ...THEMES.filter((other) => other !== theme).map((other): Command => {
      const Icon = THEME_ICONS[other]
      return {
        id: `theme-${other}`,
        label: t(`search.action.theme.${other}`),
        group: actions,
        icon: <Icon aria-hidden />,
        run: () => setTheme(other),
      }
    }),
    ...LANGUAGES.filter((other) => other !== language).map((other): Command => ({
      id: `language-${other}`,
      label: t(`search.action.language.${other}`),
      group: actions,
      icon: <Languages aria-hidden />,
      run: () => setLanguage(other),
    })),
    {
      id: 'rail',
      label: t(shell.compact ? 'shell.expandMenu' : 'shell.collapseMenu'),
      group: actions,
      icon: <RailIcon aria-hidden />,
      run: shell.toggleRail,
    },
    ...onRail(import.meta.env.DEV, config).map((screen): Command => {
      const Icon = screen.rail!.icon
      return {
        id: `screen-${screen.key}`,
        label: t(screen.nav),
        group: screens,
        icon: <Icon aria-hidden />,
        keys: screen.jump === undefined ? undefined : `G ${screen.jump.toUpperCase()}`,
        run: () => navigate(`/${screen.key}`),
      }
    }),
    ...(spaces.data ?? [])
      .filter((space) => !space.open && space.trashed_at === null && space.reachable)
      .map((space): Command => ({
        id: `space-${space.id}`,
        label: t('spaces.goTo', { name: space.name }),
        group: screens,
        icon: <Layers aria-hidden />,
        run: () => openSpace.mutate(space.id),
      })),
  ])
}
