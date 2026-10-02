import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Plus, X } from 'lucide-react'
import type { Bank, Strictness, Sung, TermKind } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { BANK, STRICTNESS, TERM_KINDS } from '@/lib/register'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Combobox,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
  ComboboxPopup,
} from '@/components/ui/combobox'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Select } from '@/components/AppSelect'

/*
 * The fields a term is written with, the same in the dialog that makes one
 * and on the register that edits it.
 */

/** What a term is: wording, or a meaning. */
export function KindPicker({
  value,
  onChange,
}: {
  value: TermKind
  onChange: (kind: TermKind) => void
}) {
  const { t } = useTranslation()
  return (
    <Select
      value={value}
      onChange={(next) => {
        if (next !== '') onChange(next as TermKind)
      }}
      options={TERM_KINDS.map((one) => ({ value: one, label: t(`register.kinds.${one}`) }))}
      aria-label={t('register.kind')}
      className="w-44"
    />
  )
}

/** The segment that stands for "none" where a facet may be absent. */
const NONE = 'none'

/**
 * How strictly it is off limits - which decides how it is marked - or, for a
 * word kept for the bank or for how it is sung, not spent at all (ADR 0052).
 * `optional` offers that last choice; the dialog that adds a term to the
 * register leaves it out.
 */
export function StrictnessPicker({
  value,
  onChange,
  optional = false,
}: {
  value: Strictness | null
  onChange: (strictness: Strictness | null) => void
  optional?: boolean
}) {
  const { t } = useTranslation()
  return (
    <SegmentedControl
      aria-label={t('register.strictness')}
      value={value ?? NONE}
      onValueChange={(next) => onChange(next === NONE ? null : (next as Strictness))}
    >
      {optional && (
        <Segment value={NONE}>
          <span title={t('register.notSpentHint')}>{t('register.notSpent')}</span>
        </Segment>
      )}
      {STRICTNESS.map((one) => (
        <Segment key={one} value={one}>
          <span title={t(`register.strictnessHints.${one}`)}>
            {t(`register.strictnesses.${one}`)}
          </span>
        </Segment>
      ))}
    </SegmentedControl>
  )
}

/** Where a word stands in the bank of words, or that it is not kept there. */
export function BankPicker({
  value,
  onChange,
}: {
  value: Bank | null
  onChange: (bank: Bank | null) => void
}) {
  const { t } = useTranslation()
  return (
    <SegmentedControl
      aria-label={t('words.bank')}
      value={value ?? NONE}
      onValueChange={(next) => onChange(next === NONE ? null : (next as Bank))}
    >
      <Segment value={NONE}>{t('words.notBanked')}</Segment>
      {BANK.map((one) => (
        <Segment key={one} value={one}>
          {t(`words.states.${one}`)}
        </Segment>
      ))}
    </SegmentedControl>
  )
}

/**
 * How a word's forms are sung where that is not how they are written - one
 * line per form, the written form on the left, the sung one on the right:
 * "Марсель" → "МарсЭль" (ADR 0053). A line saves when it is left; a line
 * emptied on either side is taken away.
 */
export function SungEditor({
  value,
  word,
  onChange,
}: {
  value: Sung[]
  /** The word itself: what a new line starts written as. */
  word: string
  onChange: (sung: Sung[]) => void
}) {
  const { t } = useTranslation()
  const [rows, setRows] = useState<Sung[]>(value)
  const [stored, setStored] = useState(value)
  // Follow what is stored when it moves underneath - a save coming back, an
  // undo - the way the release form follows its release.
  if (stored !== value) {
    setStored(value)
    setRows(value)
  }
  const settle = (next: Sung[]) => {
    const kept = next.filter((one) => one.written.trim() !== '' && one.sung.trim() !== '')
    if (JSON.stringify(kept) !== JSON.stringify(value)) onChange(kept)
  }
  const set = (index: number, patch: Partial<Sung>) =>
    setRows(rows.map((one, at) => (at === index ? { ...one, ...patch } : one)))

  return (
    <div className="flex flex-col gap-1.5">
      {rows.map((one, index) => (
        <div key={index} className="flex items-center gap-1.5">
          <Input
            value={one.written}
            onChange={(event) => set(index, { written: event.target.value })}
            onBlur={() => settle(rows)}
            aria-label={t('words.written')}
            className="min-w-0 flex-1"
          />
          <span aria-hidden className="text-faint">
            →
          </span>
          <Input
            value={one.sung}
            onChange={(event) => set(index, { sung: event.target.value })}
            onBlur={() => settle(rows)}
            aria-label={t('words.sungAs')}
            className="min-w-0 flex-1 font-mono"
          />
          <Button
            size="icon-sm"
            variant="icon"
            title={t('words.removeSung')}
            aria-label={t('words.removeSung')}
            onClick={() => {
              const next = rows.filter((_, at) => at !== index)
              setRows(next)
              settle(next)
            }}
          >
            <X aria-hidden />
          </Button>
        </div>
      ))}
      <Button
        size="xs"
        variant="ghost"
        className="self-start"
        onClick={() => setRows([...rows, { written: rows.length === 0 ? word : '', sung: '' }])}
      >
        <Plus aria-hidden />
        {t('words.addSung')}
      </Button>
    </div>
  )
}

/**
 * The topic a term is grouped under: any words, with the topics already in
 * use offered as they are typed - so the second term about the kitchen is
 * filed under the same words as the first.
 */
export function TopicField({
  value,
  onChange,
  onCommit,
}: {
  value: string
  onChange: (topic: string) => void
  /** The field was left: the moment a form that saves by field saves it. */
  onCommit?: () => void
}) {
  const { t } = useTranslation()
  const topics = useQuery(queries.termTopics())
  const [open, setOpen] = useState(false)
  const typed = value.trim().toLowerCase()
  const suggestions = (topics.data ?? [])
    .map(([name]) => name)
    .filter((name) => name.toLowerCase() !== typed && name.toLowerCase().includes(typed))
    .slice(0, 8)

  return (
    <Combobox
      items={suggestions}
      filter={null}
      value={value}
      onValueChange={(next) => onChange(typeof next === 'string' ? next : '')}
      open={open && suggestions.length > 0}
      onOpenChange={setOpen}
    >
      <ComboboxInput aria-label={t('register.topic')} onBlur={onCommit} />
      <ComboboxPopup className="p-1">
        <ComboboxList>
          {(topic: string) => (
            <ComboboxItem key={topic} value={topic} className="rounded-md px-2 py-1 text-sm">
              {topic}
            </ComboboxItem>
          )}
        </ComboboxList>
      </ComboboxPopup>
    </Combobox>
  )
}
