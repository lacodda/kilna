import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { createNote } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/ui/AppDialog'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/AppSelect'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** The answer being kept, verbatim. */
  body: string
  /** The work the note hangs on. Absent in a chat about nothing: the note is
      then the workspace's, which is what a note about the craft rather than
      about one song actually is. */
  workId?: string
}

/**
 * Turns an assistant answer into a note.
 *
 * The manual counterpart of an agent's note proposal, and the same shape as
 * inserting a version: the answer is kept word for word, the person says what
 * kind of note it is and what to call it, and the application writes. An
 * answer worth keeping is very often not a draft of anything — a list of
 * references, a piece of lore, a thing to remember — and before this there was
 * nowhere for it to go but the clipboard.
 */
export function KeepAsNoteDialog({ open, onOpenChange, body, workId }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const { config } = useProfile()

  const kinds = config.note_kinds ?? []
  const [kind, setKind] = useState(kinds[0]?.key ?? '')
  const [title, setTitle] = useState('')

  const keep = useMutation({
    mutationFn: () =>
      createNote({
        body,
        work_id: workId,
        kind: kind === '' ? null : kind,
        title: title.trim() === '' ? null : title.trim(),
      }),
    onSuccess: () => {
      for (const key of [keys.notes, keys.tags, keys.journal]) {
        void client.invalidateQueries({ queryKey: key })
      }
      say.ok(t('assistant.noteKept'))
      onOpenChange(false)
    },
    onError: (cause) => {
      say.failedTo(t('assistant.noteKeepFailed'), cause)
    },
  })

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('assistant.keepAsNoteTitle')}
      description={t('assistant.keepAsNoteBody')}
      footer={
        <Button
          variant="primary"
          disabled={keep.isPending}
          onClick={() => {
            keep.mutate()
          }}
        >
          {t('assistant.keepAsNote')}
        </Button>
      }
    >
      <div className="flex flex-col gap-3">
        {/* Only when the craft names kinds of note. One that names none takes
            any word a person writes, and an empty picker would be a control
            asking a question with no answers. */}
        {kinds.length > 0 && (
          <Select
            value={kind}
            onChange={setKind}
            options={kinds.map((one) => ({ value: one.key, label: sayLabel(one.label) }))}
            aria-label={t('assistant.noteKind')}
          />
        )}
        <Input
          value={title}
          onChange={(event) => {
            setTitle(event.target.value)
          }}
          placeholder={t('assistant.noteTitlePlaceholder')}
          aria-label={t('assistant.noteTitlePlaceholder')}
        />
      </div>
    </Dialog>
  )
}
