import { useEffect, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { LayerProvider } from '@/components/ui/layer'

interface Props {
  /** Back to the card. */
  onClose: () => void
  children: ReactNode
}

/**
 * A text over the whole window, to be read with nothing else in view.
 *
 * Rendered through a portal, so the pane on it keeps the state of whoever
 * rendered it - the version open, the way it is being read, the text being
 * typed - and only its place on screen changes.
 *
 * Escape takes it back to the card. An editor inside puts its pen down on
 * the first Escape and stops the key there, so leaving a stage mid-sentence is
 * two presses, one step each.
 *
 * The stage sits on the token ladder, not beside it. It carried a raw
 * Tailwind `z-50` once - a literal 50, chosen to clear the page rather than to
 * take a place in the scale - and menus are `--z-menu`, a 30, so the compare
 * control opened its list of versions UNDERNEATH the stage: the button
 * highlighted, nothing appeared, and it read as broken. The stage stands on
 * the overlay rung, and `LayerProvider` hands anything opened in here a floor
 * above it, the way a dialog does for the popups inside it.
 */
export function Stage({ onClose, children }: Props) {
  const { t } = useTranslation()

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || event.defaultPrevented) return
      event.preventDefault()
      onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  return createPortal(
    <LayerProvider above="overlay">
      <div className="fixed inset-0 flex flex-col gap-2 bg-bg p-6 [z-index:var(--z-overlay)]">
        <p className="text-xs text-faint">{t('versions.stageHint')}</p>
        <div className="flex min-h-0 flex-1 flex-col">{children}</div>
      </div>
    </LayerProvider>,
    document.body,
  )
}
