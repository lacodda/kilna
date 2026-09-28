import { Navigate, NavLink, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { SectionNav } from '@/components/ui/section-nav'
import { StatusDot } from '@/components/ui/status-dot'
import { Frame, ListDetail, Pane } from '@/components/frame'
import { AgentsSection } from '@/features/settings/AgentsSection'
import { CardSection } from '@/features/settings/CardSection'
import { DataSection } from '@/features/settings/DataSection'
import { GeneralSection } from '@/features/settings/GeneralSection'
import { ProfileDraftBar } from '@/features/settings/ProfileDraftBar'
import { ProfileSection } from '@/features/settings/ProfileSection'
import { useProfileDraft } from '@/features/settings/useProfileDraft'

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
  // Read here as well as in the editor: the profile's draft outlives the
  // section (`lib/profileDraft`), so the list says it is waiting from any
  // section, and the bar that saves it stands at the foot of the pane.
  const draft = useProfileDraft()

  if (!isSection(section)) {
    return <Navigate to={`/settings/${DEFAULT_SECTION}`} replace />
  }

  return (
    // The nav does not move at all, and `sticky` could not give that: sticky
    // travels with the page until it reaches its offset, so a long section on
    // the right dragged the whole list up as far as the screen's title
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
            {/* Each section by its name and a line of what is in it, so a
                section is chosen by its contents rather than guessed from a
                word - which is also why the open section draws no heading of
                its own: the row beside it already says both. The caption
                "Settings" is kept for a screen reader only; the title bar
                says it on screen, and the audit found the screen naming
                itself twice. */}
            <SectionNav
              label={t('settings.title')}
              labelHidden
              activeId={section}
              items={SECTIONS.map((entry) => ({
                id: entry,
                // Every name in the weight of the mockup's list, not only the
                // open one: the tint already says which is open, and a name
                // that thickens on selection makes the row beside it look
                // like a lesser section.
                label: (
                  <>
                    <span className="font-semibold text-text">
                      {t(`settings.section.${entry}`)}
                    </span>
                    {entry === 'profile' && draft.dirty && (
                      <StatusDot
                        status="warn"
                        size="sm"
                        label={t('settings.unsaved')}
                        className="ml-1.5 align-middle"
                      />
                    )}
                  </>
                ),
                description: t(`settings.hint.${entry}`),
              }))}
              // A link, so the back button walks between sections and one can
              // be opened by address.
              render={(item) => <NavLink to={`/settings/${item.id}`} />}
            />
          </Pane>
        }
        detail={
          <Pane
            label={t(`settings.section.${section}`)}
            bodyClassName="flex flex-col px-4.5 py-4"
            // The profile is the one section with a Save: its bar stands at
            // the foot of the pane, in sight however far down the edit was,
            // and only while there is something to save.
            foot={
              section === 'profile' && draft.dirty ? <ProfileDraftBar draft={draft} /> : undefined
            }
          >
            <Body section={section} />
          </Pane>
        }
      />
    </Frame>
  )
}
