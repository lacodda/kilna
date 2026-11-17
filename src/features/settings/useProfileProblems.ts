import { useCallback, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { Reason } from '@/lib/api/generated/Reason'
import type { ProfileConfig } from '@/lib/api/types'
import { checkProfileConfig } from '@/lib/api/spaces'
import { sayReason } from '@/lib/errors'
import { problemsAt, type Wanted } from '@/lib/profileProblems'
import { useDebounced } from '@/lib/useDebounced'

const NONE: Reason[] = []

/**
 * What is wrong with the profile's draft, asked of the backend as it is
 * edited (v0.112): the same check a save runs, so the editor says "this will
 * not save" beside the field and not at the Save button.
 *
 * Asked a moment after the last keystroke, and only of a draft: what is
 * stored was checked when it was stored.
 */
export function useProfileProblems(config: ProfileConfig, dirty: boolean): Reason[] {
  const settled = useDebounced(config, 300)
  const text = useMemo(() => JSON.stringify(settled), [settled])
  const checked = useQuery({
    queryKey: ['profileProblems', text],
    queryFn: () => checkProfileConfig(settled),
    enabled: dirty,
    // The same draft has the same problems.
    staleTime: Infinity,
    // What the last draft had wrong stays on screen while the next is asked
    // about: a list that blinks empty between keystrokes reads as "fixed".
    placeholderData: (previous) => previous,
  })
  return dirty ? (checked.data ?? NONE) : NONE
}

/** The problems at a place, each said in the window's language. */
export type ProblemsAt = (...path: Wanted[]) => string[]

/** A way to ask for the problems at a place, said. */
export function useSaidProblems(problems: readonly Reason[]): ProblemsAt {
  return useCallback(
    (...path: Wanted[]) => problemsAt(problems, path).map((problem) => sayReason(problem)),
    [problems],
  )
}
