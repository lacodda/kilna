import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { convertWikilinks } from '@/lib/api/notes'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'

/** How many links not found the window lists before it says how many more. */
const LISTED = 40

/**
 * Obsidian's `[[links]]` in the notes turned into kilna's (v0.145, FR-09
 * 9.2): what a conversion would do, read first - how many notes change, how
 * many links are found, which are not - and one press that writes it, an
 * undo away.
 */
export function WikilinkDialog({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation()
  const looked = useQuery({
    queryKey: ['notes', 'wikilinks'],
    queryFn: () => convertWikilinks(false),
    gcTime: 0,
  })
  const apply = useAppMutation({
    mutationFn: () => convertWikilinks(true),
    failure: 'toast.noteSaveFailed',
    refresh: refresh.note,
    onSuccess: (done) => {
      say.ok(t('notes.wikilinksDone', { count: done.resolved, notes: done.notes }))
      onClose()
    },
  })
  const report = looked.data
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
      title={t('notes.wikilinks')}
      description={t('notes.wikilinksHint')}
    >
      {report === undefined ? (
        <p className="text-sm text-faint">{t('notes.wikilinksLooking')}</p>
      ) : (
        <div className="flex flex-col gap-2 text-sm">
          <p>{t('notes.wikilinksFound', { count: report.resolved, notes: report.notes })}</p>
          {report.unresolved.length > 0 && (
            <>
              <p className="text-warn">
                {t('notes.wikilinksUnresolved', { count: report.unresolved.length })}
              </p>
              <ul className="max-h-48 overflow-y-auto rounded-md border border-line px-2 py-1 text-xs">
                {report.unresolved.slice(0, LISTED).map((one, index) => (
                  <li key={`${one.note_id}-${String(index)}`} className="truncate">
                    <span className="font-mono">[[{one.target}]]</span>
                    <span className="text-faint"> · {one.note_title ?? t('notes.untitled')}</span>
                  </li>
                ))}
              </ul>
              {report.unresolved.length > LISTED && (
                <p className="text-xs text-faint">
                  {t('notes.wikilinksMore', { count: report.unresolved.length - LISTED })}
                </p>
              )}
            </>
          )}
          <div className="flex justify-end gap-2 pt-1">
            <Button size="sm" onClick={onClose}>
              {t('dialog.cancel')}
            </Button>
            <Button
              size="sm"
              variant="primary"
              disabled={report.resolved === 0 || apply.isPending}
              onClick={() => apply.mutate()}
            >
              {t('notes.wikilinksApply', { count: report.resolved })}
            </Button>
          </div>
        </div>
      )}
    </Dialog>
  )
}
