import { useMutation, useQueryClient, type QueryKey } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { keys } from '@/lib/query/keys'
import { say } from '@/lib/toast'

export interface AppMutation<TData, TVariables> {
  mutationFn: (variables: TVariables) => Promise<TData>
  /**
   * What failed, as a locale key naming the gesture - `toast.noteSaveFailed`.
   * Without one the failure is said in the backend's own words.
   */
  failure?: string
  /** The query areas the write disturbs; see `refresh`. The journal always is. */
  refresh?: readonly QueryKey[]
  onSuccess?: (data: TData, variables: TVariables) => void
  /** In place of saying the failure: for a gesture that puts something back first. */
  onError?: (cause: unknown, variables: TVariables) => void
}

/**
 * A write, the way every screen makes one.
 *
 * Until v0.77 each of the 122 mutations wrote its own `onError` - sixty-one
 * the same line - and its own list of what to refresh. The shape is one:
 * write, refresh what the write disturbed and the journal it wrote to, then
 * whatever this screen does next; on failure, say which gesture failed and why.
 */
export function useAppMutation<TData, TVariables = void>({
  mutationFn,
  failure,
  refresh = [],
  onSuccess,
  onError,
}: AppMutation<TData, TVariables>) {
  const { t } = useTranslation()
  const client = useQueryClient()

  return useMutation({
    mutationFn,
    onSuccess: (data, variables) => {
      for (const key of [...refresh, keys.journal]) void client.invalidateQueries({ queryKey: key })
      onSuccess?.(data, variables)
    },
    onError: (cause, variables) => {
      if (onError !== undefined) onError(cause, variables)
      else if (failure !== undefined) say.failedTo(t(failure), cause)
      else say.failed(cause)
    },
  })
}
