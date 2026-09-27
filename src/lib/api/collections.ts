import { invoke } from '@tauri-apps/api/core'
import type { Collection } from '@/lib/api/types'

// Collections: albums, books, seasons.

export const listCollections = () => invoke<Collection[]>('list_collections')
