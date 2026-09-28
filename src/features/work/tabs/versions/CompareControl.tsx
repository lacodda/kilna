import { useTranslation } from 'react-i18next'
import { ChevronDown, Diff } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { cn } from '@/lib/utils'

/** A version the open one can be compared with. */
export interface Candidate {
  id: string
  /** Its name, as the list gives it. */
  label: string
  revision: number
  /** Whether it is the revision directly before the open one. */
  previous: boolean
}

interface Props {
  /** The version standing beside the open one, if any. */
  againstId: string | null
  candidates: Candidate[]
  onPick: (id: string | null) => void
}

/**
 * "Compare with v9" - the mockup's one button in the version's bar.
 *
 * The question asked most often of a history is "what did this revision
 * change", so one press puts the revision before it beside the text, and the
 * same press takes it away again. Any other version is in the list behind the
 * arrow, the predecessor first; the ± on a row of the list does the same from
 * the other side. A version with nothing else in its role has nothing to be
 * compared with, and the button is not drawn.
 */
export function CompareControl({ againstId, candidates, onPick }: Props) {
  const { t } = useTranslation()
  if (candidates.length === 0) return null

  // What the press compares with: the one already beside the text, so the
  // button says what is on screen; otherwise the predecessor; and for the
  // oldest revision, which has none, the next one up.
  const target =
    candidates.find((candidate) => candidate.id === againstId) ??
    candidates.find((candidate) => candidate.previous) ??
    candidates[0]!
  const comparing = againstId !== null
  const several = candidates.length > 1

  const press = (
    <Button
      variant={comparing ? 'soft' : 'ghost'}
      size="sm"
      aria-pressed={comparing}
      className={cn(several && 'rounded-r-none')}
      title={
        comparing ? t('versions.stopComparing') : t('versions.compareWith', { name: target.label })
      }
      onClick={() => onPick(comparing ? null : target.id)}
    >
      <Diff aria-hidden />
      <span className="@max-2xl:sr-only">
        {t('versions.compareWith', { name: `v${target.revision}` })}
      </span>
    </Button>
  )

  if (!several) return press

  return (
    <span className="inline-flex shrink-0 items-stretch">
      {press}
      <Menu>
        <MenuTrigger
          render={
            <Button
              variant={comparing ? 'soft' : 'ghost'}
              size="sm"
              className="rounded-l-none border-l-0 px-1.5"
              title={t('versions.compareMenu')}
              aria-label={t('versions.compareMenu')}
            />
          }
        >
          <ChevronDown aria-hidden />
        </MenuTrigger>
        <MenuPopup align="end">
          {[...candidates]
            .sort((a, b) => Number(b.previous) - Number(a.previous))
            .map((candidate) => (
              <MenuItem key={candidate.id} onClick={() => onPick(candidate.id)}>
                <span className="truncate">{candidate.label}</span>
                {candidate.previous && (
                  <span className="ml-auto pl-3 text-xs text-faint">{t('versions.previous')}</span>
                )}
              </MenuItem>
            ))}
        </MenuPopup>
      </Menu>
    </span>
  )
}
