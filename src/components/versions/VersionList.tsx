import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { CopyPlus, Star, X } from 'lucide-react'
import type { VersionSummary } from '@/lib/api'
import { neighbour } from '@/lib/history'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'

interface Props {
  versions: VersionSummary[]
  loading: boolean
  openId: string | null
  /** Which version the open one is being compared against, if any. */
  comparedId: string | null
  onOpen: (id: string) => void
  onCompare: (id: string) => void
  onMakeCurrent: (id: string) => void
  onDelete: (id: string) => void
  /** Start a new draft from this version's body. */
  onDeriveFrom: (id: string) => void
}

/**
 * Every version of one role, newest first.
 *
 * A row is a button rather than a link: which draft is open is a view state of
 * this tab, not an address of its own. Only the tab itself is addressable.
 *
 * The arrows walk the history, so reading through six revisions is six presses
 * rather than six aimed clicks. The list is a listbox for that reason: the open
 * row is the selected option, and only it is in the tab order.
 */
export function VersionList({
  versions,
  loading,
  openId,
  comparedId,
  onOpen,
  onCompare,
  onMakeCurrent,
  onDelete,
  onDeriveFrom,
}: Props) {
  const { t } = useTranslation()
  const list = useRef<HTMLUListElement>(null)

  // Arrowing moves the open version, and the focus has to follow it or the next
  // press comes from where the finger was rather than from what is on screen.
  //
  // Only when a row itself held the focus. Stepping from the editor must not
  // yank the cursor out of the text, and arrowing while the ± or delete button
  // of a row is focused must not drag the focus off that button either — the
  // keystroke bubbles up from there, but it was not aimed at the row.
  useEffect(() => {
    const from = document.activeElement
    if (openId === null || !(from instanceof HTMLElement) || from.dataset.version === undefined) {
      return
    }
    const row = list.current?.querySelector<HTMLElement>(`[data-version="${openId}"]`)
    row?.focus()
  }, [openId])

  const step = (direction: -1 | 1) => {
    const next = neighbour(versions, openId, direction)
    if (next !== null) onOpen(next.id)
  }

  const onKeyDown = (event: React.KeyboardEvent) => {
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        step(1)
        break
      case 'ArrowUp':
        event.preventDefault()
        step(-1)
        break
      case 'Home':
        event.preventDefault()
        if (versions[0] !== undefined) onOpen(versions[0].id)
        break
      case 'End':
        event.preventDefault()
        if (versions.at(-1) !== undefined) onOpen(versions.at(-1)!.id)
        break
      default:
        break
    }
  }

  if (loading) return <Skeleton className="h-32 w-full" />

  if (versions.length === 0) {
    return <p className="py-4 text-sm text-dim">{t('versions.none')}</p>
  }

  return (
    <ul
      ref={list}
      role="listbox"
      aria-label={t('versions.title')}
      onKeyDown={onKeyDown}
      className="flex flex-col gap-1"
    >
      {versions.map((version) => {
        const isOpen = version.id === openId
        const isCompared = version.id === comparedId

        return (
          <li key={version.id} className="group flex items-center gap-1">
            <button
              type="button"
              role="option"
              aria-selected={isOpen}
              data-version={version.id}
              // Roving focus: one stop for the whole history, and the arrows do
              // the rest. Tabbing past twenty revisions is not navigation.
              tabIndex={isOpen ? 0 : -1}
              onClick={() => onOpen(version.id)}
              className={cn(
                'min-w-0 flex-1 rounded-[9px] px-2 py-1.5 text-left text-sm transition-colors',
                isOpen && 'bg-accent-soft text-accent-2',
                !isOpen && isCompared && 'bg-soft',
                !isOpen && !isCompared && 'hover:bg-soft',
              )}
            >
              <span className="flex items-center gap-1.5">
                <span className="font-mono text-[11.5px] text-faint">v{version.revision}</span>
                <span className="min-w-0 flex-1 truncate font-medium">
                  {version.label ?? t('versions.revision', { number: version.revision })}
                </span>
                {version.is_current && (
                  <span className="rounded bg-accent-soft px-1 text-[10px] font-semibold uppercase tracking-wide text-accent-2">
                    {t('versions.current')}
                  </span>
                )}
              </span>
              <span className="block text-xs text-dim">
                {t('versions.length', { count: version.length })} · {version.created_at.slice(0, 10)}
              </span>
            </button>

            {/* Only shown for versions other than the open one: comparing a
                draft with itself is not a thing anyone means to do. */}
            {!isOpen && (
              <Button
                variant={isCompared ? 'soft' : 'icon'}
                size="icon-sm"
                onClick={() => onCompare(version.id)}
                title={isCompared ? t('versions.stopComparing') : t('versions.compare')}
                aria-label={isCompared ? t('versions.stopComparing') : t('versions.compare')}
              >
                <span aria-hidden className="font-mono text-[11px]">
                  ±
                </span>
              </Button>
            )}

            {/* Versions never change once saved — revising means starting the
                next revision from this one, so the act needs a button of its
                own. See `Решения` on why there is no edit. */}
            <Button
              variant="icon"
              size="icon-sm"
              onClick={() => onDeriveFrom(version.id)}
              title={t('versions.deriveFrom')}
              aria-label={t('versions.deriveFrom')}
            >
              <CopyPlus aria-hidden className="size-4" />
            </Button>

            {!version.is_current && (
              <Button
                variant="icon"
                size="icon-sm"
                onClick={() => onMakeCurrent(version.id)}
                title={t('versions.makeCurrent')}
                aria-label={t('versions.makeCurrent')}
              >
                <Star aria-hidden className="size-4" />
              </Button>
            )}

            <Button
              variant="danger"
              size="icon-sm"
              onClick={() => onDelete(version.id)}
              title={t('versions.delete')}
              aria-label={t('versions.delete')}
            >
              <X aria-hidden className="size-3.5" />
            </Button>
          </li>
        )
      })}
    </ul>
  )
}
