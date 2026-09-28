import { useCallback, useEffect, useId, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { Sparkles, Trash2, X } from 'lucide-react'
import { deleteStyleBrick, startStyleTask } from '@/lib/api/styles'
import type { StyleBrick, StyleBrickStatus, StyleType } from '@/lib/api/types'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { nameProblem, type StyleForm } from '@/lib/styleDraft'
import { styleIconOf } from '@/lib/styleIcon'
import { announceDeleted } from '@/lib/trash'
import { useAssistant } from '@/lib/useAssistant'
import { say as sayLabel } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { SaveState } from '@/components/ui/save-state'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Textarea } from '@/components/ui/textarea'
import { Pane } from '@/components/frame'
import { StyleReferences } from '@/features/styles/StyleReferences'
import { useStyleDraft } from '@/features/styles/useStyleDraft'

/** The action of the profile that writes a description from references. */
const DESCRIBE = 'describe-style'

const STATUSES = ['draft', 'ready', 'dropped'] as const satisfies readonly StyleBrickStatus[]

interface Props {
  brick: StyleBrick
  types: StyleType[]
  /** Every style of the dictionary, for the names a type already has. */
  bricks: StyleBrick[]
  /** Open with the name selected: a style just made is named first. */
  naming: boolean
  onClose: () => void
}

/**
 * A style, open beside the dictionary: its name along the top, its pictures,
 * its type, what it says and the steer for whoever says it, where it stands
 * along the foot.
 *
 * A panel of the screen rather than a dialog, since v0.79. The dialog was a
 * long form in a 448px box whose footer scrolled away with it; it lost what
 * was typed to a stray click, and "Describe" closed it and threw the edits
 * away. Here nothing needs saving - every field writes itself a moment after
 * the typing pauses - and describing leaves the style open, so the
 * description the assistant writes arrives in the box it is meant for.
 */
