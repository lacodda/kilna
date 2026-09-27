import type { ReactNode } from 'react'
import type { Work } from '@/lib/api/types'
import type { Tab } from '@/features/work/tabs'
import { PluginBar } from '@/features/work/PluginBar'
import { ActionBar } from '@/features/assistant/ActionBar'
import { WorkHistory } from '@/features/journal/JournalFeed'
import { AssistantPanel } from '@/features/work/tabs/assistant/AssistantPanel'
import { CommentsPanel } from '@/features/work/tabs/comments/CommentsPanel'
import { CutsTab } from '@/features/work/tabs/cuts/CutsTab'
import { FilesTab } from '@/features/work/tabs/files/FilesTab'
import { LinksTab } from '@/features/work/tabs/links/LinksTab'
import { NotePanel } from '@/features/work/tabs/notes/NotePanel'
import { OverviewTab } from '@/features/work/tabs/overview/OverviewTab'
import { ReleasePanel } from '@/features/work/tabs/releases/ReleasePanel'
import { ScenesTab } from '@/features/work/tabs/scenes/ScenesTab'
import { ScorePanel } from '@/features/work/tabs/score/ScorePanel'
import { VersionPanel } from '@/features/work/tabs/versions/VersionPanel'

interface Props {
  workId: string
  work: Work
}

/**
 * What each tab draws. A `Record` over every tab, so a tab added to the list
 * in `tabs.ts` does not compile until it is given a body here.
 */
const BODIES: Readonly<Record<Tab, (props: Props) => ReactNode>> = {
  // Plugins write into the work's own `meta` and may rewrite its versions, so
  // they belong beside the fields they change rather than beside its releases.
  overview: ({ workId, work }) => (
    <div className="flex flex-col gap-4">
      <OverviewTab work={work} />
      {/* Profile actions and plugin commands are the same gesture from the
          user's side — do this to this work — so they sit together, below
          the fields both of them read. One button that opens them: eight
          actions spelled out across the overview was a wall of buttons
          under a screen that is meant to be read. */}
      <ActionBar workId={workId} menu />
      <PluginBar target="work" id={workId} />
    </div>
  ),
  versions: ({ workId }) => <VersionPanel workId={workId} />,
  scenes: ({ work }) => <ScenesTab work={work} />,
  cuts: ({ work }) => <CutsTab work={work} />,
  score: ({ workId }) => <ScorePanel workId={workId} />,
  releases: ({ workId, work }) => <ReleasePanel workId={workId} workTitle={work.title} />,
  files: ({ work }) => <FilesTab work={work} />,
  links: ({ work }) => <LinksTab work={work} />,
  notes: ({ workId }) => <NotePanel workId={workId} />,
  comments: ({ workId }) => <CommentsPanel workId={workId} />,
  // The work's own conversation. The drawer from the window's bar holds every
  // chat; this tab holds this work's, as the mockup draws it (#p-asst).
  assistant: ({ workId }) => <AssistantPanel workId={workId} />,
  history: ({ workId }) => <WorkHistory workId={workId} />,
}

/** The one tab that is open. Everything else is not mounted at all. */
export function TabBody({ tab, ...props }: Props & { tab: Tab }) {
  return BODIES[tab](props)
}
