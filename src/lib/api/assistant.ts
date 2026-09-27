import { invoke } from '@tauri-apps/api/core'
import type {
  Applied,
  Availability,
  Chat,
  ChatSummary,
  ComposedTask,
  PendingProposal,
  ProposalOverrides,
  Run,
  StartedBatch,
  StartedTask,
  TaskAbout,
  TaskQueue,
  Transcript,
} from '@/lib/api/types'

// The assistant: chats, runs, tasks and what their answers propose.

export const assistantStatus = () => invoke<Availability>('assistant_status')
export const listChatSummaries = (workId?: string) =>
  invoke<ChatSummary[]>('list_chat_summaries', { workId })
export const createChat = (chat: { work_id?: string | null; title?: string | null }) =>
  invoke<Chat>('create_chat', { chat })
export const renameChat = (id: string, title: string | null) =>
  invoke<void>('rename_chat', { id, title })
export const getTranscript = (chatId: string) =>
  invoke<Transcript | null>('get_transcript', { chatId })
export const deleteChat = (id: string) => invoke<void>('delete_chat', { id })
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
export const startTask = (workId: string, action: string, about: TaskAbout = {}) =>
  invoke<StartedTask>('start_task', { workId, action, about })
/** What starting the action would send, without sending it. */
export const previewTask = (workId: string, action: string, about: TaskAbout = {}) =>
  invoke<ComposedTask>('preview_task', { workId, action, about })
export const startTasks = (workIds: readonly string[], action: string) =>
  invoke<StartedBatch>('start_tasks', { workIds, action })
export const activeTasks = () => invoke<string[]>('active_tasks')
export const taskQueue = () => invoke<TaskQueue>('task_queue')
export const clearTaskQueue = () => invoke<number>('clear_task_queue')
export const waitingChats = () => invoke<ChatSummary[]>('waiting_chats')
export const clearWaiting = (chatId: string) => invoke<void>('clear_waiting', { chatId })
export const renderPrompt = (workId: string, template: string) =>
  invoke<string>('render_prompt', { workId, template })
