import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Lock } from 'lucide-react'
import { queries } from '@/lib/query/queries'

/**
 * A space another machine holds open is open here for reading only (#102,
 * ADR 0077): said once above every screen, so a refused save is no surprise.
 * A banner and not a toast for the reason the waiting banner gives: it holds
 * for as long as the hold does.
 */
export function HeldBanner() {
  const { t } = useTranslation()
  const current = useQuery(queries.currentSpace())
  const holder = current.data?.held_by
  if (holder === undefined || holder === null) return null

  return (
    <div
      role="status"
      className="flex items-center gap-3 rounded-xl border border-warn bg-warn-soft px-3 py-2 text-sm text-text"
    >
      <Lock aria-hidden className="size-4 shrink-0 text-warn" />
      {t('spaces.held', { machine: holder.machine })}
    </div>
  )
}
