import { useTranslation } from 'react-i18next'
import type { PhraseHit } from '@/lib/api/types'
import { StyleIcon } from '@/lib/styleIcon'
import { say as sayLabel, styleTypesOf, useProfile } from '@/lib/useProfile'

interface Props {
  hit: PhraseHit
  /** Where the phrase stands on screen. */
  box: DOMRect
}

/**
 * What a phrase of a style text means, under it while the pointer is on it
 * (v0.94): its type, the explanation in the window's language, when to take
 * it. Drawn by the pane rather than as a tooltip of the mark: the marks are
 * spans the text is cut into, and a tip of their own would be one per word.
 * Never in the way of the pointer, so it does not steal the hover it shows.
 */
export function PhraseTip({ hit, box }: Props) {
  const { t } = useTranslation()
  const type = styleTypesOf(useProfile().config).find((one) => one.key === hit.type_key)
  const meaning = sayLabel(hit.explanation)
  return (
    <div
      role="tooltip"
      className="pointer-events-none fixed z-50 flex max-w-80 flex-col gap-1 rounded-md border border-line bg-raise px-3 py-2 text-xs shadow-lg"
      style={{ top: box.bottom + 6, left: Math.max(8, box.left) }}
    >
      <span className="flex items-center gap-1.5 text-faint">
        <StyleIcon of={type} aria-hidden className="size-3" />
        {type === undefined ? hit.type_key : sayLabel(type.label)}
        {hit.house && <span className="text-accent">· {t('styles.house')}</span>}
      </span>
      <span className="font-mono text-text">{hit.phrase}</span>
      {meaning !== '' && <span className="text-dim">{meaning}</span>}
      {hit.when !== undefined && hit.when !== null && (
        <span className="text-faint">
          {t('styles.when')}: {hit.when}
        </span>
      )}
    </div>
  )
}
