import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type { Asset, NewAsset } from '@/lib/api/types'

// Files kept in the workspace, and the covers among them.

export const attachAsset = (source: string, asset: NewAsset) =>
  invoke<Asset>('attach_asset', { source, asset })
export const listWorkAssets = (workId: string) => invoke<Asset[]>('list_work_assets', { workId })
/** The cover of every work that has one, as [work id, path] pairs. */
export const listCovers = () => invoke<[string, string][]>('list_covers')
export const detachAsset = (id: string) => invoke<void>('detach_asset', { id })
/** A file in the workspace, as a URL the window may fetch.

    A path on disk is not one: the webview reaches a local file through the
    asset protocol, whose scope is granted at startup for the workspace's own
    files directory and nothing else. */
export const fileSrc = (path: string) => convertFileSrc(path)
