import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowUpRight, Link2, Trash2, X } from 'lucide-react'
import { deleteTerm, linkTerm, unlinkTerm, updateTerm } from '@/lib/api/register'
import type { RegisterEntry, TermPatch } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { isWording, splitForms } from '@/lib/register'
import { announceDeleted } from '@/lib/trash'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { ListRow } from '@/components/ui/list-row'
import { SkeletonList } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { Pane } from '@/components/frame'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import {
  BankPicker,
  KindPicker,
  StrictnessPicker,
  SungEditor,
  TopicField,
} from '@/features/register/TermFields'

interface Props {
  entry: RegisterEntry
  /** The term left: deleted. */
  onGone: () => void
}

/**
 * One word of the record, open (ADR 0052): what it is along the top - how
 * strictly it is spent, or not at all - then where it stands in the bank,
 * how it is sung, its forms, topic and note, and the works it is in.
 *
 * The works are the point. For wording they are read off the works' current
 * texts - how many times each says it - with the works a person named beside
 * them; for a meaning, which no text is searched for, they are only the named
 * ones, and naming them is done here.
 */
export function TermDetail({ entry, onGone }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const [word, setWord] = useState(entry.word)
  const [forms, setForms] = useState(entry.forms.join(', '))
  const [topic, setTopic] = useState(entry.topic ?? '')
  const [note, setNote] = useState(entry.note ?? '')
  const [naming, setNaming] = useState(false)

  const uses = useQuery(queries.termUses(entry.id))

  const patch = useAppMutation({
    mutationFn: (change: TermPatch) => updateTerm(entry.id, change),
    failure: 'register.saveFailed',
    refresh: refresh.term,
    // A refused rename puts the word back: the field shows what is kept.
    onError: () => setWord(entry.word),
  })
  const link = useAppMutation({
    mutationFn: (workId: string) => linkTerm(entry.id, workId),
    refresh: refresh.term,
  })
  const unlink = useAppMutation({
    mutationFn: (workId: string) => unlinkTerm(entry.id, workId),
    refresh: refresh.term,
  })
  const remove = useAppMutation({
    mutationFn: () => deleteTerm(entry.id),
    failure: 'register.saveFailed',
    onSuccess: (deletionId) => {
      announceDeleted({
        client,
        deletionId,
        message: t('register.deleted', { word: entry.word }),
        refresh: refresh.term,
      })
      onGone()
    },
  })

  // A field is written when it is left, and only when it changed: leaving
  // one untouched writes nothing and records nothing.
  const saveText = (key: 'topic' | 'note', value: string, was: string | null) => {
    const next = value.trim()
    if (next === (was ?? '')) return
    patch.mutate({ [key]: next === '' ? null : next })
  }
  const wording = isWording(entry.kind)

  return (
    <Pane
      label={entry.word}
      bodyClassName="flex flex-col gap-4 px-4 py-3"
      head={
        <>
          <Input
            value={word}
            onChange={(event) => setWord(event.target.value)}
            onBlur={() => {
              const next = word.trim()
              if (next !== '' && next !== entry.word) patch.mutate({ word: next })
              else setWord(entry.word)
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter') event.currentTarget.blur()
            }}
            aria-label={t('register.word')}
            className="min-w-40 flex-1 border-transparent bg-transparent px-1.5 text-sm font-semibold hover:border-line focus:border-line"
          />
          <StrictnessPicker
            optional
            value={entry.strictness}
            onChange={(strictness) => {
              if (strictness !== entry.strictness) patch.mutate({ strictness })
            }}
          />
          <KindPicker
            value={entry.kind}
            onChange={(kind) => {
              if (kind !== entry.kind) patch.mutate({ kind })
            }}
          />
          <Button
            size="icon-sm"
            variant="danger"
            title={t('register.delete')}
            aria-label={t('register.delete')}
            disabled={remove.isPending}
            onClick={() => remove.mutate()}
          >
            <Trash2 aria-hidden />
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-1.5">
        <span className="caption">{t('words.bank')}</span>
        <BankPicker
          value={entry.bank}
          onChange={(bank) => {
            if (bank !== entry.bank) patch.mutate({ bank })
          }}
        />
      </div>
      {wording && (
        <div className="flex flex-col gap-1.5">
          <span className="caption">{t('words.sung')}</span>
          <p className="text-xs text-dim">{t('words.sungHelp')}</p>
          <SungEditor
            value={entry.sung}
            word={entry.word}
            onChange={(sung) => patch.mutate({ sung })}
          />
        </div>
      )}
      {wording && (
        <Field label={t('register.forms')} help={t('register.formsHelp')}>
          <Input
            value={forms}
            onChange={(event) => setForms(event.target.value)}
            onBlur={() => {
              const next = splitForms(forms)
              if (next.join('\n') !== entry.forms.join('\n')) patch.mutate({ forms: next })
            }}
          />
        </Field>
      )}
      <div className="flex flex-col gap-1.5">
        <span className="caption">{t('register.topic')}</span>
        <TopicField
          value={topic}
          onChange={setTopic}
          onCommit={() => saveText('topic', topic, entry.topic)}
        />
      </div>
      <Field label={t('register.note')}>
        <Textarea
          value={note}
          onChange={(event) => setNote(event.target.value)}
          onBlur={() => saveText('note', note, entry.note)}
          placeholder={t('register.notePlaceholder')}
          rows={3}
        />
      </Field>

      <section className="flex flex-col gap-2">
        <div className="flex items-center gap-2">
          <h3 className="caption">{t('register.where')}</h3>
          <span className="text-xs text-faint tabular-nums">
            {t('register.uses', { count: entry.uses })}
          </span>
          <Button size="xs" variant="ghost" className="ml-auto" onClick={() => setNaming(true)}>
            <Link2 aria-hidden />
            {t('register.name')}
          </Button>
        </div>
        <p className="text-xs text-dim">
          {wording ? t('register.whereWording') : t('register.whereMeaning')}
        </p>
        {uses.isPending ? (
          <SkeletonList rows={3} />
        ) : (uses.data ?? []).length === 0 ? (
          <EmptyState plain title={t('register.whereNone')} className="p-2" />
        ) : (
          <ul className="rounded-md border border-line">
            {(uses.data ?? []).map((one) => (
              <ListRow
                key={one.work_id}
                render={<li />}
                end={
                  <span className="flex items-center gap-2">
                    {one.found > 0 && (
                      <span className="tabular-nums">
                        {t('register.found', { times: one.found })}
                      </span>
                    )}
                    {one.named && (
                      <>
                        <span className="text-faint">{t('register.named')}</span>
                        <Button
                          size="icon-xs"
                          variant="icon"
                          title={t('register.letGo')}
                          aria-label={t('register.letGo')}
                          disabled={unlink.isPending}
                          onClick={() => unlink.mutate(one.work_id)}
                        >
                          <X aria-hidden />
                        </Button>
                      </>
                    )}
                  </span>
                }
              >
                <Button
                  variant="link"
                  className="min-w-0"
                  onClick={() => void navigate(`/works/${one.work_id}`)}
                >
                  <span className="truncate">{one.title}</span>
                  <ArrowUpRight aria-hidden />
                </Button>
              </ListRow>
            ))}
          </ul>
        )}
      </section>

      <PickWorkDialog
        open={naming}
        onOpenChange={setNaming}
        title={t('register.nameTitle', { word: entry.word })}
        onPick={(picked) => link.mutate(picked.work_id)}
      />
    </Pane>
  )
}
