import { useEffect, useId, useRef, useState } from 'react'
import { cn } from '@/lib/utils'

export interface Option {
  value: string
  label: string
}

interface Props {
  value: string
  onChange: (value: string) => void
  options: Option[]
  /// Entry shown when nothing is chosen; picking it yields an empty string.
  placeholder?: string
  className?: string
  'aria-label'?: string
}

// A dropdown built from buttons rather than a native <select>: the platform
// control cannot be styled to match the rest of the app and reads as foreign.
// Keyboard behaviour is implemented here because that is what is given up.
export function Select({
  value,
  onChange,
  options,
  placeholder,
  className,
  'aria-label': ariaLabel,
}: Props) {
  const [open, setOpen] = useState(false)
  const [active, setActive] = useState(0)
  const container = useRef<HTMLDivElement>(null)
  const listId = useId()

  const entries = placeholder === undefined ? options : [{ value: '', label: placeholder }, ...options]
  const selected = entries.find((option) => option.value === value)

  useEffect(() => {
    if (!open) return

    const onPointerDown = (event: PointerEvent) => {
      if (!container.current?.contains(event.target as Node)) setOpen(false)
    }
    document.addEventListener('pointerdown', onPointerDown)
    return () => document.removeEventListener('pointerdown', onPointerDown)
  }, [open])

  const choose = (option: Option) => {
    onChange(option.value)
    setOpen(false)
  }

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'Escape') {
      setOpen(false)
      return
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      if (!open) {
        setOpen(true)
        return
      }
      const step = event.key === 'ArrowDown' ? 1 : -1
      setActive((current) => (current + step + entries.length) % entries.length)
      return
    }
    if ((event.key === 'Enter' || event.key === ' ') && open) {
      event.preventDefault()
      const option = entries[active]
      if (option !== undefined) choose(option)
    }
  }

  return (
    <div ref={container} className={cn('relative', className)}>
      <button
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        aria-label={ariaLabel}
        onClick={() => {
          setOpen((current) => !current)
          setActive(Math.max(0, entries.findIndex((option) => option.value === value)))
        }}
        onKeyDown={onKeyDown}
        className={cn(
          'flex h-9 w-full items-center justify-between gap-2 rounded-md border px-2.5 text-sm',
          'border-neutral-300 hover:bg-neutral-50 dark:border-neutral-700 dark:hover:bg-neutral-800',
          'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-kiln-500',
        )}
      >
        <span className={cn('truncate', selected === undefined && 'text-neutral-400')}>
          {selected?.label ?? placeholder ?? ''}
        </span>
        <span aria-hidden className="text-neutral-400">
          ▾
        </span>
      </button>

      {open && (
        <ul
          id={listId}
          role="listbox"
          className={cn(
            'absolute z-20 mt-1 max-h-64 w-full overflow-auto rounded-md border py-1 shadow-lg',
            'border-neutral-200 bg-white dark:border-neutral-700 dark:bg-neutral-900',
          )}
        >
          {entries.map((option, index) => (
            <li key={option.value || '__empty'}>
              <button
                type="button"
                role="option"
                aria-selected={option.value === value}
                onMouseEnter={() => setActive(index)}
                onClick={() => choose(option)}
                className={cn(
                  'w-full px-2.5 py-1.5 text-left text-sm',
                  index === active && 'bg-neutral-100 dark:bg-neutral-800',
                  option.value === value && 'font-medium text-kiln-700 dark:text-kiln-200',
                  option.value === '' && 'text-neutral-500',
                )}
              >
                {option.label}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
