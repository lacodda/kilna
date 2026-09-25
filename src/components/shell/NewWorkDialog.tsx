import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { X } from 'lucide-react'
import { createWork } from '@/lib/api'
import { keys } from '@/lib/query'
import { useTypedSinceOpen } from '@/lib/dialogGuard'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import {
  Dialog,
  DialogActions,
  DialogBody,
  DialogClose,
  DialogHeader,
  DialogPopup,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/Field'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/AppSelect'

interface Props {
  /** The kind the dialog opens on; `null` keeps it closed. */
  kind: string | null
  onClose: () => void
  /** Called with the new work's id, after the toast. */
  onCreated: (workId: string) => void
}

/**
 * The one door through which a work is added.
 *
 * Until v0.74 the catalogue had a title box and an Add button at its top,
 * which took the first forty pixels of every visit to a list that is mostly
 * read, and which the calendar and the dashboard did not have at all. The
 * title bar's New button opens this from anywhere instead. The kind is chosen
 * before the dialog opens - the menu under the button names them - and can be
 * changed here, because "Video" pressed by mistake should not mean cancelling.
 */
export function NewWorkDialog({ kind, onClose, onCreated }: Props) {
  // A title typed is a title a stray click beside the dialog would lose, as
  // in every other dialog of the app.
  const { typed, onInput } = useTypedSinceOpen(kind)

  return (
    <Dialog
      open={kind !== null}
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
      disablePointerDismissal={typed}
    >
      <DialogPopup onInput={onInput}>
        {/* Keyed by the kind it opened on, so each opening starts from a blank
            title and the kind that was picked: the last title typed here
            belongs to the work it made, not to the next one. */}
        {kind !== null && <Form key={kind} kind={kind} onClose={onClose} onCreated={onCreated} />}
      </DialogPopup>
    </Dialog>
  )
}

function Form({
  kind,
  onClose,
  onCreated,
}: {
  kind: string
  onClose: () => void
  onCreated: (workId: string) => void
}) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const profile = useProfile()
  const [title, setTitle] = useState('')
  const [chosen, setChosen] = useState(kind)

  const add = useMutation({
    mutationFn: createWork,
    onSuccess: (work) => {
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.catalogue })
      void client.invalidateQueries({ queryKey: keys.workspace })
      void client.invalidateQueries({ queryKey: keys.journal })
      say.ok(t('toast.workCreated'))
      onClose()
      // Straight into the new work: adding one is the start of writing it, not
      // an entry in a list to admire.
      onCreated(work.id)
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

  const submit = () => {
    const trimmed = title.trim()
    if (trimmed === '' || add.isPending) return
    add.mutate({ kind: chosen, title: trimmed })
  }

  const kinds = profile.config.work_kinds
  const kindLabel = kinds.find((entry) => entry.key === chosen)?.label ?? ''

  return (
    <form
      // The form is the popup's column: the header and the actions stay put
      // and only the fields between them scroll, as in every other dialog.
      className="flex min-h-0 flex-1 flex-col"
      onSubmit={(event) => {
        event.preventDefault()
        submit()
      }}
    >
      <DialogHeader
        action={
          <DialogClose
            render={<Button variant="icon" size="icon-sm" aria-label={t('dialog.close')} />}
          >
            <X aria-hidden />
          </DialogClose>
        }
      >
        <DialogTitle>
          {kinds.length > 1 ? t('works.newOfKind', { kind: kindLabel }) : t('works.newTitle')}
        </DialogTitle>
      </DialogHeader>
      <DialogBody className="flex flex-col gap-3">
        <Field label={t('works.title')}>
          <Input
            autoFocus
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder={t('works.newPlaceholder')}
          />
        </Field>
        {kinds.length > 1 && (
          <Field label={t('works.kind')}>
            <Select
              value={chosen}
              onChange={setChosen}
              options={kinds.map((entry) => ({ value: entry.key, label: sayLabel(entry.label) }))}
            />
          </Field>
        )}
      </DialogBody>
      <DialogActions>
        <DialogClose render={<Button />}>{t('dialog.cancel')}</DialogClose>
        <Button variant="primary" type="submit" disabled={title.trim() === '' || add.isPending}>
          {t('works.add')}
        </Button>
      </DialogActions>
    </form>
  )
}
