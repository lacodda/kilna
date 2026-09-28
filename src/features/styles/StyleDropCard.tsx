import { useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { open as openFile } from '@tauri-apps/plugin-dialog'
import { ImagePlus } from 'lucide-react'
import { PICTURES } from '@/lib/media'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { useWindowDrop } from '@/features/styles/useWindowDrop'

interface Props {
  /** Make a style of these files. */
  onPaths: (paths: string[]) => void
  /** A style is being made from the last ones. */
  busy: boolean
}

/**
 * The last card of the dictionary, dashed: a style that is not there yet.
 *
 * Pictures dropped on it from a folder become a new style with them as its
 * references; pressing it picks them from disk instead, so the keyboard has
 * the same way in. A picture pasted with Ctrl+V while no style is open does
 * the same - the screen listens for that, since a paste has no position to
 * land on this card with.
 */
export function StyleDropCard({ onPaths, busy }: Props) {
  const { t } = useTranslation()
  const zone = useRef<HTMLLIElement>(null)
  const over = useWindowDrop(zone, onPaths)

  const pick = async () => {
    const picked = await openFile({
      multiple: true,
      filters: [{ name: t('styles.pictures'), extensions: PICTURES }],
    })
    if (picked === null) return
    onPaths(Array.isArray(picked) ? picked : [picked])
  }

  return (
    <li ref={zone} className="flex">
      {/* A quiet Button in the shape of a card, dashed like every "not there
          yet" of the line; no size, because a card is as tall as what it
          holds. */}
      <Button
        variant="ghost"
        size={null}
        onClick={() => void pick()}
        disabled={busy}
        className={cn(
          'min-h-37.5 w-full flex-col justify-center gap-1.5 rounded-lg border-dashed bg-softer p-4.5 text-center font-normal whitespace-normal',
          over && 'border-accent bg-accent-soft',
        )}
      >
        <ImagePlus aria-hidden className="size-5 text-faint" />
        <span className="caption">{t('styles.new')}</span>
        <span className="text-sm text-faint">
          {over ? t('styles.dropToMake') : t('styles.dropHint')}
        </span>
      </Button>
    </li>
  )
}
