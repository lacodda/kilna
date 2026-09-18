import { Navigate, NavLink, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import {
  Bot,
  Database,
  LayoutPanelTop,
  SlidersHorizontal,
  Sparkles,
  type LucideIcon,
} from 'lucide-react'
import { AgentsSection } from '@/components/settings/AgentsSection'
import { CardSection } from '@/components/settings/CardSection'
import { DataSection } from '@/components/settings/DataSection'
import { GeneralSection } from '@/components/settings/GeneralSection'
import { ProfileSection } from '@/components/settings/ProfileSection'
import { cn } from '@/lib/utils'

/**
 * The sections of Settings, in the order they are listed.
 *
 * Until v0.74 everything sat on one page: the whole profile editor, the status
 * check, a switch for the card, export, backup, import and the MCP command, in
 * one scroll. Nobody could find the one thing they came for. Each section is
 * now its own address, so it can be linked to and the back button walks
 * between them - the same reason a card's tab is in the URL.
 */
export const SECTIONS = ['general', 'card', 'profile', 'data', 'agents'] as const
export type Section = (typeof SECTIONS)[number]

const ICONS: Record<Section, LucideIcon> = {
  general: SlidersHorizontal,
  card: LayoutPanelTop,
  profile: Sparkles,
  data: Database,
  agents: Bot,
}

const DEFAULT_SECTION: Section = 'general'

function isSection(value: string | undefined): value is Section {
  return value !== undefined && (SECTIONS as readonly string[]).includes(value)
}

function Body({ section }: { section: Section }) {
  switch (section) {
    case 'general':
      return <GeneralSection />
    case 'card':
      return <CardSection />
    case 'profile':
      return <ProfileSection />
    case 'data':
      return <DataSection />
    case 'agents':
      return <AgentsSection />
  }
}

export function SettingsView() {
  const { t } = useTranslation()
  const { section } = useParams()

  if (!isSection(section)) {
    return <Navigate to={`/settings/${DEFAULT_SECTION}`} replace />
  }

  return (
    <div className="grid gap-8 md:grid-cols-[11rem_minmax(0,1fr)]">
      <nav aria-label={t('settings.title')} className="flex flex-col gap-0.5 md:sticky md:top-0">
        <h2 className="px-2.5 pb-2 text-[10.5px] font-medium uppercase tracking-[0.09em] text-faint">
          {t('settings.title')}
        </h2>
        {SECTIONS.map((entry) => {
          const Icon = ICONS[entry]
          return (
            <NavLink
              key={entry}
              to={`/settings/${entry}`}
              className={({ isActive }) =>
                cn(
                  'flex items-center gap-2.5 rounded-[10px] px-2.5 py-1.5 text-sm text-dim transition-colors hover:bg-soft hover:text-text [&_svg]:size-4 [&_svg]:shrink-0',
                  isActive && 'bg-accent-soft text-text [&_svg]:text-accent',
                )
              }
            >
              <Icon aria-hidden />
              {t(`settings.section.${entry}`)}
            </NavLink>
          )
        })}
      </nav>

      <div className="flex min-w-0 flex-col gap-1">
        <h2 className="text-base font-semibold">{t(`settings.section.${section}`)}</h2>
        <p className="mb-4 text-sm text-dim">{t(`settings.hint.${section}`)}</p>
        <Body section={section} />
      </div>
    </div>
  )
}
