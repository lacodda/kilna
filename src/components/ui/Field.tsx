import { Children, cloneElement, isValidElement, useId, type ReactElement, type ReactNode } from 'react'

/*
 * Field - a caption, the control, and an optional hint under it.
 *
 * kilna's own until dowel's grows a group form: dowel's Field labels exactly
 * one control, and three of these hold a row of buttons instead.
 *
 * The caption is never a `<label>` wrapped around the content. A label
 * without `for` labels the first labelable element inside it, and a click on
 * any plain part of it - the caption, the hint, a gap between chips, a
 * thumbnail - is forwarded to that element. Around a single input that was
 * harmless; around the style dialog's rows of buttons it was not: a click on
 * "Type" silently set the first type, and a click on "References" pressed the
 * hidden remove cross of the first picture, which deletes it for good.
 * Measured in the live window before this was written.
 *
 * So there are two shapes, and the content decides between them. One control
 * gets a label tied to it by id, which still focuses it and names it. Anything
 * else - a row of chips, a grid of pictures, a block of text - is a named
 * group, and a group has nothing a click could be forwarded to.
 */
interface FieldProps {
  label: string
  children: ReactNode
  hint?: string
}

/** Host elements that are controls themselves rather than containers. */
const CONTROL_TAGS = new Set(['input', 'textarea', 'select'])

type ControlProps = { id?: string; 'aria-describedby'?: string }

export function Field({ label, children, hint }: FieldProps) {
  const captionId = useId()
  const fallbackId = useId()
  const hintId = useId()
  const control = singleControl(children)

  const caption = 'text-2xs font-semibold uppercase tracking-caption text-faint'
  const note = hint !== undefined && (
    <span id={hintId} className="text-xs text-faint">
      {hint}
    </span>
  )

  if (control === null) {
    return (
      <div role="group" aria-labelledby={captionId} className="flex flex-col gap-1">
        <span id={captionId} className={caption}>
          {label}
        </span>
        {children}
        {note}
      </div>
    )
  }

  const id = control.props.id ?? fallbackId
  return (
    <div className="flex flex-col gap-1">
      <label id={captionId} htmlFor={id} className={caption}>
        {label}
      </label>
      {cloneElement(control, {
        id,
        ...(hint !== undefined && { 'aria-describedby': hintId }),
      })}
      {note}
    </div>
  )
}

/**
 * The one control a field holds, or `null` when it holds anything else.
 *
 * A component counts as a control - Input, Select, DatePicker all put the id
 * on the element that takes focus. A host element counts only when it is an
 * input of its own; a `div` of buttons or a `pre` is content, not a control.
 */
function singleControl(children: ReactNode): ReactElement<ControlProps> | null {
  if (Children.count(children) !== 1 || !isValidElement<ControlProps>(children)) return null
  const { type } = children
  if (typeof type === 'string' && !CONTROL_TAGS.has(type)) return null
  return children
}
