import { QueryClient } from '@tanstack/react-query'

/**
 * Query keys in one place.
 *
 * Every key starts with a coarse segment (`works`, `calendar`) so a mutation can
 * invalidate a whole area with one prefix rather than naming each query it might
 * have disturbed — the thing the revision counters used to do by hand, and got
 * wrong whenever a new screen forgot to bump.
 */
export const keys = {
  workspace: ['workspace'] as const,
  profiles: ['profiles'] as const,

  works: ['works'] as const,
  work: (id: string) => ['works', 'item', id] as const,

  versions: (workId: string) => ['versions', workId] as const,
  // One prefix over every work's links: a link changes two cards at once.
  links: ['links'] as const,
  linksFor: (workId: string) => ['links', workId] as const,
  scenes: ['scenes'] as const,
  scenesFor: (workId: string) => ['scenes', workId] as const,
  // One prefix over every splice: a stretch changes the short's track and the
  // donor's card at the same time, the way a link changes two cards.
  cuts: ['cuts'] as const,
  cutsFor: (workId: string) => ['cuts', workId] as const,
  // What the cutter is told to do. Under the same prefix, because a stretch
  // moving changes it and so does a video arriving on the donor.
  shotsFor: (workId: string) => ['cuts', 'shots', workId] as const,
  // The frames of a whole board in one key: the storyboard draws every scene
  // together, and a frame arriving changes the row it lands on and the
  // readiness of that row at the same time.
  sceneFramesFor: (workId: string) => ['sceneFrames', workId] as const,
  version: (id: string) => ['version', id] as const,

  /** Proposals waiting for an answer, as the bell reads them. */
  pendingProposals: ['proposals', 'pending'] as const,
  scores: ['scores'] as const,
  scoreHistory: (workId: string) => ['scores', 'history', workId] as const,
  latestScore: (workId: string) => ['scores', 'latest', workId] as const,
  kindVerdicts: (workId: string) => ['scores', 'kinds', workId] as const,
  catalogue: ['catalogue'] as const,

  releases: ['releases'] as const,
  releasesForWork: (workId: string) => ['releases', 'work', workId] as const,
  // What one release goes out as. Its own key rather than part of the work's
  // list: the boxes refetch as they are typed into, and pulling the whole
  // list each time would redraw every row around them.
  releaseFields: (releaseId: string) => ['releases', 'fields', releaseId] as const,
  calendar: ['calendar'] as const,
  releaseQueue: ['releaseQueue'] as const,

  collections: ['collections'] as const,
  // One key for every work's cover: attaching one changes the catalogue,
  // the card, the calendar and the dashboard at once.
  covers: ['covers'] as const,
  assetsFor: (workId: string) => ['assets', workId] as const,
  notes: ['notes'] as const,
  // One prefix over the inbox, the channels, a work's counter and what waits
  // to be kept: keeping a comment changes all four at once.
  comments: ['comments'] as const,
  commentChannels: ['comments', 'channels'] as const,
  commentCount: (workId: string) => ['comments', 'count', workId] as const,
  commentProposals: ['comments', 'proposals'] as const,
  /** What the links in one body point at, keyed by the ids they name: two
      bodies naming the same works share the answer. */
  resolvedLinks: (ids: string) => ['links', 'resolved', ids] as const,
  // One coarse prefix over the style dictionary: a brick changes the
  // dictionary screen, the counts beside its types and the picker the
  // constructor opens, and none of the three is worth invalidating alone.
  styles: ['styles'] as const,
  styleBricks: ['styles', 'list'] as const,
  styleCounts: ['styles', 'counts'] as const,
  styleBrick: (id: string) => ['styles', 'item', id] as const,
  styleReferences: (id: string) => ['styles', 'references', id] as const,
  tags: ['tags'] as const,
  workTags: ['workTags'] as const,
  deletions: ['deletions'] as const,
  search: (query: string) => ['search', query] as const,
  /** The catalogue's own question: which works answer this text. */
  worksMatching: (query: string) => ['worksMatching', query] as const,

  // One coarse prefix over the feed, a work's history and the unread count:
  // every mutation that writes an entry disturbs all three, and none of them is
  // ever worth invalidating alone.
  journal: ['journal'] as const,
  journalFeed: ['journal', 'feed'] as const,
  journalForWork: (workId: string) => ['journal', 'work', workId] as const,
  journalUnread: ['journal', 'unread'] as const,

  // One coarse prefix over every chat list: a finished run moves captions and
  // prices in the card's list and the drawer's alike.
  allChats: ['chats'] as const,
  chats: (workId?: string) => ['chats', workId ?? null] as const,
  transcript: (chatId: string) => ['transcript', chatId] as const,
  // Every transcript: a proposal applied marks its message, whichever chat
  // it is in, and the component that applied it need not know the chat.
  transcripts: ['transcript'] as const,
  runs: (chatId: string) => ['runs', chatId] as const,
  activeRuns: ['runs', 'active'] as const,
  activeTasks: ['runs', 'tasks'] as const,
  taskQueue: ['runs', 'queue'] as const,
  waitingChats: ['chats', 'waiting'] as const,
  assistantStatus: ['assistantStatus'] as const,

  // One coarse prefix over the focus board: a dismissal changes what the
  // findings half shows, a note changes the other, and the board reads both.
  focus: ['focus'] as const,
  dismissals: ['focus', 'dismissals'] as const,
  focusNotes: ['focus', 'notes'] as const,

  plugins: ['plugins'] as const,
} as const

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // Local SQLite behind an IPC call: refetching is cheap, but not free
      // enough to do on every window focus while someone is typing.
      staleTime: 30_000,
      refetchOnWindowFocus: false,
      // A missing row will not become present by asking again, and a broken
      // query should surface now rather than after three silent retries.
      retry: false,
    },
    mutations: { retry: false },
  },
})
