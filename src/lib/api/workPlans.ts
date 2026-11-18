import { invoke } from '@tauri-apps/api/core'
import type { Fulfilled, NewWorkPlan, WorkPlan, WorkPlanPatch } from '@/lib/api/types'

// The planned slots of works (v0.138, FR-11).

export const workPlans = () => invoke<WorkPlan[]>('work_plans')
export const planWork = (plan: NewWorkPlan) => invoke<WorkPlan>('plan_work', { plan })
export const updateWorkPlan = (id: string, patch: WorkPlanPatch) =>
  invoke<WorkPlan>('update_work_plan', { id, patch })
export const deleteWorkPlan = (id: string) => invoke<null>('delete_work_plan', { id })
export const fulfillWorkPlan = (id: string, kind: string | null) =>
  invoke<Fulfilled>('fulfill_work_plan', { id, kind })
