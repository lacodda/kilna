import type { ReactNode } from 'react'
import { Textarea } from '@/components/ui/textarea'
import { Input } from '@/components/ui/input'
import { useFieldDraft } from '@/components/ui/field-draft'

/**
 * A block of a constructor's column, the mockup's `csec`: a title, a hint
 * and the block's own buttons across the top, a line under it.
 */
export function Section({
  title,
  hint,
  actions,
  children,
}: {
  title: string
  hint?: string
  actions?: ReactNode
  children: ReactNode
}) {
  return (
    <section
      aria-label={title}
      className="flex flex-col gap-2.5 border-b border-line py-3 last:border-b-0"
    >
      <header className="flex flex-wrap items-center gap-x-2 gap-y-0.5">
        <b className="text-sm font-semibold">{title}</b>
        {hint !== undefined && <span className="text-2xs text-faint">{hint}</span>}
        {actions !== undefined && (
          <span className="ml-auto flex items-center gap-1.5">{actions}</span>
        )}
      </header>
      {children}
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
