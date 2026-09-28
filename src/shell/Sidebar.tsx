import { Link, useLocation } from 'react-router'
import { useTranslation } from 'react-i18next'
import { Languages, Monitor, Moon, Sun, type LucideIcon } from 'lucide-react'
import { nextTheme, useTheme, type Theme } from '@/lib/theme'
import { nextLanguage, useLanguage } from '@/lib/language'
import { useProfile } from '@/lib/useProfile'
import { onRail, railAt, SOON, type ScreenSpec } from '@/app/screens'
import { NavGroup, NavRail, NavSpacer, type NavRailItem } from '@/components/ui/nav-rail'
import { ProfileSwitcher } from '@/shell/ProfileSwitcher'
import { cn } from '@/lib/utils'

const THEME_ICONS: Record<Theme, LucideIcon> = {
  system: Monitor,
  light: Sun,
  dark: Moon,
}

// Each run of entries is a NavRail of its own, stacked in one column: the
// primitive draws destinations, and the caption and the foot between them are
// the rail a product assembles around it (dowel's stand builds its rail the
// same way). A run gives its height, border and padding up to the column, and
// its overflow too - clipped at a run's edge, the focus ring of the first
// entry in it would be cut in half.
const RUN = 'h-auto overflow-visible border-r-0 p-0'

/** A screen as an entry of the rail. */
function entryOf(screen: ScreenSpec, label: string): NavRailItem {
  const Icon = screen.rail!.icon
  return { id: screen.key, label, icon: <Icon /> }
}

/** A screen's entry is a link to it: the rail puts its clothes on the router's. */
const asLink = (item: NavRailItem) => <Link to={`/${item.id}`} />

interface Props {
  profileId: string
  onProfileSwitched: () => void
  /** Icons only: the words, the version chips and the profile row are
   *  dropped, the names move into tooltips, and the Library caption becomes a
   *  rule. */
  compact: boolean
}

// The left rail of the app frame: screens, the roadmap's next doors, and the
// footer with settings, theme, language and profile. The brand moved up into
// the title bar in v0.74, where a system title bar would have printed the
// name; the handle that folds this rail to icons sits beside it there.
export function Sidebar({ profileId, onProfileSwitched, compact }: Props) {
  const { t } = useTranslation()
  const { pathname } = useLocation()
  const { config } = useProfile()
  const { theme, setTheme } = useTheme()
  const { language, setLanguage } = useLanguage()
  const ThemeIcon = THEME_ICONS[theme]

  // The rail is the screens' own list (`app/screens.tsx`): a screen joins its
  // group the day it is added there, in the order it is listed.
  const screens = onRail(import.meta.env.DEV, config)
  const group = (name: 'work' | 'library' | 'foot') =>
    screens
      .filter((screen) => screen.rail?.group === name)
      .map((screen) => entryOf(screen, t(screen.nav)))

  // A door to a screen the plan promises and the build does not have yet. The
  // chip says which version brings it; folded to icons there is no chip, so
  // the promise moves into the name the tooltip shows.
  const soon = SOON.map((door): NavRailItem => {
    const Icon = door.icon
    const promise = t('nav.soon', { version: door.version })
    return {
      id: door.nav,
      label: compact ? `${t(door.nav)} · ${promise}` : t(door.nav),
      icon: <Icon />,
      soon: true,
      end: (
        <span title={promise} className="rounded-full border border-line px-1.5 font-mono">
          <span aria-hidden>{door.version}</span>
          <span className="sr-only">{promise}</span>
        </span>
      ),
    }
  })

  const active = railAt(pathname)
  const run = { activeId: active, collapsed: compact, className: RUN, render: asLink }

  return (
    // `h-full` so the rail runs the height of the window: the footer sits at
    // the bottom because the column reaches it, not because the content does.
    // `overflow-x-hidden`: while the width animates, the words of the full
    // menu are wider than the track for a moment and must not draw a bar.
    <div
      className={cn(
        'flex h-full flex-col gap-0.5 overflow-x-hidden overflow-y-auto border-r border-line pt-2.5 pb-3',
        compact ? 'px-2' : 'px-2.5',
      )}
    >
      <NavRail label={t('nav.screens')} items={group('work')} {...run} />

      {/* Folded, the caption becomes a rule: the grouping still reads, and a
          word cut to three letters would not. */}
      <NavGroup collapsed={compact}>{t('nav.library')}</NavGroup>
      <NavRail label={t('nav.library')} items={[...soon, ...group('library')]} {...run} />

      <NavSpacer>
        <NavRail label={t('nav.data')} items={group('foot')} {...run} />
        {/* The theme and the language are entries of the rail too, dressed
            as the links above them - but they act rather than go, so they are
            a run of their own that answers a press instead of rendering a
            link. */}
        <NavRail
          label={t('nav.appearance')}
          collapsed={compact}
          className={RUN}
          items={[
            { id: 'theme', label: t(`themeName.${theme}`), icon: <ThemeIcon /> },
            { id: 'language', label: t(`languageName.${language}`), icon: <Languages /> },
          ]}
          onSelect={(id) => {
            if (id === 'theme') setTheme(nextTheme(theme))
            else setLanguage(nextLanguage(language))
          }}
        />
        {/* The profile row is a name and a pill, and the folded rail has room
            for neither; it comes back with the full menu. */}
        {!compact && (
          <div className="px-1 pt-1">
            <ProfileSwitcher activeId={profileId} onSwitched={onProfileSwitched} />
          </div>
        )}
      </NavSpacer>
    </div>
  )
}
