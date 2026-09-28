import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { open as openFile } from '@tauri-apps/plugin-dialog'
import { Sparkles, Trash2, X } from 'lucide-react'
import { attachAsset, detachAsset, fileSrc } from '@/lib/api/assets'
import {
  createStyleBrick,
  deleteStyleBrick,
  pasteStyleReference,
  startStyleTask,
  updateStyleBrick,
} from '@/lib/api/styles'
import type { StyleBrick, StyleBrickStatus, StyleType } from '@/lib/api/types'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { say as sayLabel } from '@/lib/useProfile'
import { useAssistant } from '@/lib/useAssistant'
import { styleIconOf } from '@/lib/styleIcon'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Dialog } from '@/components/AppDialog'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Textarea } from '@/components/ui/textarea'

/** The action of the profile that writes a description from references. */
const DESCRIBE = 'describe-style'

/**
 * A brick, open: its type, its name, its references, what it says, and the
 * button that writes what it says from what it shows.
 *
 * One dialog for making and for editing. A brick is born on the first save
 * rather than on opening, so closing the "new" dialog leaves nothing behind —
 * but the references need a brick to hang on, so they are only offered once
 * there is one.
 */
export function StyleBrickDialog({
  open,
  brick,
  types,
  onOpenChange,
  onSettled,
}: {
  open: boolean
  /** Absent for a brick being made. */
  brick?: StyleBrick
  types: StyleType[]
  onOpenChange: (open: boolean) => void
  onSettled: () => void
}) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const assistant = useAssistant()

  const [typeKey, setTypeKey] = useState(brick?.type_key ?? types[0]?.key ?? '')
  const [name, setName] = useState(brick?.name ?? '')
  const [description, setDescription] = useState(brick?.description ?? '')
  const [hint, setHint] = useState(brick?.hint ?? '')
  const [status, setStatus] = useState<StyleBrickStatus>(brick?.status ?? 'draft')

  const id = brick?.id

  // The form as the backend would write it, and whether it differs from the
  // brick as stored. Describing reads the stored brick, so an edit not yet
  // saved has to be saved first - or the steer typed a moment ago is ignored
  // and the name, type and status typed with it are thrown away when the
  // dialog closes.
  const written = {
    type_key: typeKey,
    name: name.trim(),
    description: description.trim() === '' ? null : description.trim(),
    hint: hint.trim() === '' ? null : hint.trim(),
  }
  const dirty =
    brick !== undefined &&
    (written.type_key !== brick.type_key ||
      written.name !== brick.name ||
      written.description !== brick.description ||
      written.hint !== brick.hint ||
      status !== brick.status)

  const references = useQuery({ ...queries.styleReferences(id ?? ''), enabled: id !== undefined })

  // Only `describe` still calls this directly: it invalidates ahead of a save
  // folded into its own mutation, before the task it starts even lands.
  const settle = () => {
    void client.invalidateQueries({ queryKey: keys.styles })
    onSettled()
  }

  const save = useAppMutation({
    mutationFn: async () => {
      if (id === undefined) return createStyleBrick(written)
      return updateStyleBrick(id, { ...written, status })
    },
    refresh: refresh.style,
    onSuccess: () => {
      onSettled()
      // Said, with the way back: both a new brick and an edit are operations
      // undo can take back, and a save that closes the dialog in silence left
      // the person to guess whether it happened.
      announceEdited({ client, message: t('styles.saved'), refresh: refresh.style })
      onOpenChange(false)
    },
  })

  // Into the trash with its references, the road every other deletion takes;
  // the toast offers the way back.
  const remove = useAppMutation({
    mutationFn: () => deleteStyleBrick(id ?? ''),
    onSuccess: (deletionId) => {
      onSettled()
      announceDeleted({
        client,
        deletionId,
        message: t('styles.deleted', { name: brick?.name ?? '' }),
        refresh: refresh.style,
      })
      onOpenChange(false)
    },
  })

  const attach = useAppMutation({
    mutationFn: async (paths: string[]) => {
      for (const path of paths) {
        await attachAsset(path, { style_brick_id: id })
      }
    },
    refresh: refresh.style,
    onSuccess: onSettled,
  })

  const paste = useAppMutation({
    mutationFn: ({ bytes, fileName }: { bytes: number[]; fileName: string }) =>
      pasteStyleReference(id ?? '', bytes, fileName),
    refresh: refresh.style,
    onSuccess: onSettled,
  })

  const detach = useAppMutation({
    mutationFn: (assetId: string) => detachAsset(assetId),
    refresh: refresh.style,
    onSuccess: onSettled,
  })

  const describe = useAppMutation({
    mutationFn: async () => {
      if (dirty && id !== undefined) {
        await updateStyleBrick(id, { ...written, status })
        settle()
      }
      return startStyleTask(id ?? '', DESCRIBE)
    },
    onSuccess: (started) => {
      // The answer lands in its own chat; the panel opens on it so the person
      // watches it arrive rather than wondering where it went.
      assistant.open(started.chatId)
      onOpenChange(false)
    },
  })

  // A reference pasted straight from a generator's tab, the way a frame is.
  // The listener lives as long as the dialog does, so Ctrl+V elsewhere keeps
  // its usual meaning.
  const onPaste = useCallback(
    (event: ClipboardEvent) => {
      if (id === undefined) return
      const file = Array.from(event.clipboardData?.files ?? [])[0]
      if (file === undefined || !file.type.startsWith('image/')) return
      event.preventDefault()
      void file.arrayBuffer().then((buffer) => {
        const extension = file.type.split('/')[1] ?? 'png'
        paste.mutate({
          bytes: Array.from(new Uint8Array(buffer)),
          fileName: file.name || `pasted-${Date.now()}.${extension}`,
        })
      })
    },
    [id, paste],
  )

  useEffect(() => {
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
  }, [onPaste])

  const choose = async () => {
    const picked = await openFile({
      multiple: true,
      filters: [{ name: t('styles.pictures'), extensions: ['png', 'jpg', 'jpeg', 'webp', 'avif'] }],
    })
    if (picked === null) return
    attach.mutate(Array.isArray(picked) ? picked : [picked])
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={id === undefined ? t('styles.new') : t('styles.edit')}
      size="xl"
      /* Picking a type is a change a stray click would throw away as surely
         as a name typed; the dialog notices typing on its own. */
      dirty={dirty || (brick === undefined && typeKey !== (types[0]?.key ?? ''))}
      /* Deleting is not an answer to the dialog, so it stands apart at the
         start of the row rather than between Cancel and Save. */
      aside={
        id === undefined ? undefined : (
          <Button variant="danger" onClick={() => remove.mutate()} disabled={remove.isPending}>
            <Trash2 aria-hidden />
            {t('work.delete')}
          </Button>
        )
      }
      /* Cancel is the dialog's own, always first in the row — only what is
         particular to this one goes here. */
      footer={
        <Button
          variant="primary"
          onClick={() => save.mutate()}
          disabled={save.isPending || name.trim() === '' || typeKey === ''}
        >
          {t('dialog.save')}
        </Button>
      }
    >
      <div className="flex flex-col gap-4">
        {/* The type first: it decides which question the description answers,
            so picking it afterwards would mean rewriting what was written.
            A group, named by its caption: a row of chips has no one control
            a label could point at. */}
        <FieldGroup label={t('styles.type')} help={hintOf(types, typeKey)}>
          <ChipGroup
            value={[typeKey]}
            // A brick always has a type: pressing the one that is on leaves
            // the group empty, and that is not a choice to keep.
            onValueChange={([next]) => {
              if (next !== undefined) setTypeKey(next)
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

        <Field label={t('styles.name')}>
          <Input value={name} onChange={(event) => setName(event.target.value)} />
        </Field>

        {id !== undefined && (
          <FieldGroup label={t('styles.referenceList')} help={t('styles.referenceHint')}>
            <div className="flex flex-wrap items-center gap-2">
              {(references.data ?? []).map((asset) => (
                <div key={asset.id} className="group relative">
                  <img
                    src={fileSrc(asset.path)}
                    alt={asset.original_name ?? ''}
                    className="size-20 rounded-lg object-cover"
                    draggable={false}
                  />
                  {/* The ground under the cross is this corner's, not the
                      button's: over a picture a bare glyph is lost. Shown on
                      hover, and whenever the cross has the keyboard. */}
                  <span className="absolute -top-1.5 -right-1.5 rounded-md border border-line bg-raise opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                    <Button
                      variant="icon"
                      size="icon-xs"
                      onClick={() => detach.mutate(asset.id)}
                      aria-label={t('styles.removeReference')}
                    >
                      <X aria-hidden />
                    </Button>
                  </span>
                </div>
              ))}
              <Button variant="ghost" onClick={() => void choose()} disabled={attach.isPending}>
                {t('styles.addReference')}
              </Button>
            </div>
          </FieldGroup>
        )}

        <Field label={t('styles.description')} help={t('styles.descriptionHint')}>
          <Textarea
            rows={7}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
        </Field>

        {id !== undefined && (
          <Button
            variant="ghost"
            onClick={() => describe.mutate()}
            disabled={describe.isPending || references.data?.length === 0}
            className="self-start"
          >
            <Sparkles aria-hidden />
            {t('styles.describe')}
          </Button>
        )}

        <Field label={t('styles.hint')} help={t('styles.hintHint')}>
          <Input value={hint} onChange={(event) => setHint(event.target.value)} />
        </Field>

        {id !== undefined && (
          <FieldGroup label={t('styles.statusLabel')}>
            <SegmentedControl
              aria-label={t('styles.statusLabel')}
              value={status}
              onValueChange={(next) => setStatus(next as StyleBrickStatus)}
            >
              {(['draft', 'ready', 'dropped'] as const).map((one) => (
                <Segment key={one} value={one}>
                  {t(`styles.status.${one}`)}
                </Segment>
              ))}
            </SegmentedControl>
          </FieldGroup>
        )}
      </div>
    </Dialog>
  )
}

/** The type's own instruction, shown under the picker: it is what the
    description will be held to, so it belongs where it is being chosen. */
function hintOf(types: StyleType[], key: string): string | undefined {
  const found = types.find((one) => one.key === key)
  const hint = found?.hint
  return hint === undefined || hint === null ? undefined : sayLabel(hint)
}