export function StyleEditor({ brick, types, bricks, naming, onClose }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const assistant = useAssistant()
  const zone = useRef<HTMLDivElement>(null)
  const nameBox = useRef<HTMLInputElement>(null)
  const problemId = useId()

  // The names a type already has, other than this style's own.
  const namesOf = useCallback(
    (typeKey: string) =>
      bricks
        .filter((one) => one.type_key === typeKey && one.id !== brick.id)
        .map((one) => one.name),
    [bricks, brick.id],
  )
  const holdKey = useCallback(
    (form: StyleForm) => nameProblem(form.name, namesOf(form.type_key)) !== null,
    [namesOf],
  )

  const draft = useStyleDraft(brick, { holdKey, failure: t('styles.saveFailed') })
  const { form, edit, flush, forget } = draft
  const problem = nameProblem(form.name, namesOf(form.type_key))

  // A style just made is called "Untitled style"; the first thing to do with
  // it is to call it something else, so the name is ready to be typed over.
  useEffect(() => {
    if (!naming) return
    nameBox.current?.focus()
    nameBox.current?.select()
  }, [naming])

  // Into the trash with its references, the road every other deletion takes;
  // the toast offers the way back. What is pending goes in first, so a
  // restore brings back the last word.
  const remove = useAppMutation({
    mutationFn: async () => {
      await flush()
      return deleteStyleBrick(brick.id)
    },
    onSuccess: (deletionId) => {
      forget()
      announceDeleted({
        client,
        deletionId,
        // The name it went under: the one typed, unless that one was held
        // back and the stored one is what the trash will show.
        message: t('styles.deleted', { name: problem === null ? form.name.trim() : brick.name }),
        refresh: refresh.style,
      })
      onClose()
    },
  })

  const describe = useAppMutation({
    mutationFn: async () => {
      // Describing reads the stored style: the steer typed a moment ago has
      // to be there before the task reads it.
      await flush()
      return startStyleTask(brick.id, DESCRIBE)
    },
    // The answer lands in its own chat, with the button that keeps it as the
    // description (wish 3034); the drawer opens on it so the person watches
    // it arrive. The style stays open, and takes the description in when it
    // is kept.
    onSuccess: (started) => assistant.open(started.chatId),
  })

  const hasReferences = brick.reference_count > 0

  return (
    <div
      ref={zone}
      // A drag over any part of the open style is a drag onto its references
      // (see StyleReferences), and the outline says so where it can be seen.
      className="flex min-h-0 min-w-0 flex-1 flex-col rounded-lg outline-offset-2 has-data-dropping:outline-2 has-data-dropping:outline-accent has-data-dropping:outline-dashed"
      onKeyDown={(event) => {
        // Escape closes the style, as it closed the dialog it replaces:
        // there is nothing to lose, everything typed is written.
        if (event.key !== 'Escape' || event.defaultPrevented) return
        event.preventDefault()
        onClose()
      }}
    >
      <Pane
        label={form.name.trim() === '' ? t('styles.untitled') : form.name}
        bodyClassName="flex flex-col gap-5 p-4"
        head={
          <>
            <Input
              ref={nameBox}
              value={form.name}
              onChange={(event) => edit({ name: event.target.value })}
              placeholder={t('styles.untitled')}
              aria-label={t('styles.name')}
              aria-invalid={problem !== null}
              aria-describedby={problem === null ? undefined : problemId}
              className="min-w-40 flex-1 border-transparent bg-transparent px-1.5 text-sm font-semibold hover:border-line focus:border-line"
            />
            <SaveState
              status={draft.status}
              savingLabel={t('save.saving')}
              savedLabel={t('save.saved')}
            />
            <Button
              size="icon-sm"
              variant="danger"
              aria-label={t('styles.delete')}
              title={t('styles.delete')}
              disabled={remove.isPending}
              onClick={() => remove.mutate()}
            >
              <Trash2 aria-hidden />
            </Button>
            <Button
              size="icon-sm"
              variant="icon"
              aria-label={t('styles.close')}
              title={t('styles.close')}
              onClick={onClose}
            >
              <X aria-hidden />
            </Button>
          </>
        }
        foot={
          <SegmentedControl
            aria-label={t('styles.statusLabel')}
            value={form.status}
            onValueChange={(next) => edit({ status: next as StyleBrickStatus }, true)}
          >
            {STATUSES.map((one) => (
              <Segment key={one} value={one}>
                {t(`styles.status.${one}`)}
              </Segment>
            ))}
          </SegmentedControl>
        }
      >
        {/* Said under the name rather than as a failed save: the name is
            held back until it is one the dictionary can take, and the rest
            of the style goes on saving meanwhile. */}
        {problem !== null && (
          <p id={problemId} role="alert" className="text-xs text-bad">
            {problem === 'missing'
              ? t('styles.nameMissing')
              : t('styles.nameTaken', { name: form.name.trim() })}
          </p>
        )}

        {/* The pictures first: a style is recognised by what it looks like,
            and they are what the description is read from. */}
        <StyleReferences brickId={brick.id} zone={zone} />

        {/* The type decides which question the description answers. A group,
            named by its caption: a row of chips has no one control a label
            could point at. */}
        <FieldGroup label={t('styles.type')} help={hintOf(types, form.type_key)}>
          <ChipGroup
            value={[form.type_key]}
            // A style always has a type: pressing the one that is on leaves
            // the group empty, and that is not a choice to keep.
            onValueChange={([next]) => {
              if (next !== undefined) edit({ type_key: next }, true)
            }}
          >
            {types.map((one) => {
              const Icon = styleIconOf(one)
              return (
                <Chip key={one.key} value={one.key}>
                  <Icon aria-hidden className="size-3.5" />
                  {sayLabel(one.label)}
                </Chip>
              )
            })}
          </ChipGroup>
        </FieldGroup>

        <div className="flex flex-col gap-2">
          <Field label={t('styles.description')} help={t('styles.descriptionHint')}>
            <Textarea
              autoResize
              rows={6}
              value={form.description}
              onChange={(event) => edit({ description: event.target.value })}
              className="leading-relaxed"
            />
          </Field>
          <Button
            variant="soft"
            size="sm"
            className="self-start"
            onClick={() => describe.mutate()}
            disabled={describe.isPending || !hasReferences}
            disabledReason={hasReferences ? undefined : t('styles.describeNeedsReferences')}
          >
            <Sparkles aria-hidden />
            {t('styles.describe')}
          </Button>
        </div>

        <Field label={t('styles.hint')} help={t('styles.hintHint')}>
          <Input value={form.hint} onChange={(event) => edit({ hint: event.target.value })} />
        </Field>
      </Pane>
    </div>
  )
}

/** The type's own instruction, shown under the picker: it is what the
    description will be held to, so it belongs where it is being chosen. */
function hintOf(types: StyleType[], key: string): string | undefined {
  const hint = types.find((one) => one.key === key)?.hint
  return hint === undefined || hint === null ? undefined : sayLabel(hint)
}
