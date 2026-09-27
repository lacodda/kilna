import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Check, LoaderCircle } from 'lucide-react'
import { cn } from '@/lib/utils'

export type SaveStatus = 'idle' | 'saving' | 'saved'

/**
 * Turns "is a mutation in flight" into the three states this indicator shows.
 *
 * Only the tick is state: `saving` is read straight off the mutation, and the
 * tick decays on its own, because one that never leaves stops meaning "just
 * now" and becomes furniture.
 */
export function useSaveStatus(isPending: boolean, isError = false): SaveStatus {
  const [justSaved, setJustSaved] = useState(false)
  // The falling edge of a real save is the only thing worth a tick — without
  // this, mounting next to an idle mutation would flash one immediately.
  const [wasPending, setWasPending] = useState(isPending)

  if (wasPending !== isPending) {
    setWasPending(isPending)
    // A failure is already being announced by a toast; do not also say "saved".
    setJustSaved(!isPending && !isError)
  }

  useEffect(() => {
    if (!justSaved) return

    const timer = setTimeout(() => setJustSaved(false), 2000)
    return () => clearTimeout(timer)
  }, [justSaved])

  if (isPending) return 'saving'
  return justSaved ? 'saved' : 'idle'
}

/**
 * The quiet "saving… / saved" line next to fields that save on blur.
 *
 * Holds its own width so the layout does not twitch as the text changes.
 */
export function SaveState({ status, className }: { status: SaveStatus; className?: string }) {
  const { t } = useTranslation()

  return (
    <span
      aria-live="polite"
      className={cn(
        'inline-flex min-w-16 items-center gap-1 text-xs text-faint transition-opacity',
        status === 'idle' && 'opacity-0',
        className,
      )}
    >
      {status === 'saving' && (
        <>
          <LoaderCircle aria-hidden className="size-3 animate-spin" />
          {t('save.saving')}
        </>
      )}
      {status === 'saved' && (
        <>
          <Check aria-hidden className="size-3 text-good" />
          {t('save.saved')}
        </>
      )}
    </span>
  )
}
