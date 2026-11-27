import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { mergeTerms, splitTerm } from '@/lib/api/register'
import type { RegisterEntry } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { partsOf } from '@/lib/register'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/AppDialog'
import { EmptyState } from '@/components/ui/empty-state'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'

/**
 * One word of the record made into several (v0.171): what only an agent
 * could propose. The parts are typed comma by comma, started from the word
 * itself where it reads as several; each is found or made with what the
 * word held, and the word goes to the trash, from where it comes back.
 */
export function SplitTermDialog({
  entry,
  onOpenChange,
  onDone,
}: {
  entry: RegisterEntry
  onOpenChange: (open: boolean) => void
  /** The word is gone: its parts stand in its place. */
  onDone: () => void
}) {
  const { t } = useTranslation()
  const [typed, setTyped] = useState(partsOf(entry.word).join(', '))
  const parts = partsOf(typed)
  const split = useAppMutation({
    mutationFn: () => splitTerm(entry.id, parts),
    failure: 'register.saveFailed',
    refresh: refresh.term,
    onSuccess: (made) => {
      say.ok(t('register.splitDone', { word: entry.word, count: made.length }))
      onDone()
    },
  })
  const ready = parts.length >= 2

  return (
    <Dialog
      open
      onOpenChange={onOpenChange}
      title={t('register.splitTitle', { word: entry.word })}
      description={t('register.splitBody')}
      footer={
        <Button
          type="submit"
          form="split-term"
          variant="primary"
          disabled={!ready || split.isPending}
        >
          {t('register.splitDo', { count: parts.length })}
        </Button>
      }
    >
      <form
        id="split-term"
        onSubmit={(event) => {
          event.preventDefault()
          if (ready) split.mutate()
        }}
      >
        <Field label={t('register.splitInto')} help={t('register.splitHelp')}>
          <Input autoFocus value={typed} onChange={(event) => setTyped(event.target.value)} />
        </Field>
      </form>
    </Dialog>
  )
}

/**
 * Two words of the record made one (v0.171): this word goes into the one
 * picked, as a form of it - with its own forms, its ways of singing and the
 * works named for it - and what the picked one says of itself stands.
 */
export function MergeTermDialog({
  entry,
  onOpenChange,
  onDone,
}: {
  entry: RegisterEntry
  onOpenChange: (open: boolean) => void
  /** The word is gone into another, whose id this is. */
  onDone: (intoId: string) => void
}) {
  const { t } = useTranslation()
  const [query, setQuery] = useState('')
  const [into, setInto] = useState<RegisterEntry | null>(null)
  const terms = useQuery(queries.terms())
  const needle = query.trim().toLowerCase()
  const others = (terms.data ?? [])
    .filter((one) => one.id !== entry.id)
    .filter(
      (one) =>
        needle === '' ||
        one.word.toLowerCase().includes(needle) ||
        one.forms.some((form) => form.toLowerCase().includes(needle)),
    )
    .slice(0, 50)
  const merge = useAppMutation({
    mutationFn: (intoId: string) => mergeTerms(entry.id, intoId),
    failure: 'register.saveFailed',
    refresh: refresh.term,
    onSuccess: (kept) => {
      say.ok(t('register.mergeDone', { word: entry.word, into: kept.word }))
      onDone(kept.id)
    },
  })

  return (
    <Dialog
      open
      onOpenChange={onOpenChange}
      title={t('register.mergeTitle', { word: entry.word })}
      description={t('register.mergeBody')}
      footer={
        <Button
          variant="primary"
          disabled={into === null || merge.isPending}
          onClick={() => {
            if (into !== null) merge.mutate(into.id)
          }}
        >
          {into === null
            ? t('register.mergePick')
            : t('register.mergeDo', { word: entry.word, into: into.word })}
        </Button>
      }
    >
      <div className="flex flex-col gap-2">
        <Input
          autoFocus
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={t('register.mergeSearch')}
          aria-label={t('register.mergeSearch')}
        />
        {others.length === 0 ? (
          <EmptyState plain title={t('register.mergeNone')} className="p-2" />
        ) : (
          <ul className="max-h-64 overflow-y-auto rounded-md border border-line">
            {others.map((one) => (
              <li key={one.id}>
                <RowButton
                  selected={into?.id === one.id}
                  onClick={() => setInto(one)}
                  description={one.forms.length > 0 ? one.forms.join(', ') : undefined}
                >
                  {one.word}
                </RowButton>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Dialog>
  )
}
