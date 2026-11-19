import { invoke } from '@tauri-apps/api/core'
import type {
  Fulfilled,
  NewWorkPlan,
  PeriodLayout,
  PeriodMade,
  PeriodPlan,
  WorkPlan,
  WorkPlanPatch,
} from '@/lib/api/types'

// The planned slots of works (v0.138, FR-11).

export const workPlans = () => invoke<WorkPlan[]>('work_plans')
export const planWork = (plan: NewWorkPlan) => invoke<WorkPlan>('plan_work', { plan })
export const updateWorkPlan = (id: string, patch: WorkPlanPatch) =>
  invoke<WorkPlan>('update_work_plan', { id, patch })
export const deleteWorkPlan = (id: string) => invoke<null>('delete_work_plan', { id })
// A period laid out ahead (v0.152, FR-20 20.3): its days by the rhythm, then
// its months' collections and its days' drafts made in one gesture.
export const planPeriod = (from: string, months: number) =>
  invoke<PeriodPlan>('plan_period', { from, months })
export const layOutPeriod = (layout: PeriodLayout) =>
  invoke<PeriodMade>('lay_out_period', { layout })
export const fulfillWorkPlan = (id: string, kind: string | null) =>
  invoke<Fulfilled>('fulfill_work_plan', { id, kind })
