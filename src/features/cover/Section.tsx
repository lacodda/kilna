import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronDown, ChevronRight } from 'lucide-react'
import { useFolded } from '@/lib/folded'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Input } from '@/components/ui/input'
import { useFieldDraft } from '@/components/ui/field-draft'

/**
 * A block of a constructor's column, the mockup's `csec`: a title, a hint
 * and the block's own buttons across the top, a line under it.
 *
 * With an `id` it folds (v0.135, FR-07 7.3): the title is a switch, and a
 * folded section shows only its `summary` - what is chosen in it, in one
 * line - so a long column reads at a glance. Which sections are folded is
 * remembered for the whole application (`useFolded`).
 */
export function Section({
  id,
  title,
  hint,
  summary,
  actions,
  children,
}: {
  id?: string
  title: string
  hint?: string
  /** What is chosen, in one line: shown while the section is folded. */
  summary?: string
  actions?: ReactNode
  children: ReactNode
}) {
  const { t } = useTranslation()
  const [folded, setFolded] = useFolded(id ?? '')
  const foldable = id !== undefined
  const shut = foldable && folded
  return (
    <section
      aria-label={title}
      className="flex flex-col gap-2.5 border-b border-line py-3 last:border-b-0"
    >
      <header className="flex flex-wrap items-center gap-x-2 gap-y-0.5">
        {foldable ? (
          <Button
            variant="icon"
            size="xs"
            aria-expanded={!shut}
            title={shut ? t('cover.unfold') : t('cover.fold')}
            onClick={() => setFolded(!shut)}
            className="-ml-1 gap-1 px-1 text-sm font-semibold text-text"
          >
            {shut ? <ChevronRight aria-hidden /> : <ChevronDown aria-hidden />}
            {title}
          </Button>
        ) : (
          <b className="text-sm font-semibold">{title}</b>
        )}
        {shut ? (
          summary !== undefined &&
          summary !== '' && (
            <span className="min-w-0 flex-1 truncate text-right text-xs text-dim">{summary}</span>
          )
        ) : (
          <>
            {hint !== undefined && <span className="text-2xs text-faint">{hint}</span>}
            {actions !== undefined && (
              <span className="ml-auto flex items-center gap-1.5">{actions}</span>
            )}
          </>
        )}
      </header>
      {!shut && children}
    </section>
  )
}

/**
 * Text of a picture's concept, held while it is typed and saved when the
 * field is left - and only when it changed, so tabbing through writes
 * nothing. A save per keystroke would write half-words and redraw the box
 * under the cursor.
 */
export function DraftText({
  label,
  value,
  rows,
  placeholder,
  mono,
  single,
  disabled,
  onCommit,
}: {
  label: string
  value: string
  rows?: number
  placeholder?: string
  /** The generator's words, set in the monospace a prompt is read in. */
  mono?: boolean
  /** One line: an Input rather than a box. */
  single?: boolean
  disabled?: boolean
  onCommit: (text: string) => void
}) {
  const draft = useFieldDraft(value, (text) => onCommit(text), { multiline: !single })
  if (single) {
    return <Input aria-label={label} placeholder={placeholder} disabled={disabled} {...draft} />
  }
  return (
    <Textarea
      aria-label={label}
      placeholder={placeholder}
      rows={rows ?? 2}
      autoResize
      maxRows={12}
      disabled={disabled}
      className={mono === true ? 'font-mono text-xs leading-relaxed' : undefined}
      {...draft}
    />
  )
}
