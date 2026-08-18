import type { InputHTMLAttributes, Ref, TextareaHTMLAttributes } from 'react'
import { cn } from '@/lib/utils'

const FIELD =
  'w-full rounded-[9px] border border-line bg-transparent px-2.5 py-1.5 text-sm text-text ' +
  'placeholder:text-faint transition-colors hover:border-line-2 ' +
  'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-accent'

// `ref` is a plain prop in React 19 — no forwardRef needed, it just has to be
// declared so callers can reach the element (focus, selection, scroll).
export function Input({
  className,
  ref,
  ...props
}: InputHTMLAttributes<HTMLInputElement> & { ref?: Ref<HTMLInputElement> }) {
  return <input ref={ref} className={cn(FIELD, 'h-9', className)} {...props} />
}

export function Textarea({
  className,
  ref,
  ...props
}: TextareaHTMLAttributes<HTMLTextAreaElement> & { ref?: Ref<HTMLTextAreaElement> }) {
  return (
    <textarea
      ref={ref}
      className={cn(FIELD, 'resize-y font-mono leading-relaxed', className)}
      {...props}
    />
  )
}

interface FieldProps {
  label: string
  children: React.ReactNode
  hint?: string
}

export function Field({ label, children, hint }: FieldProps) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-[10px] font-semibold uppercase tracking-[0.08em] text-faint">{label}</span>
      {children}
      {hint !== undefined && <span className="text-xs text-faint">{hint}</span>}
    </label>
  )
}
