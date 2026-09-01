import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Eye, Maximize2, Minimize2, PenLine } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Markdown } from '@/components/ui/Markdown'
import { cn } from '@/lib/utils'

interface Props {
  draft: string
  onDraftChange: (body: string) => void
  label: string
  onLabelChange: (label: string) => void
  makeCurrent: boolean
  onMakeCurrentChange: (value: boolean) => void
  onSave: () => void
  saving: boolean
  /** Shown while there is unsaved text: nothing is lost, but nothing is a version yet. */
  kept: boolean
}

/**
 * Where a new version is written.
 *
 * Two modes and a size. Write and Preview swap what the pane shows; full screen
 * takes over the window, because a page of lyrics read inside a 6-row box is
 * not read at all.
 */
export function VersionEditor({
  draft,
  onDraftChange,
  label,
  onLabelChange,
  makeCurrent,
  onMakeCurrentChange,
  onSave,
  saving,
  kept,
}: Props) {
  const { t } = useTranslation()
  const [preview, setPreview] = useState(false)
  const [full, setFull] = useState(false)
  const area = useRef<HTMLTextAreaElement>(null)

  // Escape leaves full screen. Bound while it is open only, so it does not
  // shadow anything else on the card.
  useEffect(() => {
    if (!full) return
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setFull(false)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [full])

  // Coming back from preview or full screen should put the cursor back in the
  // text, not leave the person clicking to resume.
  useEffect(() => {
    if (!preview) area.current?.focus()
  }, [preview, full])

  const body = (
    <div className={cn('flex min-h-0 flex-1 flex-col gap-2', full && 'h-full')}>
      <div className="flex flex-wrap items-center gap-2">
        <Input
          className="w-56"
          value={label}
          onChange={(event) => onLabelChange(event.target.value)}
          placeholder={t('versions.labelPlaceholder')}
          aria-label={t('versions.labelPlaceholder')}
        />

        <div className="ml-auto flex items-center gap-1">
          <Button
            variant={preview ? 'icon' : 'soft'}
            size="icon-sm"
            onClick={() => setPreview(false)}
            title={t('versions.write')}
            aria-label={t('versions.write')}
            aria-pressed={!preview}
          >
            <PenLine aria-hidden />
          </Button>
          <Button
            variant={preview ? 'soft' : 'icon'}
            size="icon-sm"
            onClick={() => setPreview(true)}
            title={t('versions.preview')}
            aria-label={t('versions.preview')}
            aria-pressed={preview}
          >
            <Eye aria-hidden />
          </Button>
          <Button
            variant="icon"
            size="icon-sm"
            onClick={() => setFull(!full)}
            title={full ? t('versions.exitFullScreen') : t('versions.fullScreen')}
            aria-label={full ? t('versions.exitFullScreen') : t('versions.fullScreen')}
          >
            {full ? <Minimize2 aria-hidden /> : <Maximize2 aria-hidden />}
          </Button>
        </div>
      </div>

      {preview ? (
        <div
          className={cn(
            'overflow-auto rounded-[9px] border border-line px-3 py-2',
            full ? 'min-h-0 flex-1' : 'min-h-[9rem]',
          )}
        >
          {draft.trim() === '' ? (
            <p className="text-sm text-faint">{t('versions.previewEmpty')}</p>
          ) : (
            <Markdown body={draft} />
          )}
        </div>
      ) : (
        <Textarea
          ref={area}
          className={cn(full ? 'min-h-0 flex-1 resize-none' : 'min-h-[9rem]')}
          rows={full ? undefined : 6}
          value={draft}
          onChange={(event) => onDraftChange(event.target.value)}
          placeholder={t('versions.draftPlaceholder')}
          aria-label={t('versions.draftPlaceholder')}
        />
      )}

      <div className="flex flex-wrap items-center gap-3">
        <label className="flex cursor-pointer items-center gap-2 text-xs text-dim">
          <input
            type="checkbox"
            className="size-4 accent-[var(--accent)]"
            checked={makeCurrent}
            onChange={(event) => onMakeCurrentChange(event.target.checked)}
          />
          {t('versions.makeCurrentOnSave')}
        </label>

        {kept && <span className="text-xs text-faint">{t('versions.kept')}</span>}

        <Button
          className="ml-auto"
          variant="primary"
          disabled={draft.trim() === '' || saving}
          onClick={onSave}
        >
          {t('versions.save')}
        </Button>
      </div>
    </div>
  )

  if (!full) return body

  return (
    <div className="fixed inset-0 z-50 flex flex-col gap-2 bg-bg p-6">
      <p className="text-xs text-faint">{t('versions.fullScreenHint')}</p>
      {body}
    </div>
  )
}
