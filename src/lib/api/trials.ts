import { invoke } from '@tauri-apps/api/core'
import type {
  ComposedTask,
  MadeExperiment,
  NewStyleBrick,
  NewTrial,
  StartedTask,
  StyleBrick,
  Trial,
  TrialBoard,
  TrialPatch,
  TrialRequest,
  TrialVerdict,
  Version,
  Work,
} from '@/lib/api/types'

// An experiment's board of trials (v0.95, ADR 0061).

/** Every trial of an experiment, with what each became. */
export const trialBoard = (id: string) => invoke<TrialBoard>('trial_board', { id })
/** A trial on the board: empty, written from the dictionary, or composed. */
export const createTrial = (trial: NewTrial) => invoke<Trial>('create_trial', { trial })
export const updateTrial = (id: string, patch: TrialPatch) =>
  invoke<Trial>('update_trial', { id, patch })
/** Keep a trial, drop it, or take the verdict back with `null`. */
export const judgeTrial = (id: string, verdict: TrialVerdict | null) =>
  invoke<Trial>('judge_trial', { id, verdict })
/** A copy of a trial as its child, with what is to move. */
export const varyTrial = (id: string, angle: string, series: string | null = null) =>
  invoke<Trial>('vary_trial', { id, angle, series })
/** A work's text in the role the lab keeps trials in, put on the board. */
export const trialFromVersion = (workId: string, versionId: string, series: string) =>
  invoke<Trial>('trial_from_version', { workId, versionId, series })
/** Off the board, into the trash. */
export const deleteTrial = (id: string) => invoke<string>('delete_trial', { id })

/** A kept trial into a work, beside its current text unless asked. */
export const harvestTrial = (
  id: string,
  workId: string,
  options: { label?: string | null; makeCurrent?: boolean } = {},
) =>
  invoke<Version>('harvest_trial', {
    id,
    workId,
    label: options.label ?? null,
    makeCurrent: options.makeCurrent ?? false,
  })
/** A phrase of the dictionary cut out of a kept trial. */
export const harvestTrialPhrase = (id: string, brick: NewStyleBrick) =>
  invoke<StyleBrick>('harvest_trial_phrase', { id, brick })
/** A new work whose first text is the kept trial's. */
export const harvestTrialWork = (id: string, kind: string, title: string) =>
  invoke<Work>('harvest_trial_work', { id, kind, title })

/** An experiment that reworks a work: its newest text becomes the first trial. */
export const makeExperiment = (sourceId: string, title: string, series: string) =>
  invoke<MadeExperiment>('make_experiment', { sourceId, title, series })

/** Ask for trials in the background; they land on the board as they come. */
export const startLabTask = (id: string, action: string, request: TrialRequest) =>
  invoke<StartedTask>('start_lab_task', { id, action, request })
export const previewLabTask = (id: string, action: string, request: TrialRequest) =>
  invoke<ComposedTask>('preview_lab_task', { id, action, request })
