import type { HTMLAttributes } from 'react'
import { cn } from '@/lib/utils'

// The raised surface everything sits on; the mockup's `.panel`.
export function Panel({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn('rounded-2xl border border-line bg-raise', className)} {...props} />
}

// Uppercase section label above a block of content; the mockup's `.sec`.
export function SectionLabel({ className, children, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        'flex items-center gap-2 text-[11px] font-medium uppercase tracking-[0.09em] text-faint',
        className,
      )}
      {...props}
    >
      {children}
    </div>
  )
}
