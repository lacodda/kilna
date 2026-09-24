import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { open as openFile } from '@tauri-apps/plugin-dialog'
import { Sparkles, Trash2, X } from 'lucide-react'
import {
  attachAsset,
  createStyleBrick,
  deleteStyleBrick,
  detachAsset,
  fileSrc,
  pasteStyleReference,
  startStyleTask,
  styleBrickReferences,
  updateStyleBrick,
  type StyleBrick,
  type StyleBrickStatus,
  type StyleType,
} from '@/lib/api'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { say as sayLabel } from '@/lib/useProfile'
import { useAssistant } from '@/lib/useAssistant'
import { styleIconOf } from '@/lib/styleIcon'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/ui/AppDialog'
import { Field } from '@/components/ui/Field'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { cn } from '@/lib/utils'

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

  const references = useQuery({
    queryKey: keys.styleReferences(id ?? ''),
    queryFn: () => styleBrickReferences(id ?? ''),
    enabled: id !== undefined,
  })

  const settle = () => {
    void client.invalidateQueries({ queryKey: keys.styles })
    onSettled()
  }

  const save = useMutation({
    mutationFn: async () => {
      if (id === undefined) return createStyleBrick(written)
      return updateStyleBrick(id, { ...written, status })
    },
    onSuccess: () => {
      settle()
      // Said, with the way back: both a new brick and an edit are operations
      // undo can take back, and a save that closes the dialog in silence left
      // the person to guess whether it happened.
      announceEdited({ client, message: t('styles.saved'), refresh: [keys.styles] })
      onOpenChange(false)
    },
    onError: (cause: unknown) => say.failed(cause),
  })

  // Into the trash with its references, the road every other deletion takes;
  // the toast offers the way back.
  const remove = useMutation({
    mutationFn: () => deleteStyleBrick(id ?? ''),
    onSuccess: (deletionId) => {
      settle()
      announceDeleted({
        client,
        deletionId,
        message: t('styles.deleted', { name: brick?.name ?? '' }),
        refresh: [keys.styles],
      })
      onOpenChange(false)
    },
    onError: (cause: unknown) => say.failed(cause),
  })

  const attach = useMutation({
    mutationFn: async (paths: string[]) => {
      for (const path of paths) {
        await attachAsset(path, { style_brick_id: id })
      }
    },
    onSuccess: settle,
    onError: (cause: unknown) => say.failed(cause),
  })

  const paste = useMutation({
    mutationFn: ({ bytes, fileName }: { bytes: number[]; fileName: string }) =>
      pasteStyleReference(id ?? '', bytes, fileName),
    onSuccess: settle,
    onError: (cause: unknown) => say.failed(cause),
  })

  const detach = useMutation({
    mutationFn: (assetId: string) => detachAsset(assetId),
    onSuccess: settle,
    onError: (cause: unknown) => say.failed(cause),
  })

  const describe = useMutation({
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
    onError: (cause: unknown) => say.failed(cause),
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
      className="max-w-2xl"
      /* Deleting is not an answer to the dialog, so it stands apart at the
         start of the row rather than between Cancel and Save. */
      aside={
        id === undefined ? undefined : (
          <Button variant="danger" onClick={() => remove.mutate()} disabled={remove.isPending}>
            <Trash2 aria-hidden className="size-4" />
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
            so picking it afterwards would mean rewriting what was written. */}
        <Field label={t('styles.type')} hint={hintOf(types, typeKey)}>
          <div className="flex flex-wrap gap-2">
            {types.map((one) => {
              const Icon = styleIconOf(one)
              const active = typeKey === one.key
              return (
                <button
                  key={one.key}
                  type="button"
                  aria-pressed={active}
                  onClick={() => setTypeKey(one.key)}
                  className={cn(
                    'flex cursor-pointer items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-sm transition-colors',
                    active
                      ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                      : 'border-line text-dim hover:border-line-2 hover:text-text',
                  )}
                >
                  <Icon aria-hidden className="size-4" />
                  {sayLabel(one.label)}
                </button>
              )
            })}
          </div>
        </Field>

        <Field label={t('styles.name')}>
          <Input value={name} onChange={(event) => setName(event.target.value)} />
        </Field>

        {id !== undefined && (
          <Field label={t('styles.referenceList')} hint={t('styles.referenceHint')}>
            <div className="flex flex-wrap items-center gap-2">
              {(references.data ?? []).map((asset) => (
                <div key={asset.id} className="group relative">
                  <img
                    src={fileSrc(asset.path)}
                    alt={asset.original_name ?? ''}
                    className="size-20 rounded-lg object-cover"
                    draggable={false}
                  />
                  <button
                    type="button"
                    onClick={() => detach.mutate(asset.id)}
                    aria-label={t('styles.removeReference')}
                    className="absolute -right-1.5 -top-1.5 cursor-pointer rounded-full border border-line bg-raise p-0.5 opacity-0 transition-opacity group-hover:opacity-100"
                  >
                    <X aria-hidden className="size-3" />
                  </button>
                </div>
              ))}
              <Button variant="ghost" onClick={() => void choose()} disabled={attach.isPending}>
                {t('styles.addReference')}
              </Button>
            </div>
          </Field>
        )}

        <Field label={t('styles.description')} hint={t('styles.descriptionHint')}>
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
            <Sparkles aria-hidden className="size-4" />
            {t('styles.describe')}
          </Button>
        )}

        <Field label={t('styles.hint')} hint={t('styles.hintHint')}>
          <Input value={hint} onChange={(event) => setHint(event.target.value)} />
        </Field>

        {id !== undefined && (
          <Field label={t('styles.statusLabel')}>
            <div className="flex gap-2">
              {(['draft', 'ready', 'dropped'] as const).map((one) => (
                <button
                  key={one}
                  type="button"
                  aria-pressed={status === one}
                  onClick={() => setStatus(one)}
                  className={cn(
                    'cursor-pointer rounded-lg border px-2.5 py-1 text-sm transition-colors',
                    status === one
                      ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                      : 'border-line text-dim hover:border-line-2 hover:text-text',
                  )}
                >
                  {t(`styles.status.${one}`)}
                </button>
              ))}
            </div>
          </Field>
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
