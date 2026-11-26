import { planPeriod, workPlans } from '@/lib/api/workPlans'
import { stylePacks } from '@/lib/api/packs'
import { queryOptions } from '@tanstack/react-query'
import i18n from '@/i18n'
import { listReleaseAssets, listWorkAssets, listCovers } from '@/lib/api/assets'
import { mediaDirectory, mediaRoot, workFolder } from '@/lib/api/folders'
import {
  activeRuns,
  activeTasks,
  assistantSpend,
  assistantSettings,
  assistantStatus,
  runSight,
  waitingScores,
  getTranscript,
  listChatSummaries,
  listRuns,
  pendingProposals,
  previewTask,
  starredAnswers,
  taskQueue,
  waitingChats,
} from '@/lib/api/assistant'
import {
  canonLinks,
  canonTimeline,
  checkQuotas,
  cardAsSeen,
  listCards,
  pendingCanonProposals,
  previewCardTask,
  readCard,
  reviewCanonProposal,
} from '@/lib/api/canon'
import { listCollections } from '@/lib/api/collections'
import {
  commentChannels,
  listComments,
  pendingCommentProposals,
  previewCommentTask,
} from '@/lib/api/comments'
import {
  cutParts,
  cutShotList,
  listCuts,
  listCutsFrom,
  shortStyleDefault,
  shortView,
} from '@/lib/api/cuts'
import { canExportPackage, logPath, surveyIntake, workspacePath } from '@/lib/api/data'
import { dismissedFindings, listFocusNotes } from '@/lib/api/focus'
import { historyKeeping, keptBackups, storageReport } from '@/lib/api/upkeep'
import { listJournal, sinceVisit, unreadJournal } from '@/lib/api/journal'
import { listLinks, listPublications, resolveLinks } from '@/lib/api/links'
import { listLines, listNotes, listShelves, listTags } from '@/lib/api/notes'
import {
  checkText,
  listTerms,
  listTermTopics,
  previewTerm,
  repeatMarks,
  termUses,
  wordsFromTexts,
  workRepeats,
} from '@/lib/api/register'
import { buildSources, listPlugins, pluginsFolder } from '@/lib/api/plugins'
import {
  calendar,
  previewLanding,
  releaseFields,
  releaseProposals,
  releaseQueue,
  releasesForWork,
  type Landing,
} from '@/lib/api/releases'
import { listTimeOff } from '@/lib/api/timeOff'
import { listSceneFrames, listSceneNotes, listScenes, sceneFrameView } from '@/lib/api/scenes'
import { kindVerdicts, latestScore, scoreHistory } from '@/lib/api/scores'
import { search, worksMatching } from '@/lib/api/search'
import {
  listStyleBricks,
  previewStyleTask,
  styleBrickCounts,
  styleBrickReferences,
  styleSlotValues,
  houseStyles,
  composeText,
  phrasesFromTexts,
} from '@/lib/api/styles'
import { coverBoard } from '@/lib/api/ideas'
import { trialBoard } from '@/lib/api/trials'
import { workBoards } from '@/lib/api/boards'
import { listDeletions } from '@/lib/api/trash'
import type {
  CardFilter,
  CommentFilter,
  IntakeRequest,
  Lens,
  NoteFilter,
  QuotaSubject,
  TaskAbout,
} from '@/lib/api/types'
import { getVersion, listVersions } from '@/lib/api/versions'
import {
  cardCounts,
  catalogue,
  coverView,
  frameView,
  getWork,
  listWorks,
  workTags,
  workTimeline,
  worksMedia,
} from '@/lib/api/works'
import { armedReceiver, receiverSettings } from '@/lib/api/receiver'
import { pluginQuotas } from '@/lib/api/plugins'
import {
  getWorkspace,
  mcpRegistration,
  shippedOffers,
  shippedStanding,
  skillTriggers,
} from '@/lib/api/workspace'
import { currentSpace, listSpaceTypes, listSpaces } from '@/lib/api/spaces'
import { activity } from '@/lib/api/shell'
import { keys } from '@/lib/query/keys'

