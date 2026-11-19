import { useEffect, useState } from 'react'
import { formatDuration } from '@/lib/format'

/**
 * How long something has been going, counted each second from `since`
 * (v0.163): beside a run that writes a release's meta, so a minute of quiet
 * reads as work under way rather than a hang.
 */
export function Elapsed({ since }: { since: string }) {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const tick = setInterval(() => setNow(Date.now()), 1000)
    return () => clearInterval(tick)
  }, [])
  const began = Date.parse(since)
  if (Number.isNaN(began)) return null
  return <span className="tabular-nums">{formatDuration(Math.max(0, now - began))}</span>
}
