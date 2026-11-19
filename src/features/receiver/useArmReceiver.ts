import type { MediaRole } from '@/lib/api/types'
import { armReceiver } from '@/lib/api/receiver'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'

/** Arm the receiver for a work's file (v0.156): the bar above every screen
 *  says what waits from then on. */
export function useArmReceiver() {
  return useAppMutation({
    mutationFn: ({ workId, role }: { workId: string; role: MediaRole }) =>
      armReceiver(workId, role),
    failure: 'receiver.armFailed',
    refresh: [keys.armedReceiver],
  })
}
