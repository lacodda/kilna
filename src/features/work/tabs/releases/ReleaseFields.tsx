import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Sparkles } from 'lucide-react'
import { generateReleaseFields, previewReleaseFields, setReleaseFields } from '@/lib/api/releases'
import type {
  GeneratedFields,
  ReleaseFieldRefusal,
  ReleaseFieldValue,
  ScheduledRelease,
} from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { fillable, over, replacements, written, type Replacement } from '@/lib/releaseFields'
import { labelOf, say as sayLabel, useVocabulary } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { CopyButton } from '@/components/ui/copy-button'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { SaveState, useSaveStatus } from '@/components/ui/save-state'
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
 * Boxes save on blur, the way the rest of the card does. Writing them from
 * the work is separate, and once something is written it is shown before it
 * lands: the new text stands under each box it would replace, and nothing is
 * replaced until the person says so. Until v0.80 the question was a dialog
 * asking whether to replace text it did not show - a button that overwrites
 * an evening's wording on the strength of a yes to an unseen draft is a
 * button nobody presses twice.
 */
export function ReleaseFields({ release }: Props) {
  const { t } = useTranslation()
  const kinds = useVocabulary(release.work_id).release_kinds

  // What the work would write, while it is being looked at; null otherwise.
  const [proposal, setProposal] = useState<Replacement[] | null>(null)

  const fields = useQuery(queries.releaseFields(release.id))

  // What a field written or generated changes: the boxes here, and the
  // release's own list where they are read as a whole.
  const refreshed = [keys.releaseFields(release.id), keys.releasesForWork(release.work_id)]

  // Each refusal is its own line: they are separate problems with separate
  // fixes, and one line holding four of them is read by nobody.
  const warnRefused = (refused: ReleaseFieldRefusal[]) => {
    for (const refusal of refused) {
      say.warn(
        t('releases.meta.refused', { label: sayLabel(refusal.label), reason: refusal.reason }),
      )
    }
  }

  const save = useAppMutation({
    mutationFn: (values: Record<string, string>) => setReleaseFields(release.id, values),
    failure: 'releases.meta.saveFailed',
    refresh: refreshed,
  })

  const generate = useAppMutation({
    mutationFn: () => generateReleaseFields(release.id),
    failure: 'releases.meta.generateFailed',
    refresh: refreshed,
    onSuccess: (generated: GeneratedFields) => {
      if (Object.keys(generated.values).length === 0 && generated.refused.length === 0) {
        say.ok(t('releases.meta.generatedNothing'))
      } else if (Object.keys(generated.values).length > 0) {
        say.ok(t('releases.meta.generated'))
      }
      warnRefused(generated.refused)
    },
  })

  // The same rendering, written nowhere. What is shown is exactly what is
  // kept: pressing Replace writes the text on screen, not a second rendering
  // that could differ from it.
  const preview = useAppMutation({
    mutationFn: () => previewReleaseFields(release.id),
    failure: 'releases.meta.generateFailed',
    onSuccess: (generated: GeneratedFields) => {
      const changes = replacements(fields.data ?? [], generated.values)
      if (changes.length > 0) setProposal(changes)
      else if (Object.keys(generated.values).length > 0) say.ok(t('releases.meta.previewSame'))
      else if (generated.refused.length === 0) say.ok(t('releases.meta.generatedNothing'))
      warnRefused(generated.refused)
    },
  })

  const replace = useAppMutation({
    mutationFn: (changes: Replacement[]) =>
      setReleaseFields(
        release.id,
        Object.fromEntries(changes.map((change) => [change.key, change.after])),
      ),
    failure: 'releases.meta.saveFailed',
    refresh: refreshed,
    onSuccess: () => {
      setProposal(null)
      say.ok(t('releases.meta.generated'))
    },
  })

  const status = useSaveStatus(save.isPending, save.isError)

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
  const busy = generate.isPending || preview.isPending || replace.isPending

  return (
    <section className="flex flex-col gap-2.5">
      <header className="flex flex-wrap items-center gap-2">
        <h4 className="caption">{t('releases.meta.title')}</h4>
        <Badge variant={count.complete ? 'good' : 'outline'}>
          {count.complete
            ? t('releases.meta.complete')
            : t('releases.meta.written', { written: count.written, total: count.total })}
        </Badge>
        <SaveState
          savingLabel={t('save.saving')}
          savedLabel={t('save.saved')}
          status={status}
          className="ml-auto"
        />
        {proposal !== null ? (
          <>
            <Button size="sm" onClick={() => setProposal(null)} disabled={replace.isPending}>
              {t('releases.meta.keepMine')}
            </Button>
            <Button
              size="sm"
              variant="primary"
              onClick={() => replace.mutate(proposal)}
              disabled={replace.isPending}
            >
              {t('releases.meta.replace')}
            </Button>
          </>
        ) : (
          canFill && (
            <Button
              size="sm"
              // Shown first only when there is something to lose. On an empty
              // release the preview would be a step between the person and
              // the thing they obviously want.
              onClick={() => (anything ? preview.mutate() : generate.mutate())}
              disabled={busy}
            >
              <Sparkles aria-hidden />
              {anything ? t('releases.meta.regenerate') : t('releases.meta.generate')}
            </Button>
          )
        )}
      </header>

      {proposal !== null && <p className="text-xs text-dim">{t('releases.meta.previewing')}</p>}

      {data.map((field) => (
        <ReleaseFieldBox
          key={field.key}
          field={field}
          proposed={proposal?.find((change) => change.key === field.key)?.after ?? null}
          onSave={(value) => {
            if (value === field.value) return
            save.mutate({ [field.key]: value })
          }}
        />
      ))}
    </section>
  )
}

