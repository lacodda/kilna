import { useEffect, useRef, useState } from 'react'
import { AudioLines } from 'lucide-react'
import { fileSrc } from '@/lib/api/assets'
import { extensionOf, mediaKindOf } from '@/lib/media'
import { cn } from '@/lib/utils'

/*
 * A stored file drawn as what it is: a picture as a picture, a clip as a
 * player, a sound as a mark the card's player can be asked to play, anything
 * else as a plain tile naming its kind. The kind comes from `lib/media`; see
 * there for why this exists.
 */
export function MediaPreview({
  path,
  alt,
  className,
  controls = false,
}: {
  path: string
  alt: string
  className?: string
  /** A player's own controls - for looking at a clip, not for a thumbnail. */
  controls?: boolean
}) {
  const kind = mediaKindOf(path)

  if (kind === 'picture') {
    // Lazy, because a work's folder on disk can hold a hundred stills, and a
    // tab that decodes all of them before it scrolls is a tab that stalls.
    return (
      <img
        src={fileSrc(path)}
        alt={alt}
        className={className}
        draggable={false}
        loading="lazy"
        decoding="async"
      />
    )
  }

  if (kind === 'clip')
    return <Clip path={path} alt={alt} className={className} controls={controls} />

  if (kind === 'sound') {
    return (
      <span
        role="img"
        aria-label={alt}
        className={cn(
          'flex flex-col items-center justify-center gap-1 font-mono text-xs uppercase tracking-caption text-faint',
          className,
        )}
      >
        <AudioLines aria-hidden className="size-6 text-dim" />
        {extensionOf(path)}
      </span>
    )
  }

  return (
    <span
      role="img"
      aria-label={alt}
      className={cn(
        'flex items-center justify-center font-mono text-sm uppercase tracking-caption text-faint',
        className,
      )}
    >
      {extensionOf(path) || '?'}
    </span>
  )
}

/**
 * A clip, whose first frame is asked for only once the tile is on screen:
 * `metadata` draws that frame without loading the whole clip, which a board
 * of fifty scenes can afford - and a folder of forty takes cannot, all at
 * once, before the person has scrolled to any of them.
 */
function Clip({
  path,
  alt,
  className,
  controls,
}: {
  path: string
  alt: string
  className?: string
  controls: boolean
}) {
  const element = useRef<HTMLVideoElement>(null)
  // Where there is no observer to ask (a test's document) the clip is taken
  // as seen, which is what it was before it learnt to wait.
  const [seen, setSeen] = useState(typeof IntersectionObserver === 'undefined')

  useEffect(() => {
    const target = element.current
    if (seen || target === null) return
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setSeen(true)
          observer.disconnect()
        }
      },
      { rootMargin: '200px' },
    )
    observer.observe(target)
    return () => observer.disconnect()
  }, [seen])

  return (
    <video
      ref={element}
      src={fileSrc(path)}
      aria-label={alt}
      preload={seen ? 'metadata' : 'none'}
      controls={controls}
      muted={!controls}
      playsInline
      className={className}
    />
  )
}
