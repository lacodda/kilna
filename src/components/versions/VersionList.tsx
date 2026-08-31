import { useTranslation } from 'react-i18next'
import { Star, X } from 'lucide-react'
import type { VersionSummary } from '@/lib/api'
import { Button } from '@/components/ui/Button'
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
}

/**
 * Every version of one role, newest first.
 *
 * A row is a button rather than a link: which draft is open is a view state of
 * this tab, not an address of its own. Only the tab itself is addressable.
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
}: Props) {
  const { t } = useTranslation()

  if (loading) return <Skeleton className="h-32 w-full" />

  if (versions.length === 0) {
    return <p className="py-4 text-sm text-dim">{t('versions.none')}</p>
  }

  return (
    <ul className="flex flex-col gap-1">
      {versions.map((version) => {
        const isOpen = version.id === openId
        const isCompared = version.id === comparedId

        return (
          <li key={version.id} className="group flex items-center gap-1">
            <button
              type="button"
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