interface BoxProps {
  field: ReleaseFieldValue
  /** What the work would write here, while that is being looked at. */
  proposed: string | null
  onSave: (value: string) => void
}

/**
 * One field's box.
 *
 * The draft is local while it is being typed and is handed over on blur, so
 * the box does not fight a refetch mid-sentence. When the stored value
 * changes underneath - a generation landed - the box takes the new text,
 * because that is what the person just asked for.
 */
function ReleaseFieldBox({ field, proposed, onSave }: BoxProps) {
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

  const label = sayLabel(field.label)
  const shared = {
    value: draft,
    onChange: (event: { target: { value: string } }) => setDraft(event.target.value),
    onBlur: () => onSave(draft),
    placeholder: t('releases.meta.empty'),
    'aria-label': label,
  }

  return (
    // `group`: the copy button shows while the pointer is anywhere over the
    // field, not only once it has found the button.
    <div className="group flex flex-col gap-1">
      <div className="flex min-w-0 items-center gap-2">
        <span className="caption">{label}</span>
        {/* The hint beside the name rather than inside the box: a
            placeholder is gone the moment there is text, and "comma
            separated" is wanted most while the commas are being typed. */}
        {field.hint != null && (
          <span className="min-w-0 truncate text-2xs text-faint">{sayLabel(field.hint)}</span>
        )}
        {counter !== null && (
          // A count, never a refusal: kilna is not the authority on what a
          // platform accepts this month, and a box that refuses to hold the
          // text is a box typed somewhere else.
          <span
            className={cn(
              'ml-auto font-mono text-2xs tabular-nums',
              excess !== null ? 'text-bad' : 'text-faint',
            )}
          >
            {counter}
          </span>
        )}
        {/* The text handed over is what is in the box, not what is stored:
            pressing Copy is what takes the focus from a field being written,
            and its save has not landed by then - copying the stored value put
            the text from before the edit on the clipboard, under a toast that
            said it was copied. */}
        <CopyButton
          value={draft}
          label={t('releases.meta.copy')}
          copiedLabel={t('releases.meta.copied', { label })}
          onCopy={(ok) => {
            if (!ok) say.failed(t('releases.meta.copyFailed'))
          }}
          disabled={draft.trim() === ''}
          title={t('releases.meta.copy')}
          className={cn(counter === null && 'ml-auto')}
        />
      </div>

      {field.type === 'line' || field.type === 'tags' ? (
        <Input {...shared} />
      ) : (
        <Textarea {...shared} autoResize maxRows={12} rows={3} />
      )}

      {proposed !== null && (
        // What would replace the box above, under it rather than in it: the
        // two are read against each other, and a box already holding the new
        // text would leave nothing to compare it with.
        <div className="flex flex-col gap-1 rounded-md border border-dashed border-accent bg-accent-soft px-2.5 py-1.5">
          {/* A caption in the accent: `caption` carries its own colour. */}
          <span className="text-2xs font-semibold uppercase tracking-caption text-accent">
            {t('releases.meta.proposed')}
          </span>
          <p className="selectable text-sm whitespace-pre-wrap">
            {proposed === '' ? (
              <span className="text-faint">{t('releases.meta.empty')}</span>
            ) : (
              proposed
            )}
          </p>
        </div>
      )}
    </div>
  )
}
