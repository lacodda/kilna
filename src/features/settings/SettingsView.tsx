import { Navigate, NavLink, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { SectionHeading, SectionNav } from '@/components/ui/section-nav'
import { Frame, ListDetail, Pane } from '@/components/frame'
import {
  Bot,
  Database,
  LayoutPanelTop,
  SlidersHorizontal,
  Sparkles,
  type LucideIcon,
} from 'lucide-react'
import { AgentsSection } from '@/features/settings/AgentsSection'
import { CardSection } from '@/features/settings/CardSection'
import { DataSection } from '@/features/settings/DataSection'
import { GeneralSection } from '@/features/settings/GeneralSection'
import { ProfileSection } from '@/features/settings/ProfileSection'

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
    // The nav does not move at all, and `sticky` could not give that: sticky
    // travels with the page until it reaches its offset, so a long section on
    // the right dragged the whole list up as far as the word "Настройки"
    // before pinning - which reads as a menu that scrolls, because for those
    // first pixels it does. Only the section scrolls instead, and the nav is
    // a column of the window's height beside it, the width of the app's own
    // rail. It stays beside the section at every width: stacked above it on a
    // narrow window, the whole screen scrolled as one.
    <Frame>
      <ListDetail
        width="rail"
        list={
          <Pane label={t('settings.title')} bodyClassName="p-1.5">
            <SectionNav
              label={t('settings.title')}
              activeId={section}
              items={SECTIONS.map((entry) => {
                const Icon = ICONS[entry]
                return { id: entry, label: t(`settings.section.${entry}`), icon: <Icon /> }
              })}
              // A link, so the back button walks between sections and one can
              // be opened by address.
              render={(item) => <NavLink to={`/settings/${item.id}`} />}
            />
          </Pane>
        }
        detail={
          <Pane label={t(`settings.section.${section}`)} bodyClassName="flex flex-col px-4.5 py-4">
            <SectionHeading
              title={t(`settings.section.${section}`)}
              description={t(`settings.hint.${section}`)}
            />
            <Body section={section} />
          </Pane>
        }
      />
    </Frame>
  )
}
