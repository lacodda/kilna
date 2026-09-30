import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import type { VersionRole, Work } from '@/lib/api/types'
import {
  applies,
  boardOf,
  fieldsOf,
  styleRoleOf,
  textRoleOf,
  type Placed,
  type WidgetFacts,
  type WidgetId,
} from '@/lib/overview'
import { useCovers } from '@/lib/useCovers'
import { fieldsFor, hasDoors, hasScenes, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Frame, Scroll } from '@/components/frame'
import { ActionBar } from '@/features/assistant/ActionBar'
import { PluginBar } from '@/features/work/PluginBar'
import { AxesWidget } from '@/features/work/tabs/overview/AxesWidget'
import { BoardLayout } from '@/features/work/tabs/overview/BoardLayout'
import { CoverWidget } from '@/features/work/tabs/overview/CoverWidget'
import { FieldsWidget } from '@/features/work/tabs/overview/FieldsWidget'
import { FindingsWidget } from '@/features/work/tabs/overview/FindingsWidget'
import { HookWidget } from '@/features/work/tabs/overview/HookWidget'
import { LinksWidget } from '@/features/work/tabs/overview/LinksWidget'
import { PublicationsWidget } from '@/features/work/tabs/overview/PublicationsWidget'
import { RecentWidget } from '@/features/work/tabs/overview/RecentWidget'
import { ReleasesWidget } from '@/features/work/tabs/overview/ReleasesWidget'
import { ScoreWidget } from '@/features/work/tabs/overview/ScoreWidget'
import { StageWidget } from '@/features/work/tabs/overview/StageWidget'
import { StoryboardWidget } from '@/features/work/tabs/overview/StoryboardWidget'
import { TrendWidget } from '@/features/work/tabs/overview/TrendWidget'
import { useWorkFindings } from '@/features/work/tabs/overview/useWorkFindings'
import { VersionWidget } from '@/features/work/tabs/overview/VersionWidget'

interface Props {
  work: Work
}

/** What a widget is drawn from: the work, and the roles its text widgets read. */
interface Drawn {
  work: Work
  text: VersionRole | undefined
  style: VersionRole | undefined
}

/**
 * What each widget of the catalogue draws. A `Record` over every id, so a
 * widget added to `WIDGETS` in `lib/overview` does not compile until it is
 * given a body here - the way `TabBody` holds the tabs.
 */
const WIDGET_BODIES: Readonly<Record<WidgetId, (drawn: Drawn) => ReactNode>> = {
  score: ({ work }) => <ScoreWidget work={work} />,
  stage: ({ work }) => <StageWidget work={work} />,
  text: ({ work, text }) => text !== undefined && <VersionWidget work={work} role={text} />,
  style: ({ work, style }) => style !== undefined && <VersionWidget work={work} role={style} />,
  fields: ({ work }) => <FieldsWidget work={work} />,
  axes: ({ work }) => <AxesWidget work={work} />,
  hook: ({ work }) => <HookWidget work={work} />,
  releases: ({ work }) => <ReleasesWidget work={work} />,
  links: ({ work }) => <LinksWidget work={work} />,
  recent: ({ work }) => <RecentWidget work={work} />,
  storyboard: ({ work }) => <StoryboardWidget work={work} />,
  cover: ({ work }) => <CoverWidget work={work} />,
  findings: ({ work }) => <FindingsWidget work={work} />,
  trend: ({ work }) => <TrendWidget work={work} />,
  publications: ({ work }) => <PublicationsWidget work={work} />,
}

/**
 * The overview: the work at a glance, as a board of widgets (v0.82).
 *
 * It was a form - the title, the status, the kind and every field of the
 * profile in boxes - under a header that shows the same values. Now it says
 * what the work is and where it stands, a widget per fact, each one the way to
 * the tab that owns its fact, and the fields are edited where they are read.
 *
 * The profile says how the widgets are laid out (`config.overview`, read by
 * `boardOf`): the owner chose the lead column and its rail, and Settings can
 * pick any of the five. What a work's kind does not have - a storyboard on a
 * song, a style prompt on a clip, releases on a song, which goes out as its
 * publications instead - is not drawn at all (`applies`).
 *
 * The profile's actions and the plugins' commands stand in the board's head:
 * the same gesture from the person's side - do this to this work - and a
 * head that stands while the board scrolls under it.
 */
export function OverviewTab({ work }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const vocabulary = vocabularyOf(profile.config, work.kind)
  const standing = useWorkFindings(work.id)
  const covers = useCovers()

  const board = boardOf(profile.config)
  const drawn: Drawn = {
    work,
    text: textRoleOf(vocabulary.version_roles),
    style: styleRoleOf(vocabulary.version_roles),
  }
  const facts: WidgetFacts = {
    scored: vocabulary.axes.length > 0,
    text: drawn.text !== undefined,
    style: drawn.style !== undefined,
    // The fields the kind has, not every field of the profile: a premise the
    // kind does not carry is no reason to draw the widget that edits one.
    prose: fieldsOf(fieldsFor(profile.config, work.kind)).prose.length > 0,
    releases: hasDoors(profile.config, work.kind),
    // A song lists what goes out for it where a clip lists its releases.
    publications: !hasDoors(profile.config, work.kind),
    scenes: hasScenes(profile.config, work.kind),
    // A song's picture is its publications'; one it already holds - from
    // before v0.86, or an import - is still shown.
    cover: hasDoors(profile.config, work.kind) || covers.has(work.id),
    findings: standing.length > 0,
  }
  const widgets = board.widgets.filter((widget) => applies(widget.id, facts))

  return (
    <Frame
      head={
        <>
          <ActionBar workId={work.id} menu />
          <PluginBar target="work" id={work.id} />
        </>
      }
    >
      <Scroll label={t('card.tab.overview')} contentClassName="@container pb-1">
        <BoardLayout
          layout={board.layout}
          widgets={widgets}
          render={(widget: Placed) => WIDGET_BODIES[widget.id](drawn)}
        />
      </Scroll>
    </Frame>
  )
}
