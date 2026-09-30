import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Strictness, TermKind } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { STRICTNESS, TERM_KINDS } from '@/lib/register'
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

/** How strictly it is off limits - which decides how it is marked. */
export function StrictnessPicker({
  value,
  onChange,
}: {
  value: Strictness
  onChange: (strictness: Strictness) => void
}) {
  const { t } = useTranslation()
  return (
    <SegmentedControl
      aria-label={t('register.strictness')}
      value={value}
      onValueChange={(next) => onChange(next as Strictness)}
    >
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
