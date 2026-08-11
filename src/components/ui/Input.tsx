import type { InputHTMLAttributes, TextareaHTMLAttributes } from 'react'
import { cn } from '@/lib/utils'

const FIELD =
  'w-full rounded-md border border-neutral-300 bg-transparent px-2.5 py-1.5 text-sm ' +
  'placeholder:text-neutral-400 focus-visible:outline-2 focus-visible:outline-offset-0 ' +
  'focus-visible:outline-kiln-500 dark:border-neutral-700'

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
      <span className="text-xs font-medium uppercase tracking-wide text-neutral-500">{label}</span>
      {children}
      {hint !== undefined && <span className="text-xs text-neutral-500">{hint}</span>}
    </label>
  )
}
