import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Eye, PenLine } from 'lucide-react'
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
  /** Offered when the form was opened on purpose and can be put away again. */
  onCancel?: () => void
  saving: boolean
  /** Shown while there is unsaved text: nothing is lost, but nothing is a version yet. */
  kept: boolean
  /** Whether this role's bodies read as markdown, which is what a preview is for. */
  markdown: boolean
}

/**
 * Where a version is written from nothing, or from a copy.
 *
 * Not the everyday way of revising: that is clicking into the open text, which
 * mints the next revision by itself. This form is for the two moments a
 * revision needs a decision first — there is no version in this role yet, or
 * the person asked for a copy to work on — and it is the only place a version
 * is given a name.
 */
export function VersionEditor({
  draft,
  onDraftChange,
  label,
  onLabelChange,
  makeCurrent,
  onMakeCurrentChange,
  onSave,
  onCancel,
  saving,
  kept,
  markdown,
}: Props) {
  const { t } = useTranslation()
  const [preview, setPreview] = useState(false)
  const area = useRef<HTMLTextAreaElement>(null)

  // Coming back from preview should put the cursor back in the text, not leave
  // the person clicking to resume.
  useEffect(() => {
    if (!preview) area.current?.focus()
  }, [preview])

  return (
    <div className="flex min-h-0 flex-col gap-2 rounded-xl border border-dashed border-line p-3">
      <div className="flex flex-wrap items-center gap-2">
        <Input
          className="w-56"
          value={label}
          onChange={(event) => onLabelChange(event.target.value)}
          placeholder={t('versions.labelPlaceholder')}
          aria-label={t('versions.labelPlaceholder')}
        />

        {markdown && (
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
          </div>
        )}
      </div>

      {preview && markdown ? (
        <div className="min-h-[9rem] overflow-auto rounded-[9px] border border-line px-3 py-2">
          {draft.trim() === '' ? (
            <p className="text-sm text-faint">{t('versions.previewEmpty')}</p>
          ) : (
            <Markdown body={draft} />
          )}
        </div>
      ) : (
        <Textarea
          ref={area}
          className={cn('min-h-[9rem]', !markdown && 'font-mono')}
          rows={6}
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

        <div className="ml-auto flex items-center gap-2">
          {onCancel !== undefined && (
            <Button variant="ghost" size="sm" onClick={onCancel}>
              {t('versions.cancel')}
            </Button>
          )}
          <Button variant="primary" disabled={draft.trim() === '' || saving} onClick={onSave}>
            {t('versions.save')}
          </Button>
        </div>
      </div>
    </div>
  )
}
