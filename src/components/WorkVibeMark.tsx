import { useTranslation } from 'react-i18next'
import type { ShownVibe } from '@/lib/api/types'
import { useProfile } from '@/lib/useProfile'
import { axesOf, resolveVibe, vibeWords } from '@/lib/vibe'
import { VibeIcon } from '@/components/VibeIcon'

/**
 * The mark of what a work shows it feels like - its own vibe, or the one it
 * takes from what it was made from (FR-02) - wherever a work stands in a row:
 * a queue, a week, a planned slot. Nothing for a work that shows none, and
 * nothing in a profile that words no lines.
 */
export function WorkVibeMark({
  shown,
  size = 16,
  className,
}: {
  shown: ShownVibe | null | undefined
  /** In pixels: 14 on a calendar's slot, 16 in a row. */
  size?: number
  className?: string
}) {
  const { i18n } = useTranslation()
  const profile = useProfile()
  const vibe = resolveVibe(axesOf(profile.config), shown?.vibe)
  if (vibe === null) return null
  return (
    <VibeIcon
      vibe={vibe}
      size={size}
      words={vibeWords(vibe, i18n.resolvedLanguage ?? 'en')}
      className={className}
    />
  )
}
