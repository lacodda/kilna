import { invoke } from '@tauri-apps/api/core'
import type { Plugin } from '@/lib/api/types'

// Plugins: programs beside kilna that act on a work.

export const listPlugins = () => invoke<Plugin[]>('list_plugins')
export const runPlugin = (
  executable: string,
  command: string,
  target: 'release' | 'work',
  id: string,
) => invoke<string | null>('run_plugin', { executable, command, target, id })
