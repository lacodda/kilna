import type { ReactNode } from 'react'
import { NavLink } from 'react-router'
import { useTranslation } from 'react-i18next'
import {
  Calendar,
  Disc,
  FileText,
  History,
  Languages,
  LayoutDashboard,
  List,
  Monitor,
  Moon,
  Palette,
  Settings,
  Shapes,
  Sun,
  Trash2,
  type LucideIcon,
} from 'lucide-react'
import { nextTheme, useTheme, type Theme } from '@/lib/theme'
import { nextLanguage, useLanguage } from '@/lib/language'
import { styleTypesOf, useProfile } from '@/lib/useProfile'
import { ProfileSwitcher } from '@/components/ProfileSwitcher'
import { cn } from '@/lib/utils'

const THEME_ICONS: Record<Theme, LucideIcon> = {
  system: Monitor,
  light: Sun,
  dark: Moon,
}

const NAV_CLASS =
  'flex w-full cursor-pointer items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm text-dim transition-colors hover:bg-soft hover:text-text [&_svg]:size-4 [&_svg]:shrink-0'

// In the compact menu an entry is its icon, centred, and the word moves into
// the tooltip: the only place left to say where the icon leads.
const COMPACT_CLASS = 'justify-center px-0'

/** The word beside an icon, which the compact menu drops. */
function Label({ children, compact }: { children: ReactNode; compact: boolean }) {
  return compact ? null : <span className="min-w-0 truncate">{children}</span>
}

function ScreenLink({
  to,
  icon: Icon,
  label,
  compact,
}: {
  to: string
  icon: LucideIcon
  label: string
  compact: boolean
}) {
  return (
    <NavLink
      to={to}
      title={compact ? label : undefined}
      aria-label={compact ? label : undefined}
      className={({ isActive }) =>
        cn(
          NAV_CLASS,
          compact && COMPACT_CLASS,
          isActive && 'bg-accent-soft text-text [&_svg]:text-accent',
        )
      }
    >
      <Icon aria-hidden />
      <Label compact={compact}>{label}</Label>
    </NavLink>
  )
}

// A nav entry for a screen that exists on the roadmap but not in the build
// yet; the chip names the version that delivers it.
function SoonLink({
  icon: Icon,
  label,
  version,
  compact,
}: {
  icon: LucideIcon
  label: string
  version: string
  compact: boolean
}) {
  const { t } = useTranslation()
  const soon = t('nav.soon', { version })
  return (
    <span
      className={cn(
        NAV_CLASS,
        compact && COMPACT_CLASS,
        'cursor-default text-faint hover:bg-transparent hover:text-faint',
      )}
      title={compact ? `${label} · ${soon}` : soon}
    >
      <Icon aria-hidden />
      <Label compact={compact}>{label}</Label>
      {!compact && (
        <span className="ml-auto rounded-full border border-line px-1.5 font-mono text-[9.5px]">
          {version}
        </span>
      )}
    </span>
  )
}

interface Props {
  profileId: string
  onProfileSwitched: () => void
  /** Icons only: the words, the version chips and the profile row are
   *  dropped, and the Library caption becomes a rule. */
  compact: boolean
}

// The left rail of the app frame: screens, the roadmap's next doors, and the
// footer with settings, theme and profile. The brand moved up into the title
// bar in v0.74, where a system title bar would have printed the name; the
// handle that folds this rail to icons sits beside it there.
export function Sidebar({ profileId, onProfileSwitched, compact }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const hasStyles = styleTypesOf(config).length > 0
  const { theme, setTheme } = useTheme()
  const { language, setLanguage } = useLanguage()
  const ThemeIcon = THEME_ICONS[theme]
  const themeName = t(`themeName.${theme}`)
  const languageName = t(`languageName.${language}`)

  return (
    // `h-full` so the rail runs the height of the window: the footer sits at
    // the bottom because the nav reaches it, not because the content does.
    // `overflow-x-hidden`: while the width animates, the words of the full
    // menu are wider than the track for a moment and must not draw a bar.
    <nav
      className={cn(
        'flex h-full flex-col gap-0.5 overflow-x-hidden overflow-y-auto border-r border-line pt-2.5 pb-3',
        compact ? 'px-1.5' : 'px-2.5',
      )}
    >
      <ScreenLink to="/dashboard" icon={LayoutDashboard} label={t('nav.dashboard')} compact={compact} />
      {/* No separate Works entry: the catalogue is the list of works, and a
          second door to the same things only made you choose between them. */}
      <ScreenLink to="/catalogue" icon={List} label={t('nav.catalogue')} compact={compact} />
      <ScreenLink to="/calendar" icon={Calendar} label={t('nav.calendar')} compact={compact} />

      {/* The caption becomes a rule in the compact menu: the grouping still
          reads, and a word cut to three letters would not. */}
      {compact ? (
        <div aria-hidden className="mx-auto my-2.5 h-px w-6 shrink-0 bg-line" />
      ) : (
        <div className="px-2.5 pt-3 pb-1 text-[10.5px] font-medium uppercase tracking-[0.09em] text-faint">
          {t('nav.library')}
        </div>
      )}
      <SoonLink icon={Disc} label={t('nav.collections')} version="0.80" compact={compact} />
      <SoonLink icon={FileText} label={t('nav.notes')} version="0.76" compact={compact} />
      {/* Only where the craft has one: a profile that names no style types
          has no dictionary, and a door to an empty room is worse than none. */}
      {hasStyles && (
        <ScreenLink to="/styles" icon={Shapes} label={t('nav.styles')} compact={compact} />
      )}
      <ScreenLink to="/journal" icon={History} label={t('nav.journal')} compact={compact} />
      <ScreenLink to="/trash" icon={Trash2} label={t('nav.trash')} compact={compact} />

      <div className="mt-auto flex flex-col gap-0.5">
        {import.meta.env.DEV && (
          <ScreenLink
            to="/styleguide"
            icon={Palette}
            label={t('nav.styleguide')}
            compact={compact}
          />
        )}
        <ScreenLink to="/settings" icon={Settings} label={t('nav.data')} compact={compact} />
        <button
          type="button"
          className={cn(NAV_CLASS, compact && COMPACT_CLASS)}
          title={compact ? themeName : undefined}
          aria-label={compact ? themeName : undefined}
          onClick={() => setTheme(nextTheme(theme))}
        >
          <ThemeIcon aria-hidden />
          <Label compact={compact}>{themeName}</Label>
        </button>
        <button
          type="button"
          className={cn(NAV_CLASS, compact && COMPACT_CLASS)}
          title={compact ? languageName : undefined}
          aria-label={compact ? languageName : undefined}
          onClick={() => setLanguage(nextLanguage(language))}
        >
          <Languages aria-hidden />
          <Label compact={compact}>{languageName}</Label>
        </button>
        {/* The profile row is a name and a pill, and the compact menu has
            room for neither; it comes back with the full menu. */}
        {!compact && (
          <div className="px-1 pt-1">
            <ProfileSwitcher activeId={profileId} onSwitched={onProfileSwitched} />
          </div>
        )}
      </div>
    </nav>
  )
}
