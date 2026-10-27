import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Plugin, PluginProgress, PluginReply } from '@/lib/api/types'

// Plugins: programs beside kilna that act on a work.

/** Where a plugin's command is offered: a release, a work's head, or a
 *  work's Cuts tab. */
export type PluginTarget = 'release' | 'work' | 'cuts'

export const listPlugins = () => invoke<Plugin[]>('list_plugins')

/** Run a command. `run` names this run, so its progress can be told from
 *  another's and it can be stopped (v0.101). */
export const runPlugin = (
  executable: string,
  command: string,
  target: PluginTarget,
  id: string,
  run: string,
) => invoke<PluginReply>('run_plugin', { executable, command, target, id, run })

/** Stop a run before it finishes. */
export const cancelPlugin = (run: string) => invoke<void>('cancel_plugin', { run })

/** Hear every run's progress, as the plugins write it. */
export const onPluginProgress = (
  handler: (progress: PluginProgress) => void,
): Promise<UnlistenFn> =>
  listen<PluginProgress>('plugin-progress', (event) => handler(event.payload))
