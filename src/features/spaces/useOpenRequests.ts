import { useEffect } from 'react'
import { onOpenRequest, takeLaunchTarget } from '@/lib/api/spaces'
import { useOpenTarget } from '@/features/spaces/useOpenSpace'

/**
 * What kilna is handed to open - a `kilna://<space>/work/<id>` link (#15), a
 * `space.kilna` double-clicked (#99) - opened in the window: the one the first
 * start was handed, once the window stands, and each one after while it is
 * open. The space is opened when it is not, and the window goes where the
 * link points.
 */
export function useOpenRequests() {
  const open = useOpenTarget()
  const { mutate } = open

  useEffect(() => {
    let gone = false
    takeLaunchTarget()
      .then((target) => {
        if (!gone && target !== null) mutate(target)
      })
      .catch(() => undefined)
    const listening = onOpenRequest((target) => mutate(target))
    return () => {
      gone = true
      void listening.then((unlisten) => unlisten()).catch(() => undefined)
    }
  }, [mutate])
}
