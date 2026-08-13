import type { HTMLAttributes } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '@/lib/utils'

export const badgeVariants = cva(
  'inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-[11.5px] whitespace-nowrap',
  {
    variants: {
      variant: {
        outline: 'border border-line text-dim',
        accent: 'bg-accent-soft font-semibold text-accent-2',
        good: 'bg-good-soft font-semibold text-good',
        warn: 'bg-warn-soft font-semibold text-warn',
        bad: 'bg-bad-soft font-semibold text-bad',
        info: 'bg-info-soft font-semibold text-info',
        soft: 'bg-soft font-semibold text-dim',
      },
    },
    defaultVariants: { variant: 'outline' },
  },
)

interface Props extends HTMLAttributes<HTMLSpanElement>, VariantProps<typeof badgeVariants> {}

export function Badge({ variant, className, ...props }: Props) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />
}
