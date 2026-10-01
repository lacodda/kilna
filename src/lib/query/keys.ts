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
  /** What was made from a work, and where each stands (v0.86). Under the
   *  works' prefix, because a status changing anywhere down the chain is a
   *  work changing; the links, releases and comments refresh it as well. */
  publications: ['works', 'publications'] as const,
  publicationsFor: (workId: string) => ['works', 'publications', workId] as const,
  /** The frame's blocks as they are copied, written on the Rust side from
   *  the frame the work holds. */
  framePromptsFor: (workId: string) => ['works', 'frame', workId] as const,

  versions: (workId: string) => ['versions', workId] as const,
  // One prefix over every work's links: a link changes two cards at once.
  links: ['links'] as const,
  linksFor: (workId: string) => ['links', workId] as const,
  scenes: ['scenes'] as const,
  scenesFor: (workId: string) => ['scenes', workId] as const,
  /** The notes a board's scenes point at, per work. */
  sceneNotesFor: (workId: string) => ['scenes', 'notes', workId] as const,
  /** Whether a work has anything to pack. Under the scenes' prefix, because a
   *  board gaining its first scene is what usually changes the answer. */
  scenePackageFor: (workId: string) => ['scenes', 'package', workId] as const,
  // One prefix over every splice: a stretch changes the short's track and the
  // donor's card at the same time, the way a link changes two cards.
  cuts: ['cuts'] as const,
  cutsFor: (workId: string) => ['cuts', workId] as const,
  /** What was cut out of a donor, for its own card. */
  cutsFrom: (sourceId: string) => ['cuts', 'from', sourceId] as const,
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
  // The files of one release, under the releases' prefix rather than the
  // work's files: they are not on the work's Files tab, and a release deleted
  // or brought back takes its files with it.
  releaseAssets: (releaseId: string) => ['releases', 'assets', releaseId] as const,
  /** What an answer or an agent proposes a release goes out under, waiting
   *  beside what is written. */
  releaseProposalsFor: (releaseId: string) => ['releases', 'proposals', releaseId] as const,
  calendar: ['calendar'] as const,
  releaseQueue: ['releaseQueue'] as const,
  /** What a day holds for a release being carried over it. */
  slotPreview: (releaseId: string, day: string) => ['slotPreview', releaseId, day] as const,

  collections: ['collections'] as const,
  // One key for every work's cover: attaching one changes the catalogue,
  // the card, the calendar and the dashboard at once.
  covers: ['covers'] as const,
  assetsFor: (workId: string) => ['assets', workId] as const,
  notes: ['notes'] as const,
  /** A work's own notes, as its Notes tab lists them. */
  notesFor: (workId: string) => ['notes', workId] as const,
  /** The notes screen's list, under the filter it is read through. */
  notesMatching: (filter: object) => ['notes', 'all', filter] as const,
  /** Every note a scene may point at: the cast, the places, the lore. */
  notesCastable: ['notes', 'castable'] as const,
  // One prefix over the inbox, the channels and what waits to be kept:
  // keeping a comment changes all three at once. A work's counter is the
  // card's, under `cardCounts`.
  comments: ['comments'] as const,
  commentChannels: ['comments', 'channels'] as const,
  commentProposals: ['comments', 'proposals'] as const,
  /** The inbox, under the filter it is read through. */
  commentsMatching: (filter: object) => ['comments', 'list', filter] as const,
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
  /** The dictionary's list, narrowed to a type and a search. */
  styleBricksMatching: (typeKey: string | null, query: string) =>
    ['styles', 'list', typeKey, query] as const,
  /** The ready bricks a prompt may be built from, under a search. */
  readyStyleBricks: (query: string) => ['styles', 'list', 'ready', query] as const,
  styleReferences: (id: string) => ['styles', 'references', id] as const,
  /** The captions a dressing is filled from - they live on the channel's
      card, so the canon's prefix refreshes them too (see `refresh.canon`). */
  styleSlots: ['styles', 'slots'] as const,
  // One coarse prefix over the canon: a fact changes its card, the list's
  // counts, the timeline and where the card appears, and none of them is
  // worth invalidating alone.
  canon: ['canon'] as const,
  /** The list of cards, under the filter it is read through. */
  cardsMatching: (filter: object) => ['canon', 'cards', filter] as const,
  card: (id: string) => ['canon', 'card', id] as const,
  /** A card read through a task's eyes, as the prompt receives it. */
  cardAsSeen: (id: string, lens: string) => ['canon', 'seen', id, lens] as const,
  canonTimeline: (id: string | null) => ['canon', 'timeline', id] as const,
  /** Proposals for the canon waiting to be kept. Under the proposals' prefix:
   *  the bell's count moves with them. */
  canonProposals: ['proposals', 'canon'] as const,
  canonReview: (messageId: string) => ['proposals', 'canon', 'review', messageId] as const,
  tags: ['tags'] as const,
  // One prefix over the register (ADR 0044): a term changes the counts, the
  // checks of every text on screen and the list at once - and so does a
  // version, since the counts are read off the works' current texts.
  register: ['register'] as const,
  terms: ['register', 'terms'] as const,
  termUses: (id: string) => ['register', 'uses', id] as const,
  termTopics: ['register', 'topics'] as const,
  termPreview: (word: string, forms: readonly string[]) =>
    ['register', 'preview', word, forms] as const,
  /** A text checked against itself and the register, keyed by the text. */
  textCheck: (text: string) => ['register', 'check', text] as const,
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
  // The numbers beside a card's tabs, under the journal's prefix on purpose.
  // Each counts rows that only a write makes or takes away, every write is an
  // entry (ADR 0033), and every write refreshes the journal - so the counters
  // are refreshed with every write the card makes, from any tab, without a
  // list of which writes move which number. One cheap query refetched too
  // often is cheaper than a count left stale by the list that forgot it.
  cardCounts: (workId: string) => ['journal', 'counts', workId] as const,

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

  // Facts about this machine's install, asked once per screen that shows them.
  workspacePath: ['workspacePath'] as const,
  logPath: ['logPath'] as const,
  mcpRegistration: ['mcpRegistration'] as const,
  /** What a task would send, composed against what the person has picked. */
  taskPreview: (parts: readonly unknown[]) => ['task-preview', ...parts] as const,
} as const
