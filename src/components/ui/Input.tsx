import type { InputHTMLAttributes, TextareaHTMLAttributes } from 'react'
import { cn } from '@/lib/utils'

const FIELD =
  'w-full rounded-[9px] border border-line bg-transparent px-2.5 py-1.5 text-sm text-text ' +
  'placeholder:text-faint transition-colors hover:border-line-2 ' +
  'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-accent'

export function Input({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={cn(FIELD, 'h-9', className)} {...props} />
}

export function Textarea({ className, ...props }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea className={cn(FIELD, 'resize-y font-mono leading-relaxed', className)} {...props} />
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
