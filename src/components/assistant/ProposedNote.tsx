import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Check } from 'lucide-react'
import { createNote, type NoteProposal } from '@/lib/api'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'

interface Props {
  /** The work the note is about; absent for a note on nothing in particular. */
  workId?: string
  proposal: NoteProposal
  /** The note itself — the message body, verbatim. */
  body: string
}

/**
 * A note an agent proposed, next to the button that keeps it.
 *
 * Same bargain as a proposed score: the text is readable before it is
 * written, and a person writes it. Kept, it is an ordinary note — the same
 * table, no mark saying a machine suggested it — and the toast offers to
 * take it back, since adding a note is one of the things undo covers.
 */
export function ProposedNote({ workId, proposal, body }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [kept, setKept] = useState(false)

  const keep = useMutation({
    mutationFn: () =>
      createNote({
        body,
        title: proposal.title ?? null,
        work_id: workId ?? null,
      }),
    onSuccess: () => {
      setKept(true)
      const refresh = [keys.notes, keys.journal]
      announceEdited({ client, message: t('assistant.noteAdded'), refresh })
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  return (
    <div className="mx-3 flex flex-wrap items-center gap-2 rounded-xl border border-line px-3 py-2 text-sm">
      <span className="text-dim">
        {t('assistant.proposedNote')}
        {proposal.title != null && proposal.title !== '' && (
          <>
            {' · '}
            <b className="font-semibold text-text">{proposal.title}</b>
          </>
        )}
      </span>
      <Button
        className="ml-auto"
        size="sm"
        variant={kept ? 'icon' : 'primary'}
        disabled={kept || keep.isPending}
        onClick={() => keep.mutate()}
      >
        {kept ? <Check aria-hidden className="size-3.5" /> : null}
        {kept ? t('assistant.noteKept') : t('assistant.addNote')}
      </Button>
    </div>
  )
}
