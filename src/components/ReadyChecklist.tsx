import { useTranslation } from 'react-i18next'
import { Check } from 'lucide-react'
import type { Need, ScheduledRelease } from '@/lib/api/types'
import { answeredOn, needKey, sayNeed, type AnsweredOn } from '@/lib/readiness'
import { useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { NeedGlyph } from '@/components/NeedGlyph'

interface Props {
  release: Pick<ScheduledRelease, 'readiness' | 'work_kind' | 'kind'>
  /** Takes a person to where a gap is closed: the tab of the work it is
   *  answered on, or - where the release is written on the same screen - the
   *  box itself. Left out, the gaps are words, not buttons. */
  onOpen?: (need: Need, tab: AnsweredOn) => void
  className?: string
}

/**
 * Everything a release's kind asks for before it goes out, each with a tick
 * or the glyph of what is missing (v0.97): the checklist the profile writes
 * for the place - the roles, the score, the cover, the fields - in its order.
 *
 * A gap is a button to where it is closed: the versions for a role, the
 * Cover tab for a cover, the overview, where the release is written, for a
 * field. What is done is said and left alone - ticked, not offered.
 */
export function ReadyChecklist({ release, onOpen, className }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const checks = release.readiness.checks
  if (checks.length === 0) return null

  return (
    <ul
      aria-label={t('readiness.checklist')}
      className={cn('flex flex-wrap items-center gap-x-3 gap-y-1 text-xs', className)}
    >
      {checks.map(({ need, done }) => {
        const name = sayNeed(need, profile.config, release, t, 'name')
        const mark = done ? (
          <Check aria-hidden className="size-3 shrink-0 text-good" />
        ) : (
          <NeedGlyph need={need} className="size-3 shrink-0 text-warn" />
        )
        return (
          <li key={needKey(need)} className={cn('flex items-center gap-1', done && 'text-dim')}>
            {done || onOpen === undefined ? (
              <>
                {mark}
                <span>{name}</span>
                <span className="sr-only">
                  {done ? t('readiness.done') : t('readiness.missing')}
                </span>
              </>
            ) : (
              // eslint-disable-next-line dowel/no-raw-button -- a word in a line of words: a Button's height and padding would break the line it reads in
              <button
                type="button"
                className="flex items-center gap-1 rounded-sm underline decoration-dotted underline-offset-2 hover:text-text focus-visible:outline-2 focus-visible:outline-accent"
                onClick={() => onOpen(need, answeredOn(need, profile.config, release.work_kind))}
              >
                {mark}
                <span>{name}</span>
                <span className="sr-only">{t('readiness.missing')}</span>
              </button>
            )}
          </li>
        )
      })}
    </ul>
  )
}
