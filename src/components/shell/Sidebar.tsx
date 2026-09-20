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
  Sun,
  Trash2,
  type LucideIcon,
} from 'lucide-react'
import { nextTheme, useTheme, type Theme } from '@/lib/theme'
import { nextLanguage, useLanguage } from '@/lib/language'
import { ProfileSwitcher } from '@/components/ProfileSwitcher'
import { cn } from '@/lib/utils'

const THEME_ICONS: Record<Theme, LucideIcon> = {
  system: Monitor,
  light: Sun,
  dark: Moon,
}

const NAV_CLASS =
  'flex w-full cursor-pointer items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm text-dim transition-colors hover:bg-soft hover:text-text [&_svg]:size-4 [&_svg]:shrink-0'

function ScreenLink({ to, icon: Icon, label }: { to: string; icon: LucideIcon; label: string }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        cn(NAV_CLASS, isActive && 'bg-accent-soft text-text [&_svg]:text-accent')
      }
    >
      <Icon aria-hidden />
      {label}
    </NavLink>
  )
}

// A nav entry for a screen that exists on the roadmap but not in the build
// yet; the chip names the version that delivers it.
function SoonLink({ icon: Icon, label, version }: { icon: LucideIcon; label: string; version: string }) {
  const { t } = useTranslation()
  return (
    <span
      className={cn(NAV_CLASS, 'cursor-default text-faint hover:bg-transparent hover:text-faint')}
      title={t('nav.soon', { version })}
    >
      <Icon aria-hidden />
      {label}
      <span className="ml-auto rounded-full border border-line px-1.5 font-mono text-[9.5px]">
        {version}
      </span>
    </span>
  )
}

interface Props {
  profileId: string
  onProfileSwitched: () => void
}

// The left rail of the app frame: screens, the roadmap's next doors, and the
// footer with settings, theme and profile. The brand moved up into the title
// bar in v0.74, where a system title bar would have printed the name.
export function Sidebar({ profileId, onProfileSwitched }: Props) {
  const { t } = useTranslation()
  const { theme, setTheme } = useTheme()
  const { language, setLanguage } = useLanguage()
  const ThemeIcon = THEME_ICONS[theme]

  return (
    // `h-full` so the rail runs the height of the window: the footer sits at
    // the bottom because the nav reaches it, not because the content does.
    <nav className="flex h-full flex-col gap-0.5 overflow-y-auto border-r border-line px-2.5 pt-2.5 pb-3">
      <ScreenLink to="/dashboard" icon={LayoutDashboard} label={t('nav.dashboard')} />
      {/* No separate Works entry: the catalogue is the list of works, and a
          second door to the same things only made you choose between them. */}
      <ScreenLink to="/catalogue" icon={List} label={t('nav.catalogue')} />
      <ScreenLink to="/calendar" icon={Calendar} label={t('nav.calendar')} />

      <div className="px-2.5 pt-3 pb-1 text-[10.5px] font-medium uppercase tracking-[0.09em] text-faint">
        {t('nav.library')}
      </div>
      <SoonLink icon={Disc} label={t('nav.collections')} version="0.58" />
      <SoonLink icon={FileText} label={t('nav.notes')} version="0.59" />
      <ScreenLink to="/journal" icon={History} label={t('nav.journal')} />
      <ScreenLink to="/trash" icon={Trash2} label={t('nav.trash')} />

      <div className="mt-auto flex flex-col gap-0.5">
        {import.meta.env.DEV && (
          <ScreenLink to="/styleguide" icon={Palette} label={t('nav.styleguide')} />
        )}
        <ScreenLink to="/settings" icon={Settings} label={t('nav.data')} />
        <button type="button" className={NAV_CLASS} onClick={() => setTheme(nextTheme(theme))}>
          <ThemeIcon aria-hidden />
          {t(`themeName.${theme}`)}
        </button>
        <button
          type="button"
          className={NAV_CLASS}
          onClick={() => setLanguage(nextLanguage(language))}
        >
          <Languages aria-hidden />
          {t(`languageName.${language}`)}
        </button>
        <div className="px-1 pt-1">
          <ProfileSwitcher activeId={profileId} onSwitched={onProfileSwitched} />
        </div>
      </div>
    </nav>
  )
}
