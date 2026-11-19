import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  BuildSources,
  JsonValue,
  Plugin,
  PluginBatchProgress,
  PluginBatchReport,
  PluginCandidate,
  PluginMonthQuota,
  PluginProgress,
  PluginReply,
} from '@/lib/api/types'

// Plugins: programs beside kilna that act on a work.

/** Where a plugin's command is offered: a release, a work's head, a work's Cuts
 *  tab, its Score tab, or its Cover tab (v0.158). */
export type PluginTarget = 'release' | 'work' | 'cuts' | 'score' | 'cover'

export const listPlugins = () => invoke<Plugin[]>('list_plugins')

/** Where a plugin is put to be found; made if it is not there yet. */
export const pluginsFolder = () => invoke<string>('plugins_folder')

/** Turn a plugin on or off. What was set for it stays. */
export const setPluginEnabled = (executable: string, enabled: boolean) =>
  invoke<void>('set_plugin_enabled', { executable, enabled })

/** Set one setting of a plugin; null puts it back to the plugin's default. */
export const setPluginSetting = (executable: string, key: string, value: JsonValue | null) =>
  invoke<void>('set_plugin_setting', { executable, key, value })

/** Run a command. `run` names this run, so its progress can be told from
 *  another's and it can be stopped (v0.101). */
export const runPlugin = (
  executable: string,
  command: string,
  target: PluginTarget,
  id: string,
  run: string,
) => invoke<PluginReply>('run_plugin', { executable, command, target, id, run })

/** What a work's simple video is built from: its picture and its track (v0.149). */
export const buildSources = (workId: string) => invoke<BuildSources>('build_sources', { workId })

/** Stop a run before it finishes. */
export const cancelPlugin = (run: string) => invoke<void>('cancel_plugin', { run })

/** Hear every run's progress, as the plugins write it. */
export const onPluginProgress = (
  handler: (progress: PluginProgress) => void,
): Promise<UnlistenFn> =>
  listen<PluginProgress>('plugin-progress', (event) => handler(event.payload))

/** One of the results a command offered, picked and kept (v0.155). */
export const keepPluginCandidate = (workId: string, candidate: PluginCandidate) =>
  invoke<string | null>('keep_plugin_candidate', { workId, candidate })

/** One batch command over many works at once (v0.155): up to `parallelism`
 *  runs at a time, its progress under `batch`, stopped by `cancelPlugin`. */
export const runPluginBatch = (
  executable: string,
  command: string,
  ids: readonly string[],
  parallelism: number,
  batch: string,
) => invoke<PluginBatchReport>('run_plugin_batch', { executable, command, ids, parallelism, batch })

/** Hear how far every batch is (v0.155). */
export const onPluginBatch = (
  handler: (progress: PluginBatchProgress) => void,
): Promise<UnlistenFn> =>
  listen<PluginBatchProgress>('plugin-batch', (event) => handler(event.payload))

/** What each plugin's service was spent this month, by executable (v0.158). */
export const pluginQuotas = () => invoke<Record<string, PluginMonthQuota>>('plugin_quotas')
