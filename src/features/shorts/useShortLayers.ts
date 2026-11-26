import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { stageShortLayers } from '@/lib/api/cuts'
import { queries } from '@/lib/query/queries'
import { draw, layersOf } from '@/lib/shorts/layers'

/**
 * Lays the still layers of a short where its builder is handed them
 * (v0.168): the look the short wears now is read fresh, drawn in the window
 * and written as pictures. Called before every run of a command on the Cuts
 * target - a build always wears what the style says at that moment.
 */
export function useShortLayers(workId: string): () => Promise<void> {
  const client = useQueryClient()
  return useCallback(async () => {
    const view = await client.fetchQuery({ ...queries.short(workId), staleTime: 0 })
    const { placed, layers } = await layersOf(await draw(view.look, view.title))
    await stageShortLayers(workId, placed, layers)
  }, [client, workId])
}
