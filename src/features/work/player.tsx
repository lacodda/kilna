import { createContext, use, useMemo, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Asset, WorkFolder } from '@/lib/api/types'
import { mediaKindOf, nameOf } from '@/lib/media'
import { queries } from '@/lib/query/queries'
import { AudioPlayer, type Track } from '@/components/AudioPlayer'

/*
 * The card's player (v0.93): the sounds of the work - the files attached to
 * it and the ones in its folder on disk - under one bar at the top of the
 * card, so that a song can be listened to while its text is read or its
 * score is given, on whichever tab that happens.
 *
 * Any work that has a sound gets the bar, whatever its craft: a podcast's
 * episode has a mix as surely as a song does, and a switch per profile would
 * be a setting standing in for a fact the files already state.
 */

/** A track, and when it was last written - what orders the list. */
interface Dated extends Track {
  at: string
}

/**
 * The sounds of a work, newest first: the take finished last is the one a
 * person opens the card to hear. Files in the folder say where they lie in
 * it; files attached say so.
 */
export function soundsOf(
  assets: Asset[] | undefined,
  folder: WorkFolder | undefined,
  attachedLabel: string,
): Track[] {
  const found: Dated[] = []
  for (const asset of assets ?? []) {
    if (mediaKindOf(asset.path) !== 'sound') continue
    found.push({
      path: asset.path,
      name: asset.original_name ?? asset.label ?? nameOf(asset.path),
      where: attachedLabel,
      at: asset.created_at,
    })
  }
  for (const file of folder?.files ?? []) {
    if (mediaKindOf(file.path) !== 'sound') continue
    const steps = file.relative.split('/')
    found.push({
      path: file.path,
      name: steps.at(-1) ?? file.relative,
      where: steps.slice(0, -1).join('/'),
      at: file.modified ?? '',
    })
  }
  // RFC 3339 instants written by one clock compare as text; a file whose
  // system said no time goes last.
  found.sort((a, b) => b.at.localeCompare(a.at))
  return found.map(({ path, name, where }) => ({ path, name, where }))
}

interface PlayerState {
  /** The track chosen, by path; null for the newest. */
  selected: string | null
  /** Bumped to say "play it now". */
  request: number
  /** Choose a track and play it - from a file's tile, say. */
  play: (path: string) => void
  select: (path: string) => void
}

const PlayerContext = createContext<PlayerState | null>(null)

/** Holds what the card's player plays, so a tab can ask it to play a file. */
export function WorkPlayerProvider({ children }: { children: ReactNode }) {
  const [selected, setSelected] = useState<string | null>(null)
  const [request, setRequest] = useState(0)
  const value = useMemo<PlayerState>(
    () => ({
      selected,
      request,
      play: (path) => {
        setSelected(path)
        setRequest((count) => count + 1)
      },
      select: setSelected,
    }),
    [selected, request],
  )
  return <PlayerContext value={value}>{children}</PlayerContext>
}

/** The card's player, for a tab that wants to start it; null outside a card. */
export function usePlayer(): PlayerState | null {
  return use(PlayerContext)
}

/** The bar itself: drawn only when the work has something to play. */
export function WorkPlayer({ workId }: { workId: string }) {
  const { t } = useTranslation()
  const player = usePlayer()
  const assets = useQuery(queries.assets(workId))
  const folder = useQuery(queries.folder(workId))
  const tracks = useMemo(
    () => soundsOf(assets.data, folder.data, t('player.attached')),
    [assets.data, folder.data, t],
  )

  const newest = tracks[0]
  if (newest === undefined || player === null) return null
  // The track chosen while it is still among them; the newest otherwise - a
  // file chosen and then deleted on disk leaves the bar on what is there.
  const selected =
    player.selected !== null && tracks.some((track) => track.path === player.selected)
      ? player.selected
      : newest.path

  return (
    <AudioPlayer
      tracks={tracks}
      selected={selected}
      onSelect={player.select}
      request={player.request}
      className="mt-1.5"
    />
  )
}
