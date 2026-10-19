import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { Sprout } from 'lucide-react'
import { promoteNote } from '@/lib/api/notes'
import type { Note } from '@/lib/api/types'
import { announceEdited } from '@/lib/edited'
import { isMaterial, titleOf } from '@/lib/notes'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/AppDialog'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/AppSelect'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** The note as it stands, with any unsaved text already in it. */
  note: Note
  /** Where to go once the work exists: its card. */
  onPromoted: (workId: string) => void
}

/**
 * A note grown up into a work.
 *
 * Asks the two things a work cannot be made without — what kind it is and what
 * it is called — and says what becomes of the note: it stays, attached to the
 * new work, as what the work grows from (ADR 0064). Nothing is copied, so the
 * words live in one place. Undo takes it all back.
 *
 * A phrase is spent as well: marked used, so it is not taken twice (ADR 0045)
 * - and the dialog says so.
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

  const promote = useAppMutation({
    mutationFn: () => promoteNote(note.id, { kind, title: title.trim() }),
    failure: 'notes.promoteFailed',
    onSuccess: (promoted) => {
      announceEdited({
        client,
        message: t('notes.promoted', { title: title.trim() }),
        refresh: [keys.notes, keys.tags, keys.works, keys.workspace, keys.catalogue, keys.journal],
      })
      onOpenChange(false)
      onPromoted(promoted.work_id)
    },
  })

  const ready = kind !== '' && title.trim() !== ''

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('notes.promoteTitle')}
      description={
        isMaterial(config, note.kind) ? t('notes.promoteBodyMaterial') : t('notes.promoteBody')
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
