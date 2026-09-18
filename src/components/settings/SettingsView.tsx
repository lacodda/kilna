import { Navigate, NavLink, useParams } from 'react-router'
import { SectionHeading, SectionNav } from '@/components/ui/section-nav'
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
      <SectionNav
        label={t('settings.title')}
        className="md:sticky md:top-0"
        activeId={section}
        items={SECTIONS.map((entry) => {
          const Icon = ICONS[entry]
          return { id: entry, label: t(`settings.section.${entry}`), icon: <Icon /> }
        })}
        // A link, so the back button walks between sections and one can be
        // opened by address.
        render={(item) => <NavLink to={`/settings/${item.id}`} />}
      />

      <div className="flex min-w-0 flex-col">
        <SectionHeading
          title={t(`settings.section.${section}`)}
          description={t(`settings.hint.${section}`)}
        />
        <Body section={section} />
      </div>
    </div>
  )
}
