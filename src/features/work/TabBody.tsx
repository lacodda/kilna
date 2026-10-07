import type { ReactNode } from 'react'
import type { Work } from '@/lib/api/types'
import type { Tab } from '@/features/work/tabs'
import { AssistantPanel } from '@/features/work/tabs/assistant/AssistantPanel'
import { CommentsPanel } from '@/features/work/tabs/comments/CommentsPanel'
import { CoverTab } from '@/features/work/tabs/cover/CoverTab'
import { CutsTab } from '@/features/work/tabs/cuts/CutsTab'
import { FilesTab } from '@/features/work/tabs/files/FilesTab'
import { FrameTab } from '@/features/work/tabs/frame/FrameTab'
import { HistoryTab } from '@/features/work/tabs/history/HistoryTab'
import { LinksTab } from '@/features/work/tabs/links/LinksTab'
import { NotePanel } from '@/features/work/tabs/notes/NotePanel'
import { OverviewTab } from '@/features/work/tabs/overview/OverviewTab'
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
  // A board of widgets over the work. Plugins write into the work's own
  // `meta` and may rewrite its versions, so their commands stand in its head,
  // beside the fields they change rather than beside its releases.
  overview: ({ work }) => <OverviewTab work={work} />,
  versions: ({ workId }) => <VersionPanel workId={workId} />,
  scenes: ({ work }) => <ScenesTab work={work} />,
  // What a publication looks like where it goes out (v0.86): the cover's
  // prompt, and the still and the loop an audio release plays under.
  cover: ({ work }) => <CoverTab work={work} />,
  frame: ({ work }) => <FrameTab work={work} />,
  cuts: ({ work }) => <CutsTab work={work} />,
  score: ({ workId }) => <ScorePanel workId={workId} />,
  files: ({ work }) => <FilesTab work={work} />,
  links: ({ work }) => <LinksTab work={work} />,
  notes: ({ workId }) => <NotePanel workId={workId} />,
  comments: ({ workId }) => <CommentsPanel workId={workId} />,
  // The work's own conversation. The drawer from the window's bar holds every
  // chat; this tab holds this work's, as the mockup draws it (#p-asst).
  assistant: ({ workId }) => <AssistantPanel workId={workId} />,
  // The work's history on one axis (ADR 0056).
  history: ({ workId }) => <HistoryTab workId={workId} />,
}

/** The one tab that is open. Everything else is not mounted at all. */
export function TabBody({ tab, ...props }: Props & { tab: Tab }) {
  return BODIES[tab](props)
}
