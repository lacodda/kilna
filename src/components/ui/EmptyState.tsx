import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

/**
 * The `ki` hex from the brand mark, drawn in the current text colour.
 *
 * Inlined rather than loaded from assets/: it is decorative, so it should not
 * cost a request, and it must take the theme's colour rather than the brand
 * magenta — a full-strength logo in an empty panel shouts.
 */
function Watermark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 100 100" aria-hidden className={cn('size-16', className)}>
      <polygon
        points="50,5 89,27.5 89,72.5 50,95 11,72.5 11,27.5"
        fill="none"
        stroke="currentColor"
        strokeWidth={6}
        strokeLinejoin="round"
      />
    </svg>
  )
}

interface Props {
  title: string
  body?: string
  /** The one thing worth doing here — an empty screen without a way out is a dead end. */
  action?: ReactNode
  className?: string
}

export function EmptyState({ title, body, action, className }: Props) {
  return (
    <div
      className={cn(
        'flex h-full flex-col items-center justify-center gap-3 rounded-2xl border border-dashed border-line-2 p-8 text-center',
        className,
      )}
    >
      <Watermark className="text-line-2" />
      <div>
        <p className="font-medium">{title}</p>
        {body !== undefined && <p className="mt-1 text-sm text-dim">{body}</p>}
      </div>
      {action}
    </div>
  )
}
