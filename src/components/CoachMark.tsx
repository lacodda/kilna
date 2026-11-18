import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Lightbulb } from 'lucide-react'
import { Button } from '@/components/ui/button'

/** The screens a first visit is told about, by their key in `app/screens`. */
const COACHED = [
  'dashboard',
  'catalogue',
  'works',
  'calendar',
  'collections',
  'notes',
  'canon',
  'register',
  'comments',
  'styles',
  'journal',
  'trash',
  'settings',
] as const

const SEEN = 'kilna.coach.seen'
/** Every hint off, until asked for again: set by "No more hints", and by the
 *  tests, which look at screens rather than at their first visit. */
export const QUIET = 'kilna.coach.quiet'

function read(): string[] {
  try {
    const raw = localStorage.getItem(SEEN)
    const parsed: unknown = raw === null ? [] : JSON.parse(raw)
    return Array.isArray(parsed)
      ? parsed.filter((one): one is string => typeof one === 'string')
      : []
  } catch {
    return []
  }
}

function quiet(): boolean {
  try {
    return localStorage.getItem(QUIET) !== null
  } catch {
    return false
  }
}

function keep(key: string, value: string): void {
  try {
    localStorage.setItem(key, value)
  } catch {
    // Storage blocked: the hint comes back next time, which is the safe way
    // to be wrong.
  }
}

/** Bring every hint back, as on a first start: what Settings offers. */
export function resetCoachMarks(): void {
  try {
    localStorage.removeItem(SEEN)
    localStorage.removeItem(QUIET)
  } catch {
    // Nothing to bring back from storage that cannot be reached.
  }
}

/**
 * What a screen is for, said once - the first time it is opened on this
 * machine (v0.129, #112). Two sentences and a way out; "No more hints" turns
 * them all off, and Settings brings them back. Kept on the machine, not in the
 * space: it is about the person learning the window, not about their works.
 */
export function CoachMark({ screen }: { screen: string }) {
  const { t } = useTranslation()
  const [seen, setSeen] = useState(read)
  const [silenced, setSilenced] = useState(quiet)
  if (silenced || seen.includes(screen) || !(COACHED as readonly string[]).includes(screen)) {
    return null
  }
  const dismiss = () => {
    const next = [...seen, screen]
    keep(SEEN, JSON.stringify(next))
    setSeen(next)
  }
  return (
    <aside
      aria-label={t('coach.label')}
      className="mb-2 flex shrink-0 items-start gap-2.5 rounded-md border border-line bg-soft px-3 py-2 text-xs"
    >
      <Lightbulb aria-hidden className="mt-0.5 size-3.5 shrink-0 text-accent" />
      <p className="min-w-0 flex-1 text-dim">{t(`coach.screens.${screen}`)}</p>
      <Button size="sm" variant="ghost" onClick={dismiss}>
        {t('coach.gotIt')}
      </Button>
      <Button
        size="sm"
        variant="icon"
        onClick={() => {
          keep(QUIET, '1')
          setSilenced(true)
        }}
      >
        {t('coach.off')}
      </Button>
    </aside>
  )
}
