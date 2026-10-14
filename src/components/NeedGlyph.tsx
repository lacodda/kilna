import { FileText, Gauge, Image, PenLine } from 'lucide-react'
import type { Need } from '@/lib/api/types'

/**
 * The glyph of one thing a release cannot go out without (v0.97): a page for
 * a version of a role, a gauge for the score, a picture for the cover, a pen
 * for a field to write. One set for every screen that marks a gap, so a gap
 * looks the same on the chip, in the queue and in the checklist.
 */
export function NeedGlyph({ need, className }: { need: Need; className?: string }) {
  const Glyph = GLYPH[need.kind]
  return <Glyph aria-hidden className={className} />
}

const GLYPH = {
  role: FileText,
  score: Gauge,
  cover: Image,
  field: PenLine,
} as const satisfies Record<Need['kind'], unknown>
