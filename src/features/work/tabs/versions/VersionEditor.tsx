import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Eye, PenLine } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { MarkedTextarea } from '@/components/ui/marked-text'
import { Markdown } from '@/components/Markdown'
import { cn } from '@/lib/utils'
import { CompareColumn } from '@/features/work/tabs/versions/CompareColumn'
import { TextScroll } from '@/features/work/tabs/versions/TextScroll'
import { textMetrics } from '@/features/work/tabs/versions/metrics'
import { useLineDiff } from '@/features/work/tabs/versions/useLineDiff'

interface Props {
  /** The role the version is written into, for the bar. */
  roleLabel: string
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
  /** The version the draft was copied from, standing beside it; null while
   *  it loads, absent for a version written from nothing. */
  source?: { label: string; body: string } | null
}

/**
 * Where a version is written from nothing, or from a copy - in the place of
 * the open version, not under it.
 *
 * It was appended below the text, and the right column turned into a page:
 * the text on top, the form under it, one scroll for both and the save button
 * somewhere past the fold (the audit of 24.09). Now the form is the detail
 * while it is open, laid out the way a version is: the bar on top, the text
 * taking the panel, and what finishes it - make it current, save - at the
 * foot, where it never moves.
 *
 * A copy keeps its original beside it, line against line, the way comparing
 * two versions does: a rewrite is written against what it rewrites.
 */
export function VersionEditor({
  roleLabel,
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
  source,
}: Props) {
  const { t } = useTranslation()
  const [preview, setPreview] = useState(false)
  const area = useRef<HTMLTextAreaElement>(null)
  const metrics = textMetrics(markdown)
  const diff = useLineDiff(source?.body ?? null, draft)

  // Coming back from preview should put the cursor back in the text, not leave
  // the person clicking to resume.
  useEffect(() => {
    if (!preview) area.current?.focus()
  }, [preview])

  return (
    <section className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-lg border border-line bg-raise">
      <div className="flex shrink-0 items-center gap-2 border-b border-line px-2.5 py-1.5">
        <span className="shrink-0 font-mono text-xs text-faint">
          {t('versions.new')} · {roleLabel}
        </span>
        <Input
          className="h-control-sm w-56 py-1 text-xs"
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

      <TextScroll label={t('versions.draftPlaceholder')}>
        <div className="flex min-w-0 flex-1">
          <div className="flex min-w-0 flex-1 flex-col *:flex-1">
            {preview && markdown ? (
              <div className={metrics}>
                {draft.trim() === '' ? (
                  <p className="text-sm text-faint">{t('versions.previewEmpty')}</p>
                ) : (
                  <Markdown body={draft} />
                )}
              </div>
            ) : (
              <MarkedTextarea
                ref={area}
                value={draft}
                onChange={onDraftChange}
                onKeyDown={(event) => {
                  // Escape puts the form away, as it leaves every editor
                  // here; the draft is kept, so nothing is lost by it.
                  if (event.key === 'Escape' && onCancel !== undefined) {
                    event.preventDefault()
                    onCancel()
                  }
                }}
                placeholder={t('versions.draftPlaceholder')}
                aria-label={t('versions.draftPlaceholder')}
                lineMarks={diff.added}
                className={cn('block w-full placeholder:text-faint', metrics)}
              />
            )}
          </div>
          {source != null && (
            <CompareColumn
              label={t('versions.derivedFrom', { name: source.label })}
              body={source.body}
              removed={diff.removed}
              counts={diff.counts}
              metrics={metrics}
            />
          )}
        </div>
      </TextScroll>

      <div className="flex shrink-0 flex-wrap items-center gap-3 border-t border-line px-2.5 py-2">
        <Checkbox checked={makeCurrent} onCheckedChange={onMakeCurrentChange}>
          {t('versions.makeCurrentOnSave')}
        </Checkbox>

        {kept && <span className="text-xs text-faint">{t('versions.kept')}</span>}

        <div className="ml-auto flex items-center gap-2">
          {onCancel !== undefined && (
            <Button variant="ghost" size="sm" onClick={onCancel}>
              {t('versions.cancel')}
            </Button>
          )}
          <Button
            variant="primary"
            size="sm"
            disabled={draft.trim() === '' || saving}
            onClick={onSave}
          >
            {t('versions.save')}
          </Button>
        </div>
      </div>
    </section>
  )
}
