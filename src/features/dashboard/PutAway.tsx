import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Undo2 } from 'lucide-react'
import type { Dismissal } from '@/lib/api/types'
import { Button } from '@/components/ui/button'

/**
 * What has been put away, and the way back.
 *
 * Folded behind a count rather than listed: hiding is meant to quieten the
 * board, and a permanent list of everything dismissed would undo that. It still
 * has to be reachable — a complaint hidden by mistake is otherwise gone for as
 * long as it keeps saying the same thing.
 */
export function PutAway({
  rows,
  onRestore,
}: {
  rows: readonly Dismissal[]
  onRestore: (row: Dismissal) => void
}) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)

  return (
    // `text-sm` is the line's, and the link in it takes its size from the
    // line it sits in.
    <div className="flex flex-wrap items-center gap-2 text-sm">
      <Button variant="link" onClick={() => setOpen((current) => !current)} aria-expanded={open}>
        {t('focus.hiddenCount', { count: rows.length })}
      </Button>

      {open &&
        rows.map((row) => (
          <Button
            key={`${row.kind}:${row.work_id}:${row.complaint}`}
            variant="ghost"
            size="xs"
            onClick={() => onRestore(row)}
            title={t('focus.restoreHint')}
          >
            <Undo2 aria-hidden />
            {t(`findings.kindShort.${row.kind}`)}
          </Button>
        ))}
    </div>
  )
}
