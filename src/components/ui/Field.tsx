import type { ReactNode } from 'react'

/*
 * Field - a label, the control, and an optional hint under it.
 *
 * kilna's own, and deliberately still here: dowel gets a `Field` in v0.16,
 * with error text and a form adapter, and this is replaced by it then. Until
 * that exists, a wrapper this small is not worth a copy of the wrong shape.
 *
 * The caption is drawn in the line's vocabulary rather than in the pixel
 * values it used to carry, so it moves with the theme like everything else.
 */
interface FieldProps {
  label: string
  children: ReactNode
  hint?: string
}

export function Field({ label, children, hint }: FieldProps) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-2xs font-semibold uppercase tracking-caption text-faint">{label}</span>
      {children}
      {hint !== undefined && <span className="text-xs text-faint">{hint}</span>}
    </label>
  )
}
