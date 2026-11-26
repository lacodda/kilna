import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from 'dowel-ui'
import type { ShortStyle } from '@/lib/api/types'
import { fileSrc } from '@/lib/api/assets'
import { FRAME } from '@/lib/shorts/frame'
import { draw, paint, pictureOf } from '@/lib/shorts/layers'

/** How long after the last change the frame is drawn again: a number being
 *  typed is drawn once, when it is whole. */
const SETTLE_MS = 180

/**
 * One frame of a short, 9:16, drawn live from a look (v0.168): the ground
 * and its blots, the picture, the strip and the wave, the header, a line of
 * words, the stamp and the sign - by the same code that draws the layers a
 * build is handed, so what is seen here is what is built.
 */
export function ShortFrame({
  look,
  title,
  cover,
  className,
}: {
  look: ShortStyle
  /** The title the header letters. */
  title: string
  /** The cover, whole, to stand where the video will; none draws a plate. */
  cover?: string | null
  className?: string
}) {
  const { t } = useTranslation()
  const canvas = useRef<HTMLCanvasElement>(null)
  const [failed, setFailed] = useState<string | null>(null)
  const words = t('shorts.sampleWords')
  // The look as text: what the effect waits on, so an object made anew on
  // every render with the same values draws nothing again.
  const key = JSON.stringify(look)

  useEffect(() => {
    let gone = false
    const timer = setTimeout(() => {
      void (async () => {
        try {
          const shown = JSON.parse(key) as ShortStyle
          const [drawn, picture] = await Promise.all([
            draw(shown, title),
            cover == null ? Promise.resolve(null) : pictureOf(fileSrc(cover)).catch(() => null),
          ])
          if (gone || canvas.current === null) return
          paint(canvas.current, shown, drawn, { picture, words })
          setFailed(null)
        } catch (cause) {
          if (!gone) setFailed(cause instanceof Error ? cause.message : String(cause))
        }
      })()
    }, SETTLE_MS)
    return () => {
      gone = true
      clearTimeout(timer)
    }
  }, [key, title, cover, words])

  return (
    <figure className={cn('flex flex-col gap-1.5', className)}>
      <canvas
        ref={canvas}
        width={FRAME.width}
        height={FRAME.height}
        role="img"
        aria-label={t('shorts.frame', { title })}
        className="aspect-[9/16] w-full rounded-lg border border-line bg-soft"
      />
      {failed !== null && (
        <figcaption role="alert" className="text-xs text-bad">
          {t('shorts.frameFailed', { reason: failed })}
        </figcaption>
      )}
    </figure>
  )
}
