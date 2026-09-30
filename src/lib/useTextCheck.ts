import { keepPreviousData, useQuery } from '@tanstack/react-query'
import type { TextCheck } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { useDebounced } from '@/lib/useDebounced'

export interface CheckedText {
  /** The last answer - for the text on screen, or one keystroke behind it. */
  check: TextCheck | undefined
  /** Whether `check` is for the text on screen: only then do its marks land
   *  on the letters they name. */
  current: boolean
}

/**
 * A text checked against itself and the register (ADR 0044), as it is typed.
 *
 * The backend reads every text through one analyser, so the window asks
 * rather than counting on its own. The question trails the typing a little,
 * and the last answer is kept while the next is on its way: the strips under
 * the toolbar hold still instead of blinking out at every letter. The marks
 * wait - an offset computed on the text before the keystroke would land on
 * the wrong word after it.
 */
export function useTextCheck(text: string | null): CheckedText {
  const settled = useDebounced(text, 120)
  const query = useQuery({
    ...queries.textCheck(settled ?? ''),
    enabled: settled !== null,
    placeholderData: keepPreviousData,
  })
  if (text === null) return { check: undefined, current: false }
  return {
    check: query.data,
    current: settled === text && query.data !== undefined && !query.isPlaceholderData,
  }
}
