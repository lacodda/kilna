import { fileSrc } from '@/lib/api'
import { extensionOf, mediaKindOf } from '@/lib/media'
import { cn } from '@/lib/utils'

/*
 * A stored file drawn as what it is: a picture as a picture, a clip as a
 * player, anything else as a plain tile naming its kind. The kind comes from
 * `lib/media`; see there for why this exists.
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
    return <img src={fileSrc(path)} alt={alt} className={className} draggable={false} />
  }

  if (kind === 'clip') {
    // `metadata` draws the first frame without loading the whole clip, which
    // is what a gallery of fifty scenes can afford.
    return (
      <video
        src={fileSrc(path)}
        aria-label={alt}
        preload="metadata"
        controls={controls}
        muted={!controls}
        playsInline
        className={className}
      />
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
