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
  version: (id: string) => ['version', id] as const,

  scores: ['scores'] as const,
  scoreHistory: (workId: string) => ['scores', 'history', workId] as const,
  latestScore: (workId: string) => ['scores', 'latest', workId] as const,
  catalogue: ['catalogue'] as const,

  releases: ['releases'] as const,
  releasesForWork: (workId: string) => ['releases', 'work', workId] as const,
  calendar: ['calendar'] as const,
  releaseQueue: ['releaseQueue'] as const,

  collections: ['collections'] as const,
  notes: ['notes'] as const,
  tags: ['tags'] as const,
  deletions: ['deletions'] as const,
  search: (query: string) => ['search', query] as const,

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
  runs: (chatId: string) => ['runs', chatId] as const,
  activeRuns: ['runs', 'active'] as const,
  activeTasks: ['runs', 'tasks'] as const,
  waitingChats: ['chats', 'waiting'] as const,
  assistantStatus: ['assistantStatus'] as const,

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
