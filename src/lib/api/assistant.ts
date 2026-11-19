import { invoke } from '@tauri-apps/api/core'
import type {
  Applied,
  AssistantSettings,
  AutoScore,
  Availability,
  CaughtUp,
  Chat,
  ChatSummary,
  ComposedTask,
  PendingProposal,
  ProposalOverrides,
  Run,
  RunSees,
  Spend,
  SpendCheck,
  StarredAnswer,
  StartedBatch,
  StartedTask,
  TaskAbout,
  TaskQueue,
  Transcript,
  WaitingScore,
} from '@/lib/api/types'
import { today } from '@/lib/month'
import { spendSince, type Since } from '@/lib/spend'

// The assistant: chats, runs, tasks and what their answers propose.

export const assistantStatus = () => invoke<Availability>('assistant_status')
/** What the person decided about the assistant on this workspace (v0.108). */
export const assistantSettings = () => invoke<AssistantSettings>('assistant_settings')
export const setParallelRuns = (runs: number) =>
  invoke<AssistantSettings>('set_parallel_runs', { runs })
/** How a text is scored by itself: whether, after how long a silence, from
    which stage, and whether what is said is kept at once (FR-01). */
export const setAutoScore = (wanted: AutoScore) =>
  invoke<AssistantSettings>('set_auto_score', { wanted })
/** Score every text of the open profile no score describes, now: how many
    joined the line, and how many were not ones to score. */
export const scoreUnscored = () => invoke<CaughtUp>('score_unscored')
/** The model a run starts with; null is what Claude Code itself is set to. */
export const setRunModel = (model: string | null) =>
  invoke<AssistantSettings>('set_run_model', { model })
export const setRunSees = (sees: RunSees) => invoke<AssistantSettings>('set_run_sees', { sees })
/** The paths a run on this work may read beyond its prompt, as the setting stands. */
export const runSight = (workId: string) => invoke<string[]>('run_sight', { workId })
/** The scores proposed for a work that wait for an answer. */
export const waitingScores = (workId: string) =>
  invoke<WaitingScore[]>('waiting_scores', { workId })
export const listChatSummaries = (workId?: string) =>
  invoke<ChatSummary[]>('list_chat_summaries', { workId })
export const createChat = (chat: { work_id?: string | null; title?: string | null }) =>
  invoke<Chat>('create_chat', { chat })
export const renameChat = (id: string, title: string | null) =>
  invoke<void>('rename_chat', { id, title })
/** A chat's messages. The language is the window's: what an agent proposed
 *  over MCP is shown with a body written out again from the proposal itself,
 *  in that language (v0.112). */
export const getTranscript = (chatId: string, locale: string) =>
  invoke<Transcript | null>('get_transcript', { chatId, locale })
export const deleteChat = (id: string) => invoke<void>('delete_chat', { id })
/** Star an answer, or take the star away; the moment it was starred, or null. */
export const starAnswer = (messageId: string, starred: boolean) =>
  invoke<string | null>('star_answer', { messageId, starred })
/** The starred answers, newest star first; a work narrows them to its chats. */
export const starredAnswers = (workId?: string) =>
  invoke<StarredAnswer[]>('starred_answers', { workId })
/** Apply what a message proposes — any kind — and mark the message. */
export const applyProposal = (messageId: string, overrides?: ProposalOverrides) =>
  invoke<Applied>('apply_proposal', { messageId, overrides: overrides ?? null })
/** Every proposal waiting for an answer, across every chat, oldest first. */
export const pendingProposals = () => invoke<PendingProposal[]>('pending_proposals')
/** Turn a proposal down: it stops waiting, and nothing is written. */
export const dismissProposal = (messageId: string) =>
  invoke<void>('dismiss_proposal', { messageId })
/** Apply every proposal in a chat nobody has applied yet, oldest first. */
export const applyPendingProposals = (chatId: string) =>
  invoke<Applied[]>('apply_pending_proposals', { chatId })
export const startRun = (chatId: string, prompt: string) =>
  invoke<Run>('start_run', { chatId, prompt })
export const cancelRun = (id: string) => invoke<void>('cancel_run', { id })
export const listRuns = (chatId: string) => invoke<Run[]>('list_runs', { chatId })
export const activeRuns = () => invoke<string[]>('active_runs')
/** Start an action on a work — on one of its versions when `versionId` is
    given: the template reads that version, and what the answer proposes is
    bound to it; on a scene of its board for a scene action. */
// Every prompt about a work is sent with the person's day: a template that
// reads `{week}` counts the week it falls in, and only the window knows the
// person's clock (v0.106).
export const startTask = (workId: string, action: string, about: TaskAbout = {}) =>
  invoke<StartedTask>('start_task', { workId, action, about: { ...about, today: today() } })
/** What starting the action would send, without sending it. */
export const previewTask = (workId: string, action: string, about: TaskAbout = {}) =>
  invoke<ComposedTask>('preview_task', { workId, action, about: { ...about, today: today() } })
export const startTasks = (workIds: readonly string[], action: string) =>
  invoke<StartedBatch>('start_tasks', { workIds, action, today: today() })
export const activeTasks = () => invoke<string[]>('active_tasks')
/** When each running task began, by its key (v0.163). */
export const activeTaskTimes = () => invoke<Record<string, string>>('active_task_times')
export const taskQueue = () => invoke<TaskQueue>('task_queue')
/** What runs cost since this window's midnight and its Monday, and the
 *  week's budget (v0.107). */
export const assistantSpend = (since: Since = spendSince()) =>
  invoke<Spend>('assistant_spend', since)
/** The same, writing the week's crossing of its budget into the history the
 *  first time it is seen; `crossed` says this call was that time. */
export const checkSpend = (since: Since = spendSince()) => invoke<SpendCheck>('check_spend', since)
/** Set what a week of runs may cost, or clear it with null. */
export const setWeeklyBudget = (budget: number | null) =>
  invoke<number | null>('set_weekly_budget', { budget })
export const clearTaskQueue = () => invoke<number>('clear_task_queue')
export const waitingChats = () => invoke<ChatSummary[]>('waiting_chats')
export const clearWaiting = (chatId: string) => invoke<void>('clear_waiting', { chatId })
export const renderPrompt = (workId: string, template: string) =>
  invoke<string>('render_prompt', { workId, template, today: today() })
