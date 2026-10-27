import { useEffect, useState } from 'react'
import { fileSrc } from '@/lib/api/assets'

/**
 * How long a clip or a sound on disk runs, in seconds, as the window's own
 * player reads it from the file's header - `null` until it is read, and for
 * a file the player cannot open.
 *
 * The window, not the core: the core never opens a video (decision of
 * 2026-09-11), and a player asked for the metadata alone reads a few
 * kilobytes, not the clip.
 */
export function useMediaLength(path: string | null | undefined): number | null {
  const [length, setLength] = useState<{ path: string; seconds: number } | null>(null)

  useEffect(() => {
    if (path === null || path === undefined) return
    const player = document.createElement('video')
    player.preload = 'metadata'
    player.muted = true
    const read = () => {
      if (Number.isFinite(player.duration)) setLength({ path, seconds: player.duration })
    }
    player.addEventListener('loadedmetadata', read)
    player.src = fileSrc(path)
    return () => {
      player.removeEventListener('loadedmetadata', read)
      // Let go of the file, so the system does not hold it open for a tab
      // that moved on.
      player.removeAttribute('src')
      player.load()
    }
  }, [path])

  // A length read for another file is not this one's.
  return length !== null && length.path === path ? length.seconds : null
}
