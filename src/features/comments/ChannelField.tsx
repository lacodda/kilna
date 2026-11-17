import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Combobox,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
  ComboboxPopup,
} from '@/components/ui/combobox'

interface Props {
  value: string
  onChange: (value: string) => void
  /** The channels already used, most waiting first. */
  known: string[]
  autoFocus?: boolean
}

/**
 * Where a comment was written: a word the person types, with the words
 * already used offered as they type.
 *
 * Free text on purpose (migration 0027): a channel is wherever the work went
 * out, and a list someone has to maintain before the first comment can be
 * kept is a list nobody maintains. The words offered are what stops the
 * second comment from being filed under a near-miss of the first one's
 * channel.
 *
 * A list under the field rather than buttons beside it (FR-19 19.2): up to
 * eight buttons stood under the box in every dialog that asks where, taking
 * three rows of a form whose question is one line. The list opens when the
 * field is entered, narrows as it is typed into, and a word that is not in
 * it is simply kept - there is nothing to add first.
 */
export function ChannelField({ value, onChange, known, autoFocus }: Props) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const typed = value.trim().toLowerCase()
  const offered = known.filter(
    (channel) => channel.toLowerCase() !== typed && channel.toLowerCase().includes(typed),
  )

  return (
    <Combobox
      items={offered}
      // Narrowed here, by what is typed: the field's value is any word, not
      // one of the list's.
      filter={null}
      // What is typed is the answer; a word picked from the list becomes
      // what is typed.
      inputValue={value}
      onInputValueChange={onChange}
      onValueChange={(next) => {
        if (typeof next === 'string') onChange(next)
      }}
      open={open && offered.length > 0}
      onOpenChange={setOpen}
    >
      <ComboboxInput
        autoFocus={autoFocus}
        placeholder={t('comments.channelPlaceholder')}
        aria-label={t('comments.channel')}
        onFocus={() => setOpen(true)}
      />
      <ComboboxPopup className="p-1">
        <ComboboxList>
          {(channel: string) => (
            <ComboboxItem key={channel} value={channel} className="rounded-md px-2 py-1 text-sm">
              {channel}
            </ComboboxItem>
          )}
        </ComboboxList>
      </ComboboxPopup>
    </Combobox>
  )
}
