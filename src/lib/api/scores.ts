import { invoke } from '@tauri-apps/api/core'
import type { KindVerdict, NewScore, Score } from '@/lib/api/types'

// Scores: snapshots of a judgement, never overwritten.

export const scoreWork = (workId: string, score: NewScore) =>
  invoke<Score>('score_work', { workId, score })
export const scoreHistory = (workId: string) => invoke<Score[]>('score_history', { workId })
export const latestScore = (workId: string) => invoke<Score | null>('latest_score', { workId })
/** The same score, read down every channel the craft ships to. */
export const kindVerdicts = (workId: string) => invoke<KindVerdict[]>('kind_verdicts', { workId })
export const deleteScore = (id: string) => invoke<string>('delete_score', { id })
