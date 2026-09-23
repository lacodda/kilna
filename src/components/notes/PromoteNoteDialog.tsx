import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Sprout } from 'lucide-react'
import { promoteNote, type Note } from '@/lib/api'
import { announceEdited } from '@/lib/edited'
import { titleOf } from '@/lib/notes'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/ui/AppDialog'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/AppSelect'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** The note as it stands, with any unsaved text already in it. */
  note: Note
  /** Where to go once the work exists: its card. */
  onPromoted: (workId: string) => void
}

/**
 * An idea grown up into a work.
 *
 * Asks the two things a work cannot be made without — what kind it is and what
 * it is called — and says where the text will go, because the note itself will
 * not stay: its body becomes the work's first version and the note goes to the
 * trash, so the same words do not live in two places. Undo takes it all back.
 */
export function PromoteNoteDialog({ open, onOpenChange, note, onPromoted }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const { config } = useProfile()
  const kinds = config.work_kinds

  const [kind, setKind] = useState(kinds[0]?.key ?? '')
  const [title, setTitle] = useState('')
  const [openedFor, setOpenedFor] = useState<string | null>(null)

  // Each opening starts from the note's own name, not what was typed and
  // abandoned last time.
  if (open && openedFor !== note.id) {
    setOpenedFor(note.id)
    setTitle(titleOf(note))
  } else if (!open && openedFor !== null) {
    setOpenedFor(null)
  }

  const role = vocabularyOf(config, kind).version_roles.find(
    (one) => one.counts_as_version ?? one.comments_on == null,
  )

  const promote = useMutation({
    mutationFn: () => promoteNote(note.id, { kind, title: title.trim() }),
    onSuccess: (promoted) => {
      announceEdited({
        client,
        message: t('notes.promoted', { title: title.trim() }),
        refresh: [
          keys.notes,
          keys.tags,
          keys.works,
          keys.workspace,
          keys.catalogue,
          keys.deletions,
          keys.journal,
        ],
      })
      onOpenChange(false)
      onPromoted(promoted.work_id)
    },
    onError: (cause) => say.failedTo(t('notes.promoteFailed'), cause),
  })

  const ready = kind !== '' && title.trim() !== '' && role !== undefined

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('notes.promoteTitle')}
      description={
        role === undefined
          ? t('notes.promoteNoRole')
          : t('notes.promoteBody', { role: sayLabel(role.label) })
      }
      footer={
        <Button
          type="submit"
          form="promote-note"
          variant="primary"
          disabled={!ready || promote.isPending}
        >
          <Sprout aria-hidden />
          {t('notes.promote')}
        </Button>
      }
    >
      <form
        id="promote-note"
        className="flex flex-col gap-3"
        onSubmit={(event) => {
          event.preventDefault()
          if (ready) promote.mutate()
        }}
      >
        {kinds.length > 1 && (
          <Select
            value={kind}
            onChange={(next) => {
              if (next !== '') setKind(next)
            }}
            options={kinds.map((one) => ({ value: one.key, label: sayLabel(one.label) }))}
            aria-label={t('notes.promoteKind')}
          />
        )}
        <Input
          autoFocus
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          placeholder={t('notes.promoteTitlePlaceholder')}
          aria-label={t('notes.promoteTitlePlaceholder')}
        />
      </form>
    </Dialog>
  )
}
