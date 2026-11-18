import { invoke } from '@tauri-apps/api/core'
import type { Installed, OpenedPack, PackCatalogue, PackSource } from '@/lib/api/types'

// Packs of styles from the registries this machine reads (v0.131, FR-06).

export const stylePacks = () => invoke<PackCatalogue>('style_packs')
export const stylePack = (source: PackSource, pack: string) =>
  invoke<OpenedPack>('style_pack', { source, pack })
export const installStylePack = (source: PackSource, pack: string, only: string[] | null) =>
  invoke<Installed>('install_style_pack', { source, pack, only })
export const addStyleSource = (source: PackSource) => invoke<null>('add_style_source', { source })
export const removeStyleSource = (source: PackSource) =>
  invoke<null>('remove_style_source', { source })
export const setPacksOffline = (offline: boolean) => invoke<null>('set_packs_offline', { offline })
