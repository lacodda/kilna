import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type { Asset, NewAsset } from '@/lib/api/types'

// Files kept in the workspace, and the covers among them.

export const attachAsset = (source: string, asset: NewAsset) =>
  invoke<Asset>('attach_asset', { source, asset })
export const listWorkAssets = (workId: string) => invoke<Asset[]>('list_work_assets', { workId })
/** The files attached to one release - its thumbnail, its subtitles - oldest first. */
export const listReleaseAssets = (releaseId: string) =>
  invoke<Asset[]>('list_release_assets', { releaseId })
/** The cover of every work that has one, as [work id, path] pairs. */
export const listCovers = () => invoke<[string, string][]>('list_covers')
export const detachAsset = (id: string) => invoke<void>('detach_asset', { id })
/** A picture pasted straight onto a work - a candidate for its cover. */
export const pasteAsset = (bytes: Uint8Array, name: string, asset: NewAsset) =>
  invoke<Asset>('paste_asset', { bytes: Array.from(bytes), name, asset })
/** Make a picture the work's cover: the final one, the release's preview. */
export const chooseCover = (id: string) => invoke<Asset>('choose_cover', { id })
/** A file's bytes, for the window to draw on without tainting a canvas. */
export const assetBytes = (id: string) => invoke<ArrayBuffer>('asset_bytes', { id })
/** Write a picture the window composed to a place the person chose. */
export const savePicture = (path: string, bytes: Uint8Array) =>
  invoke<void>('save_picture', { path, bytes: Array.from(bytes) })
/** A file in the workspace, as a URL the window may fetch.

    A path on disk is not one: the webview reaches a local file through the
    asset protocol, whose scope is granted at startup for the workspace's own
    files directory and nothing else. */
export const fileSrc = (path: string) => convertFileSrc(path)
