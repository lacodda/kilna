import { cn } from '@/lib/utils'

/**
 * The product's mark: the hexagon of the line with `ki` in it.
 *
 * Drawn inline rather than loaded: the content security policy allows images
 * from the bundle only, and a file this small is inlined by the bundler
 * anyway. The same paths are written out once more, statically, in
 * `index.html` for the splash that shows before this code has loaded.
 */
export function Mark({ className }: { className?: string }) {
  return (
    <svg className={cn('shrink-0', className)} viewBox="0 0 32 32" aria-hidden>
      <path d="M16 2 28 9v14L16 30 4 23V9z" fill="var(--accent)" />
      <text
        x="16"
        y="20.5"
        fontFamily="Consolas, monospace"
        fontSize="11"
        fontWeight="700"
        fill="var(--on-accent)"
        textAnchor="middle"
      >
        ki
      </text>
    </svg>
  )
}