/**
 * Every question the window asks the backend, with the key its answer is kept
 * under.
 *
 * A key and the function that answers it are one thing: written apart at
 * each call, the same list was read under two keys (`[...keys.links, id]`
 * beside `keys.linksFor(id)`), and a key could be invalidated while the query
 * that mattered sat under another. A screen asks `useQuery(queries.work(id))`,
 * and adds only what is its own - `enabled`, `select`, a refetch interval.
 */
export const queries = {
  workspace: () => queryOptions({ queryKey: keys.workspace, queryFn: getWorkspace }),
  skillTriggers: () => queryOptions({ queryKey: keys.skillTriggers, queryFn: skillTriggers }),
  stylePacks: () => queryOptions({ queryKey: keys.stylePacks, queryFn: stylePacks }),
  workPlans: () => queryOptions({ queryKey: keys.workPlans, queryFn: workPlans }),
  periodPlan: (from: string, months: number) =>
    queryOptions({
      queryKey: keys.periodPlan(from, months),
      queryFn: () => planPeriod(from, months),
    }),
  shippedStanding: (profileId: string, collection: string) =>
    queryOptions({
      queryKey: keys.shippedStanding(profileId, collection),
      queryFn: () => shippedStanding(profileId, collection),
    }),
  shippedOffers: (profileId: string) =>
    queryOptions({
      queryKey: keys.shippedOffers(profileId),
      queryFn: () => shippedOffers(profileId),
    }),
  spaces: () => queryOptions({ queryKey: keys.spaces, queryFn: listSpaces }),
  currentSpace: () => queryOptions({ queryKey: keys.currentSpace, queryFn: currentSpace }),
  spaceTypes: () => queryOptions({ queryKey: keys.spaceTypes, queryFn: listSpaceTypes }),

  works: () => queryOptions({ queryKey: keys.works, queryFn: () => listWorks() }),
  work: (id: string) => queryOptions({ queryKey: keys.work(id), queryFn: () => getWork(id) }),
  cardCounts: (workId: string) =>
    queryOptions({ queryKey: keys.cardCounts(workId), queryFn: () => cardCounts(workId) }),
  timeline: (workId: string) =>
    queryOptions({ queryKey: keys.timeline(workId), queryFn: () => workTimeline(workId) }),
  publications: (workId: string) =>
    queryOptions({
      queryKey: keys.publicationsFor(workId),
      queryFn: () => listPublications(workId),
    }),
  coverView: (workId: string, format: string | null) =>
    queryOptions({
      queryKey: keys.coverView(workId, format),
      queryFn: () => coverView(workId, format),
    }),
  coverBoard: (workId: string) =>
    queryOptions({ queryKey: keys.coverBoard(workId), queryFn: () => coverBoard(workId) }),
  trialBoard: (workId: string) =>
    queryOptions({ queryKey: keys.trialBoard(workId), queryFn: () => trialBoard(workId) }),
  boards: (workId: string) =>
    queryOptions({ queryKey: keys.boards(workId), queryFn: () => workBoards(workId) }),
  frameView: (workId: string) =>
    queryOptions({ queryKey: keys.frameView(workId), queryFn: () => frameView(workId) }),
  sceneFrameView: (sceneId: string) =>
    queryOptions({
      queryKey: keys.sceneFrameView(sceneId),
      queryFn: () => sceneFrameView(sceneId),
    }),
  catalogue: () => queryOptions({ queryKey: keys.catalogue, queryFn: catalogue }),
  worksMedia: () => queryOptions({ queryKey: keys.worksMedia, queryFn: worksMedia }),
  workTags: () => queryOptions({ queryKey: keys.workTags, queryFn: workTags }),
  collections: () => queryOptions({ queryKey: keys.collections, queryFn: listCollections }),
  covers: () => queryOptions({ queryKey: keys.covers, queryFn: listCovers }),
  assets: (workId: string) =>
    queryOptions({ queryKey: keys.assetsFor(workId), queryFn: () => listWorkAssets(workId) }),
  /** A work's folder on disk. Looked at again whenever the window comes
      back into focus: the files in it are written by other programs - a
      render finishing, a take exported - and returning to kilna is the
      moment a person expects to see them. */
  folder: (workId: string) =>
    queryOptions({
      queryKey: keys.folderFor(workId),
      queryFn: () => workFolder(workId),
      refetchOnWindowFocus: 'always',
    }),
  mediaRoot: () => queryOptions({ queryKey: keys.mediaRoot, queryFn: mediaRoot }),
  /** Never changes while the app runs. */
  mediaDirectory: () =>
    queryOptions({ queryKey: keys.mediaDirectory, queryFn: mediaDirectory, staleTime: Infinity }),

  search: (query: string) =>
    queryOptions({ queryKey: keys.search(query), queryFn: () => search(query) }),
  worksMatching: (query: string) =>
    queryOptions({ queryKey: keys.worksMatching(query), queryFn: () => worksMatching(query) }),

  versions: (workId: string) =>
    queryOptions({ queryKey: keys.versions(workId), queryFn: () => listVersions(workId) }),
  version: (id: string) =>
    queryOptions({ queryKey: keys.version(id), queryFn: () => getVersion(id) }),

  links: (workId: string) =>
    queryOptions({ queryKey: keys.linksFor(workId), queryFn: () => listLinks(workId) }),
  resolvedLinks: (works: string[], versions: string[]) =>
    queryOptions({
      queryKey: keys.resolvedLinks(`${works.join(',')}|${versions.join(',')}`),
      queryFn: () => resolveLinks(works, versions),
    }),

  scenes: (workId: string) =>
    queryOptions({ queryKey: keys.scenesFor(workId), queryFn: () => listScenes(workId) }),
  sceneNotes: (workId: string) =>
    queryOptions({ queryKey: keys.sceneNotesFor(workId), queryFn: () => listSceneNotes(workId) }),
  sceneFrames: (workId: string) =>
    queryOptions({ queryKey: keys.sceneFramesFor(workId), queryFn: () => listSceneFrames(workId) }),
  // Asked afresh whenever the board is opened: the other half of the answer
  // is what a release goes out as, written on another tab under another
  // prefix, and a button hanging on a stale no would stay hidden after it.
  canExportPackage: (workId: string) =>
    queryOptions({
      queryKey: keys.scenePackageFor(workId),
      queryFn: () => canExportPackage(workId),
      staleTime: 0,
    }),

  cuts: (workId: string) =>
    queryOptions({ queryKey: keys.cutsFor(workId), queryFn: () => listCuts(workId) }),
  cutsFrom: (sourceId: string) =>
    queryOptions({ queryKey: keys.cutsFrom(sourceId), queryFn: () => listCutsFrom(sourceId) }),
  shots: (workId: string) =>
    queryOptions({ queryKey: keys.shotsFor(workId), queryFn: () => cutShotList(workId) }),
  parts: (workId: string) =>
    queryOptions({ queryKey: keys.partsFor(workId), queryFn: () => cutParts(workId) }),
  short: (workId: string) =>
    queryOptions({ queryKey: keys.shortFor(workId), queryFn: () => shortView(workId) }),
  shortStyleDefault: () =>
    queryOptions({
      queryKey: keys.shortStyleDefault,
      queryFn: shortStyleDefault,
      staleTime: Infinity,
    }),

  scoreHistory: (workId: string) =>
    queryOptions({ queryKey: keys.scoreHistory(workId), queryFn: () => scoreHistory(workId) }),
  latestScore: (workId: string) =>
    queryOptions({ queryKey: keys.latestScore(workId), queryFn: () => latestScore(workId) }),
  kindVerdicts: (workId: string) =>
    queryOptions({ queryKey: keys.kindVerdicts(workId), queryFn: () => kindVerdicts(workId) }),

  calendar: () => queryOptions({ queryKey: keys.calendar, queryFn: calendar }),
  releaseQueue: () => queryOptions({ queryKey: keys.releaseQueue, queryFn: releaseQueue }),
  timeOff: () => queryOptions({ queryKey: keys.timeOff, queryFn: listTimeOff }),
  releasesForWork: (workId: string) =>
    queryOptions({
      queryKey: keys.releasesForWork(workId),
      queryFn: () => releasesForWork(workId),
    }),
  releaseFields: (releaseId: string) =>
    queryOptions({
      queryKey: keys.releaseFields(releaseId),
      queryFn: () => releaseFields(releaseId),
    }),
  releaseProposals: (releaseId: string) =>
    queryOptions({
      queryKey: keys.releaseProposalsFor(releaseId),
      queryFn: () => releaseProposals(releaseId),
    }),
  releaseAssets: (releaseId: string) =>
    queryOptions({
      queryKey: keys.releaseAssets(releaseId),
      queryFn: () => listReleaseAssets(releaseId),
    }),
  slotPreview: (landing: Landing, day: string) =>
    queryOptions({
      queryKey:
        'work' in landing
          ? keys.slotPreview('work', landing.work, day)
          : keys.slotPreview('release', landing.release, day),
      queryFn: () => previewLanding(landing, day),
    }),

  notesFor: (workId: string) =>
    queryOptions({
      queryKey: keys.notesFor(workId),
      queryFn: () => listNotes({ work_id: workId }),
    }),
  notesMatching: (filter: NoteFilter) =>
    queryOptions({ queryKey: keys.notesMatching(filter), queryFn: () => listNotes(filter) }),
  notesCastable: () => queryOptions({ queryKey: keys.notesCastable, queryFn: () => listNotes() }),
  tags: () => queryOptions({ queryKey: keys.tags, queryFn: listTags }),

  terms: () => queryOptions({ queryKey: keys.terms, queryFn: listTerms }),
  termUses: (id: string) =>
    queryOptions({ queryKey: keys.termUses(id), queryFn: () => termUses(id) }),
  termTopics: () => queryOptions({ queryKey: keys.termTopics, queryFn: listTermTopics }),
  termPreview: (word: string, forms: readonly string[]) =>
    queryOptions({
      queryKey: keys.termPreview(word, forms),
      queryFn: () => previewTerm(word, [...forms]),
    }),
  textCheck: (
    text: string,
    sung = false,
    workId: string | null = null,
    role: string | null = null,
  ) =>
    queryOptions({
      queryKey: keys.textCheck(text, sung, workId, role),
      queryFn: () => checkText(text, sung, workId ?? undefined, role ?? undefined),
      // A text is typed past in a moment and never asked about again.
      gcTime: 30_000,
    }),
  shelves: () => queryOptions({ queryKey: keys.shelves, queryFn: listShelves }),
  lines: (filter: NoteFilter) =>
    queryOptions({ queryKey: keys.lines(filter), queryFn: () => listLines(filter) }),
  wordsFromTexts: () =>
    queryOptions({ queryKey: keys.wordsFromTexts, queryFn: wordsFromTexts, staleTime: 0 }),
  repeatMarks: (today: string) =>
    queryOptions({ queryKey: keys.repeatMarks(today), queryFn: () => repeatMarks(today) }),
  workRepeats: (workId: string, today: string) =>
    queryOptions({
      queryKey: keys.workRepeats(workId, today),
      queryFn: () => workRepeats(workId, today),
    }),

  cardsMatching: (filter: CardFilter) =>
    queryOptions({ queryKey: keys.cardsMatching(filter), queryFn: () => listCards(filter) }),
  canonLinks: () => queryOptions({ queryKey: keys.canonLinks, queryFn: canonLinks }),
  card: (id: string) => queryOptions({ queryKey: keys.card(id), queryFn: () => readCard(id) }),
  quotas: (subject: QuotaSubject) =>
    queryOptions({
      queryKey: keys.quotas(subject),
      queryFn: () => checkQuotas(subject),
      // Asked when a report opens: a release marked since, or a field
      // written, is measured anew rather than read from before.
      staleTime: 0,
    }),
  cardAsSeen: (id: string, lens: Lens) =>
    queryOptions({ queryKey: keys.cardAsSeen(id, lens), queryFn: () => cardAsSeen(id, lens) }),
  canonTimeline: (id: string | null) =>
    queryOptions({
      queryKey: keys.canonTimeline(id),
      queryFn: () => canonTimeline(id ?? undefined),
    }),
  canonProposals: () =>
    queryOptions({
      queryKey: keys.canonProposals,
      queryFn: () => pendingCanonProposals(i18n.resolvedLanguage ?? 'en'),
    }),
  canonReview: (messageId: string) =>
    queryOptions({
      queryKey: keys.canonReview(messageId),
      queryFn: () => reviewCanonProposal(messageId),
    }),

  commentsMatching: (filter: CommentFilter) =>
    queryOptions({ queryKey: keys.commentsMatching(filter), queryFn: () => listComments(filter) }),
  commentChannels: () => queryOptions({ queryKey: keys.commentChannels, queryFn: commentChannels }),
  commentProposals: () =>
    queryOptions({ queryKey: keys.commentProposals, queryFn: pendingCommentProposals }),

  styleBricksMatching: (typeKey: string | null, query: string) =>
    queryOptions({
      queryKey: keys.styleBricksMatching(typeKey, query),
      queryFn: () =>
        listStyleBricks({
          type_key: typeKey ?? undefined,
          query: query === '' ? undefined : query,
        }),
    }),
  readyStyleBricks: (query: string) =>
    queryOptions({
      queryKey: keys.readyStyleBricks(query),
      queryFn: () => listStyleBricks({ ready_only: true, query: query === '' ? undefined : query }),
    }),
  styleCounts: () => queryOptions({ queryKey: keys.styleCounts, queryFn: styleBrickCounts }),
  styleReferences: (id: string) =>
    queryOptions({ queryKey: keys.styleReferences(id), queryFn: () => styleBrickReferences(id) }),
  styleSlots: () => queryOptions({ queryKey: keys.styleSlots, queryFn: styleSlotValues }),
  houseStyles: () => queryOptions({ queryKey: keys.houseStyles, queryFn: houseStyles }),
  composedText: (composition: string, bricks: readonly string[], workId: string | null) =>
    queryOptions({
      queryKey: keys.composedText(composition, bricks, workId),
      queryFn: () =>
        composeText({ composition, bricks: [...bricks], work_id: workId ?? undefined }),
    }),
  phrasesFromTexts: (composition: string) =>
    queryOptions({
      queryKey: keys.phrasesFromTexts(composition),
      queryFn: () => phrasesFromTexts(composition),
      // Read afresh on every opening: a style written a minute ago is what
      // the person opened it to see.
      staleTime: 0,
    }),

  journalFeed: () => queryOptions({ queryKey: keys.journalFeed, queryFn: listJournal }),
  sinceVisit: () => queryOptions({ queryKey: keys.sinceVisit, queryFn: sinceVisit }),
  journalUnread: () => queryOptions({ queryKey: keys.journalUnread, queryFn: unreadJournal }),
  deletions: () => queryOptions({ queryKey: keys.deletions, queryFn: listDeletions }),

  dismissals: () => queryOptions({ queryKey: keys.dismissals, queryFn: dismissedFindings }),
  focusNotes: () => queryOptions({ queryKey: keys.focusNotes, queryFn: listFocusNotes }),

  assistantStatus: () => queryOptions({ queryKey: keys.assistantStatus, queryFn: assistantStatus }),
  assistantSettings: () =>
    queryOptions({ queryKey: keys.assistantSettings, queryFn: assistantSettings }),
  runSight: (workId: string) =>
    queryOptions({ queryKey: keys.runSight(workId), queryFn: () => runSight(workId) }),
  waitingScores: (workId: string) =>
    queryOptions({ queryKey: keys.waitingScores(workId), queryFn: () => waitingScores(workId) }),
  chats: (workId?: string) =>
    queryOptions({ queryKey: keys.chats(workId), queryFn: () => listChatSummaries(workId) }),
  // Keyed by the window's language too: a proposal's body is said in it, and
  // a window that changes language reads the chat again. Everything that
  // refreshes a transcript names it by its first two parts, as before.
  transcript: (chatId: string, language: string = i18n.resolvedLanguage ?? 'en') =>
    queryOptions({
      queryKey: [...keys.transcript(chatId), language] as const,
      queryFn: () => getTranscript(chatId, language),
    }),
  runs: (chatId: string) =>
    queryOptions({ queryKey: keys.runs(chatId), queryFn: () => listRuns(chatId) }),
  activeRuns: () => queryOptions({ queryKey: keys.activeRuns, queryFn: activeRuns }),
  activeTasks: () => queryOptions({ queryKey: keys.activeTasks, queryFn: activeTasks }),
  taskQueue: () => queryOptions({ queryKey: keys.taskQueue, queryFn: taskQueue }),
  armedReceiver: () => queryOptions({ queryKey: keys.armedReceiver, queryFn: armedReceiver }),
  pluginQuotas: () => queryOptions({ queryKey: keys.pluginQuotas, queryFn: pluginQuotas }),
  receiverSettings: () =>
    queryOptions({ queryKey: keys.receiverSettings, queryFn: receiverSettings }),
  activity: () => queryOptions({ queryKey: keys.activity, queryFn: activity }),
  spend: () => queryOptions({ queryKey: keys.spend, queryFn: () => assistantSpend() }),
  waitingChats: () => queryOptions({ queryKey: keys.waitingChats, queryFn: waitingChats }),
  starred: (workId?: string) =>
    queryOptions({ queryKey: keys.starred(workId), queryFn: () => starredAnswers(workId) }),
  pendingProposals: () =>
    queryOptions({ queryKey: keys.pendingProposals, queryFn: pendingProposals }),
  taskPreview: (workId: string, action: string, about: TaskAbout) =>
    queryOptions({
      queryKey: keys.taskPreview([
        workId,
        action,
        about.versionId ?? '',
        about.sceneId ?? '',
        about.block ?? '',
        about.attachments ?? [],
      ]),
      queryFn: () => previewTask(workId, action, about),
    }),

  /** What describing a style would send, before it is sent. */
  styleTaskPreview: (id: string, action: string) =>
    queryOptions({
      queryKey: keys.taskPreview(['style', id, action]),
      queryFn: () => previewStyleTask(id, action),
    }),
  /** What an action on a card of the canon would send, before it is sent. */
  cardTaskPreview: (id: string, action: string) =>
    queryOptions({
      queryKey: keys.taskPreview(['card', id, action]),
      queryFn: () => previewCardTask(id, action),
    }),
  /** What drafting a reply would send, before it is sent. */
  commentTaskPreview: (id: string, action: string) =>
    queryOptions({
      queryKey: keys.taskPreview(['comment', id, action]),
      queryFn: () => previewCommentTask(id, action),
    }),

  plugins: () => queryOptions({ queryKey: keys.plugins, queryFn: listPlugins }),
  buildSources: (workId: string) =>
    queryOptions({ queryKey: keys.buildSources(workId), queryFn: () => buildSources(workId) }),
  pluginsFolder: () => queryOptions({ queryKey: keys.pluginsFolder, queryFn: pluginsFolder }),
  workspacePath: () => queryOptions({ queryKey: keys.workspacePath, queryFn: workspacePath }),
  // Read from the disk each time it is asked: a file chosen a moment ago may
  // have been saved again since.
  surveyIntake: (request: IntakeRequest) =>
    queryOptions({
      queryKey: keys.intake(request),
      queryFn: () => surveyIntake(request),
      staleTime: 0,
    }),
  logPath: () => queryOptions({ queryKey: keys.logPath, queryFn: logPath }),
  // Read from the disk each time the screen opens: a copy is taken at start,
  // and a size moves with everything that is written.
  keptBackups: () =>
    queryOptions({ queryKey: keys.keptBackups, queryFn: keptBackups, staleTime: 0 }),
  storage: () => queryOptions({ queryKey: keys.storage, queryFn: storageReport, staleTime: 0 }),
  historyKeeping: () => queryOptions({ queryKey: keys.historyKeeping, queryFn: historyKeeping }),
  mcpRegistration: () => queryOptions({ queryKey: keys.mcpRegistration, queryFn: mcpRegistration }),
}
