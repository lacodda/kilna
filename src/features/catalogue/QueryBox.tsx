import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Search } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'

interface Props {
  value: string
  onChange: (line: string) => void
  /** A value the profile does not have, from the line as typed. */
  unknown: { field: string; value: string } | undefined
  /** How many works the table shows, while something narrows it. */
  shown: number | undefined
  total: number
}

/**
 * The query box: words to search, and `field:value` to narrow.
 *
 * It leads the toolbar and takes the room the row has spare, as the mockup's
 * does, with the number it found at its end - the answer beside the question.
 * The number is there only while something narrows the table; with nothing
 * typed it would repeat the count at the end of the row below.
 *
 * What the box can do is said in its tooltip and to a screen reader rather
 * than in a line under it: the toolbar is two rows, and a third that says the
 * same thing on every visit was the line the mockup did without. A value the
 * profile does not have is the one thing that must be seen - the box turns
 * red, and the row below says which value, in place of its count.
 */
export function QueryBox({ value, onChange, unknown, shown, total }: Props) {
  const { t } = useTranslation()
  const help = useId()

  return (
    <div className="relative min-w-52 flex-1">
      <Search
        aria-hidden
        className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-faint"
      />
      <Input
        className="pr-14 pl-8 font-mono"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={t('catalogue.queryPlaceholder')}
        title={t('catalogue.queryHint')}
        aria-label={t('works.search')}
        aria-describedby={help}
        aria-invalid={unknown !== undefined || undefined}
      />
      <span id={help} hidden>
        {unknown === undefined
          ? t('catalogue.queryHint')
          : t('catalogue.queryUnknown', { field: unknown.field, value: unknown.value })}
      </span>
      {shown !== undefined && (
        <Badge
          variant="accent"
          className="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 px-2 font-mono"
          title={t('catalogue.showing', { shown, total })}
        >
          {shown}
        </Badge>
      )}
    </div>
  )
}
