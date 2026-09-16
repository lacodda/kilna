import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Copy, Sparkles } from 'lucide-react'
import {
  generateReleaseFields,
  releaseFields,
  setReleaseFields,
  type ReleaseFieldValue,
  type ScheduledRelease,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { fillable, over, written } from '@/lib/releaseFields'
import { labelOf, useVocabulary } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/ui/AppDialog'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/Skeleton'
import { SaveState, useSaveStatus } from '@/components/ui/SaveState'
import { Textarea } from '@/components/ui/textarea'

interface Props {
  release: ScheduledRelease
}

/**
 * What a release goes out as: its title, its description, its tags, the
 * comment pinned under it.
 *
 * Which boxes appear is the profile's answer, not this component's - a clip
 * is asked for four things and a beta read for two, and neither list is
 * spelled here. A kind that names no fields draws a line saying so rather
 * than nothing at all, because "this release says nothing about itself" and
 * "kilna forgot to draw the boxes" look identical when both are blank.
 *
 * Boxes save on blur, the way the rest of the card does. Generating is
 * separate and always confirmed once something is written: it replaces what
 * is in the templated boxes, and a button that overwrites an evening's
 * wording without asking is a button nobody presses twice.
 */
export function ReleaseFields({ release }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const kinds = useVocabulary(release.work_id).release_kinds

  const [confirming, setConfirming] = useState(false)

  const fields = useQuery({
    queryKey: keys.releaseFields(release.id),
    queryFn: () => releaseFields(release.id),
  })

  const refresh = () => {
    void client.invalidateQueries({ queryKey: keys.releaseFields(release.id) })
    void client.invalidateQueries({ queryKey: keys.releasesForWork(release.work_id) })
  }

  const save = useMutation({
    mutationFn: (values: Record<string, string>) => setReleaseFields(release.id, values),
    onSuccess: refresh,
    onError: (cause: unknown) => say.failedTo(t('releases.meta.saveFailed'), cause),
  })

  const generate = useMutation({
    mutationFn: () => generateReleaseFields(release.id),
    onSuccess: (generated) => {
      refresh()
      if (Object.keys(generated.values).length === 0 && generated.refused.length === 0) {
        say.ok(t('releases.meta.generatedNothing'))
      } else if (Object.keys(generated.values).length > 0) {
        say.ok(t('releases.meta.generated'))
      }
      // Each refusal is its own line: they are separate problems with
      // separate fixes, and one line holding four of them is read by nobody.
      for (const refusal of generated.refused) {
        say.warn(t('releases.meta.refused', { label: refusal.label, reason: refusal.reason }))
      }
    },
    onError: (cause: unknown) => say.failedTo(t('releases.meta.generateFailed'), cause),
  })

  const status = useSaveStatus(save.isPending, save.isError)

  const copy = (field: ReleaseFieldValue) => {
    navigator.clipboard.writeText(field.value).then(
      () => say.ok(t('releases.meta.copied', { label: field.label })),
      (cause: unknown) => say.failedTo(t('releases.meta.copyFailed'), cause),
    )
  }

  if (fields.isPending) return <Skeleton className="h-24 w-full" />
  if (fields.isError) return null

  const data = fields.data
  const canFill = fillable(data)
  const anything = data.some((field) => field.value.trim() !== '')

  if (data.length === 0) {
    return (
      <p className="text-xs text-faint">
        {t('releases.meta.none', { kind: labelOf(kinds, release.kind) })}
      </p>
    )
  }

  const count = written(data)

  return (
    <section className="flex flex-col gap-3 rounded-xl border border-line p-3">
      <header className="flex items-center gap-2">
        <h4 className="text-sm font-semibold">{t('releases.meta.title')}</h4>
        <span className={cn('text-xs', count.complete ? 'text-good' : 'text-dim')}>
          {count.complete
            ? t('releases.meta.complete')
            : t('releases.meta.written', { written: count.written, total: count.total })}
        </span>
        <SaveState status={status} className="ml-auto" />
        {canFill && (
          <Button
            // Confirmed only when there is something to lose. On an empty
            // release the dialog would be a step between the person and the
            // thing they obviously want.
            onClick={() => (anything ? setConfirming(true) : generate.mutate())}
            disabled={generate.isPending}
            className={cn(!anything && 'ml-auto')}
          >
            <Sparkles aria-hidden className="size-3.5" />
            {anything ? t('releases.meta.regenerate') : t('releases.meta.generate')}
          </Button>
        )}
      </header>

      {data.map((field) => (
        <ReleaseFieldBox
          key={field.key}
          field={field}
          onCopy={() => copy(field)}
          onSave={(value) => {
            if (value === field.value) return
            save.mutate({ [field.key]: value })
          }}
        />
      ))}

      <Dialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('releases.meta.confirmTitle')}
      >
        <p className="text-sm text-dim">{t('releases.meta.confirmBody')}</p>
        <div className="mt-3 flex justify-end gap-2">
          <Button type="button" onClick={() => setConfirming(false)}>
            {t('dialog.cancel')}
          </Button>
          <Button
            type="button"
            variant="primary"
            onClick={() => {
              setConfirming(false)
              generate.mutate()
            }}
          >
            {t('releases.meta.confirmAction')}
          </Button>
        </div>
      </Dialog>
    </section>
  )
}

interface BoxProps {
  field: ReleaseFieldValue
  onSave: (value: string) => void
  onCopy: () => void
}

/**
 * One field's box.
 *
 * The draft is local while it is being typed and is handed over on blur, so
 * the box does not fight a refetch mid-sentence. When the stored value
 * changes underneath - a generation landed - the box takes the new text,
 * because that is what the person just asked for.
 */
function ReleaseFieldBox({ field, onSave, onCopy }: BoxProps) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState(field.value)
  const [syncedTo, setSyncedTo] = useState(field.value)

  if (syncedTo !== field.value) {
    setSyncedTo(field.value)
    setDraft(field.value)
  }

  const excess = over(field, draft)
  const counter =
    field.limit == null
      ? null
      : excess !== null
        ? t('releases.meta.overLimit', { count: excess })
        : t('releases.meta.ofLimit', { used: draft.length, limit: field.limit })

  const shared = {
    value: draft,
    onChange: (event: { target: { value: string } }) => setDraft(event.target.value),
    onBlur: () => onSave(draft),
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-2">
        <span className="text-2xs font-semibold uppercase tracking-caption text-faint">
          {field.label}
        </span>
        {counter !== null && (
          // A count, never a refusal: kilna is not the authority on what a
          // platform accepts this month, and a box that refuses to hold the
          // text is a box typed somewhere else.
          <span className={cn('text-2xs', excess !== null ? 'text-bad' : 'text-faint')}>
            {counter}
          </span>
        )}
        <button
          type="button"
          onClick={onCopy}
          disabled={draft.trim() === ''}
          title={t('releases.meta.copy')}
          aria-label={t('releases.meta.copy')}
          className="ml-auto text-faint hover:text-fg disabled:opacity-40"
        >
          <Copy aria-hidden className="size-3.5" />
        </button>
      </div>

      {field.type === 'line' || field.type === 'tags' ? (
        <Input {...shared} placeholder={field.hint ?? undefined} />
      ) : (
        <Textarea {...shared} autoResize maxRows={12} rows={3} />
      )}

      {field.hint != null && field.type === 'text' && (
        <span className="text-xs text-faint">{field.hint}</span>
      )}
    </div>
  )
}
