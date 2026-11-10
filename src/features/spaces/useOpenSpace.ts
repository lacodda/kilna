import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { openSpace, openTarget } from '@/lib/api/spaces'
import type { CurrentSpace } from '@/lib/api/types'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'

/**
 * Open another space in the window: the backend swaps its database, and
 * everything the window holds belongs to the space it left - so it starts over
 * on the dashboard, every cached answer dropped and asked again (ADR 0077).
 *
 * The dashboard first, then the reset: a work's screen left open would ask for
 * an id the new space does not have.
 */
function useArrive() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  return async (current: CurrentSpace, route: string | null = null) => {
    navigate(route ?? '/dashboard')
    await client.resetQueries()
    say.ok(t('spaces.opened', { name: current.space.name }))
  }
}

/** Open a listed space by its id. */
export function useOpenSpace() {
  const arrive = useArrive()
  return useAppMutation({
    mutationFn: openSpace,
    failure: 'spaces.openFailed',
    onSuccess: (current) => void arrive(current),
  })
}

/** Open what a link or a double-clicked `space.kilna` names, and go where it
 *  points. */
export function useOpenTarget() {
  const arrive = useArrive()
  return useAppMutation({
    mutationFn: openTarget,
    failure: 'spaces.openFailed',
    onSuccess: (opened) => void arrive(opened.current, opened.route),
  })
}
