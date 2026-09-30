import type { Work } from '@/lib/api/types'

/**
 * Where the writing of a publication's release meta stands: running, done
 * with what it wrote, or waiting for the person on the fields they started.
 * Drawn at the top of a publication's Cover and Frame tabs, where "Make…"
 * lands (v0.86).
 */
export function ReleaseMetaStatus({ work }: { work: Work }) {
  void work
  return null
}
