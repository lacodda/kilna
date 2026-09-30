import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { createTerm } from '@/lib/api/register'
import type { Strictness, TermKind } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { guessKind, isWording, splitForms } from '@/lib/register'
import { say } from '@/lib/toast'
import { useDebounced } from '@/lib/useDebounced'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/AppDialog'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { KindPicker, StrictnessPicker, TopicField } from '@/features/register/TermFields'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** The words to start from: what was selected in a text. */
  word?: string
}

/**
 * A term entered into the register (ADR 0044).
 *
 * Opened from a selection in a text, with the selection as the word, or by
 * hand on the register. While the words are typed it says how many works
 * already say them - the count the register will keep, read off the texts, so
 * "is this spent?" is answered before the term exists. The kind is guessed
 * until it is picked: one word a noun, several a phrase.
 */
export function NewTermDialog({ open, onOpenChange, word = '' }: Props) {
  return open ? <Contents onOpenChange={onOpenChange} word={word} /> : null
}

function Contents({ onOpenChange, word: initial = '' }: Omit<Props, 'open'>) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [word, setWord] = useState(initial.trim())
  const [forms, setForms] = useState('')
  const [kind, setKind] = useState<TermKind | null>(null)
  const [strictness, setStrictness] = useState<Strictness>('limit')
  const [topic, setTopic] = useState('')
  const [note, setNote] = useState('')

  const shownKind = kind ?? guessKind(word)
  const wording = isWording(shownKind)
  // Each debounced as the string it is: an object made anew every render
  // would restart the wait for ever.
  const typedWord = useDebounced(word.trim(), 250)
  const typedForms = useDebounced(forms, 250)
  const preview = useQuery({
    ...queries.termPreview(typedWord, splitForms(typedForms)),
    enabled: typedWord !== '' && wording,
  })

  const create = useAppMutation({
    mutationFn: () =>
      createTerm({
        word: word.trim(),
        forms: wording ? splitForms(forms) : [],
        kind: shownKind,
        strictness,
        ...(topic.trim() === '' ? {} : { topic: topic.trim() }),
        ...(note.trim() === '' ? {} : { note: note.trim() }),
      }),
    failure: 'register.createFailed',
    refresh: refresh.term,
    onSuccess: (made) => {
      onOpenChange(false)
      say.withAction(t('register.created', { word: made.word }), t('register.open'), () => {
        void navigate(`/register/${made.id}`)
      })
    },
  })

  const ready = word.trim() !== ''
  const help = !wording
    ? t('register.meaningHelp')
    : typedWord === '' || preview.data === undefined
      ? t('register.wordHelp')
      : t('register.alreadyIn', { count: preview.data })

  return (
    <Dialog
      open
      onOpenChange={onOpenChange}
      title={t('register.newTitle')}
      description={t('register.newBody')}
      size="lg"
      footer={
        <Button
          type="submit"
          form="new-term"
          variant="primary"
          disabled={!ready || create.isPending}
        >
          <Plus aria-hidden />
          {t('register.add')}
        </Button>
      }
    >
      <form
        id="new-term"
        className="flex flex-col gap-3"
        onSubmit={(event) => {
          event.preventDefault()
          if (ready) create.mutate()
        }}
      >
        <Field label={t('register.word')} help={help}>
          <Input autoFocus value={word} onChange={(event) => setWord(event.target.value)} />
        </Field>
        {wording && (
          <Field label={t('register.forms')} help={t('register.formsHelp')}>
            <Input value={forms} onChange={(event) => setForms(event.target.value)} />
          </Field>
        )}
        <div className="flex flex-wrap items-center gap-3">
          <KindPicker value={shownKind} onChange={setKind} />
          <StrictnessPicker value={strictness} onChange={setStrictness} />
        </div>
        <div className="flex flex-col gap-1.5">
          <span className="caption">{t('register.topic')}</span>
          <TopicField value={topic} onChange={setTopic} />
        </div>
        <Field label={t('register.note')}>
          <Textarea
            value={note}
            onChange={(event) => setNote(event.target.value)}
            placeholder={t('register.notePlaceholder')}
            rows={2}
          />
        </Field>
      </form>
    </Dialog>
  )
}
