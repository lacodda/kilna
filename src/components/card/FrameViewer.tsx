import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronLeft, ChevronRight, X } from 'lucide-react'
import { fileSrc, type SceneFrame } from '@/lib/api'
import { Dialog, DialogBackdrop, DialogPopup } from '@/components/ui/dialog'

/** One step of the viewer: a scene, and the frame of it being shown. */
export interface Viewing {
  sceneId: string
  /** The scene's number, shown as the caption. */
  number: number
  frame: SceneFrame
}

interface Props {
  viewing: Viewing | null
  onClose: () => void
  /** Move to the previous or next scene that has a frame to show. */
  onStep: (direction: -1 | 1) => void
  /** Whether stepping that way leads anywhere, so the arrows can say so. */
  canStep: (direction: -1 | 1) => boolean
}

/**
 * A frame at full size, with the board underneath it.
 *
 * The arrows walk scenes rather than the frames of one scene: what a person
 * checks at this size is whether the story reads — scene after scene — and the
 * candidates within a scene are compared in the strip, where they sit side by
 * side. A scene with no frame is skipped rather than shown empty; there is
 * nothing to look at there and stopping on it breaks the walk.
 *
 * The keys are the ones the gesture already implies: arrows step, Escape
 * closes. They are bound while the viewer is open and released with it.
 */
export function FrameViewer({ viewing, onClose, onStep, canStep }: Props) {
  const { t } = useTranslation()

  useEffect(() => {
    if (!viewing) return
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'ArrowLeft') {
        event.preventDefault()
        onStep(-1)
      } else if (event.key === 'ArrowRight') {
        event.preventDefault()
        onStep(1)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [viewing, onStep])

  if (!viewing) return null

  return (
    <Dialog open onOpenChange={(next) => !next && onClose()}>
      <DialogBackdrop />
      <DialogPopup
        className="flex h-[92vh] w-[92vw] max-w-none flex-col gap-2 p-3"
        aria-label={t('scenes.openFrame')}
      >
        <div className="flex items-center justify-between gap-2">
          <span className="text-sm font-medium">
            {t('scenes.frameOfScene', { number: viewing.number })}
          </span>
          <span className="truncate text-xs text-dim">{viewing.frame.original_name}</span>
          <button
            type="button"
            onClick={onClose}
            aria-label={t('scenes.closeFrame')}
            title={t('scenes.closeFrame')}
            className="rounded p-1 text-dim hover:text-fg"
          >
            <X className="size-4" aria-hidden />
          </button>
        </div>

        <div className="flex min-h-0 flex-1 items-center gap-2">
          <button
            type="button"
            onClick={() => onStep(-1)}
            disabled={!canStep(-1)}
            aria-label={t('scenes.previousScene')}
            title={t('scenes.previousScene')}
            className="rounded p-2 text-dim enabled:hover:text-fg disabled:opacity-30"
          >
            <ChevronLeft className="size-6" aria-hidden />
          </button>

          {/* Contained: a frame is looked at whole here, whatever its shape. */}
          <img
            src={fileSrc(viewing.frame.path)}
            alt={viewing.frame.original_name ?? ''}
            className="min-h-0 flex-1 object-contain"
          />

          <button
            type="button"
            onClick={() => onStep(1)}
            disabled={!canStep(1)}
            aria-label={t('scenes.nextScene')}
            title={t('scenes.nextScene')}
            className="rounded p-2 text-dim enabled:hover:text-fg disabled:opacity-30"
          >
            <ChevronRight className="size-6" aria-hidden />
          </button>
        </div>
      </DialogPopup>
    </Dialog>
  )
}
