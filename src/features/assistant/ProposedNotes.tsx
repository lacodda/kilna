import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Applied, NotesProposal, ProposedNote } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Checkbox } from '@/components/ui/checkbox'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  messageId: string
  proposal: NotesProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/** The key an apply names the `index`-th note by. */
const itemOf = (index: number) => `note:${String(index)}`

/**
 * Notes an agent proposed several at once, or one that says more than its
 * words (v0.116): tags, a layer, or a note it changes rather than a new one.
 *
 * Note by note, each with a box to leave it out, the way words and a package
 * for the canon are read: one note that misses must not cost the rest. A
 * change is shown against what the note held when it was proposed - its
 * title, its tags, its text - because "the assistant changed your note" is
 * agreed to only by someone who has seen what it changes.
 */
export function ProposedNotes({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const all = proposal.notes.map((_, index) => itemOf(index))
  const [chosen, setChosen] = useState<string[]>(all)
  const answered = applied !== null || dismissed

  const keep = useApplyProposal({
    messageId,
    message: t('assistant.notesKept'),
    refresh: [keys.notes, keys.pendingProposals],
  })

  const toggle = (item: string, on: boolean) =>
    setChosen(on ? [...chosen, item] : chosen.filter((one) => one !== item))

  return (
    <ProposalCard
      messageId={messageId}
      title={t('assistant.proposedNotes')}
      figure={answered ? undefined : String(chosen.length)}
      applied={applied}
      dismissed={dismissed}
      applyLabel={
        chosen.length === all.length
          ? t('assistant.keepNotesAll')
          : t('assistant.keepNotes', { n: chosen.length })
      }
      applying={keep.isPending || chosen.length === 0}
      // Whole when every box is ticked, as words are kept.
      onApply={() => keep.mutate(chosen.length === all.length ? undefined : { items: chosen })}
      appliedLabel={t('assistant.notesKeptMark', { count: applied?.notes?.length ?? 0 })}
    >
      <ul className="flex flex-col gap-2 text-xs">
        {proposal.notes.map((note, index) => {
          const item = itemOf(index)
          return (
            <li key={item} className="flex items-start gap-2">
              {!answered && (
                <Checkbox
                  checked={chosen.includes(item)}
                  onCheckedChange={(on) => toggle(item, on)}
                  aria-label={t('assistant.keepNoteItem', { title: nameOf(note) })}
                  className="mt-0.5"
                />
              )}
              <OneNote note={note} />
            </li>
          )
        })}
      </ul>
    </ProposalCard>
  )
}

/** What a note is called in a list: its title, the title of the note it
 * changes, or its first words. */
function nameOf(note: ProposedNote): string {
  const title = note.title ?? note.before?.title
  if (title !== undefined && title.trim() !== '') return title
  const text = note.body.trim() !== '' ? note.body : (note.before?.body ?? '')
  const first = text.split('\n').find((line) => line.trim() !== '') ?? ''
  return first.trim().slice(0, 80)
}

/** One note: a new one with its words, or a change with what it changes. */
function OneNote({ note }: { note: ProposedNote }) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const before = note.before
  const tags = note.tags ?? []

  const said = [
    note.kind !== undefined ? labelOf(config.note_kinds ?? [], note.kind) : null,
    note.attribution !== undefined ? `— ${note.attribution}` : null,
    note.shelf !== undefined ? t('assistant.noteOnShelf', { shelf: note.shelf }) : null,
    note.layer !== undefined ? t(LAYER_KEYS[note.layer]) : null,
  ].filter((part) => part !== null)

  if (before === undefined) {
    return (
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <b className="text-sm font-semibold text-text">{nameOf(note)}</b>
        {said.length > 0 && <span className="text-dim">{said.join(' · ')}</span>}
        {tags.length > 0 && <span className="text-faint">{tags.join(', ')}</span>}
        {note.title !== undefined && (
          <span className="selectable line-clamp-4 whitespace-pre-wrap text-dim">{note.body}</span>
        )}
      </span>
    )
  }

  const retitled = note.title !== undefined && note.title !== (before.title ?? '')
  const beforeTags = before.tags ?? []
  const retagged = tags.length > 0 && tags.join('\u0000') !== beforeTags.join('\u0000')
  const rewritten = note.body.trim() !== '' && note.body.trim() !== before.body.trim()

  return (
    <span className="flex min-w-0 flex-1 flex-col gap-1">
      <span className="flex flex-wrap items-baseline gap-x-2">
        <b className="text-sm font-semibold text-text">{nameOf(note)}</b>
        <span className="text-warn">{t('assistant.noteChange')}</span>
      </span>
      {said.length > 0 && <span className="text-dim">{said.join(' · ')}</span>}
      {retitled && (
        <Change
          label={t('assistant.noteTitle')}
          was={before.title ?? ''}
          becomes={note.title ?? ''}
        />
      )}
      {retagged && (
        <Change
          label={t('assistant.noteTags')}
          was={beforeTags.join(', ')}
          becomes={tags.join(', ')}
        />
      )}
      {rewritten && (
        <Change label={t('assistant.noteText')} was={before.body} becomes={note.body} long />
      )}
      {!retitled && !retagged && !rewritten && (
        <span className="text-faint">{t('assistant.noteSame')}</span>
      )}
    </span>
  )
}

/** A word per layer, written out so a new one is a type error here. */
const LAYER_KEYS: Record<NonNullable<ProposedNote['layer']>, string> = {
  public: 'assistant.noteLayer.public',
  internal: 'assistant.noteLayer.internal',
  inWorks: 'assistant.noteLayer.inWorks',
}

/** One thing a change changes: what it was, and what it becomes. */
function Change({
  label,
  was,
  becomes,
  long = false,
}: {
  label: string
  was: string
  becomes: string
  long?: boolean
}) {
  const { t } = useTranslation()
  // A note's words, as they are and as they would be: text a person may
  // want to copy out, so it can be selected.
  const block = long ? 'selectable line-clamp-6 whitespace-pre-wrap' : 'selectable truncate'
  return (
    <span className="flex flex-col gap-0.5">
      <span className="caption">{label}</span>
      <span className={`text-faint line-through ${block}`} title={t('assistant.noteWas')}>
        {was === '' ? '—' : was}
      </span>
      <span className={`text-text ${block}`} title={t('assistant.noteBecomes')}>
        {becomes}
      </span>
    </span>
  )
}
