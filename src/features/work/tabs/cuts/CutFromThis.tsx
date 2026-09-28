import { useTranslation } from 'react-i18next'
import { useQueries } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { Scissors } from 'lucide-react'
import type { Cut, Derived, Work } from '@/lib/api/types'
import { scaleOf, totalLength } from '@/lib/cuts'
import { queries } from '@/lib/query/queries'
import { formatSeconds } from '@/lib/timecode'
import { Button } from '@/components/ui/button'
import { Panel, SectionLabel } from '@/components/ui/panel'
import { CutTrack } from '@/features/work/tabs/cuts/CutTrack'

interface Props {
  /** The donor whose card this is. */
  work: Work
  /** Every stretch taken out of it, by any work. */
  cuts: Cut[]
  /** The works made from it, which name most of the ones cut from it. */
  derived: Derived[]
}

/** The stretches one work took out of the donor, in the order they fall. */
interface Taker {
  work_id: string
  cuts: Cut[]
}

/**
 * What has been cut out of this work, seen from the work itself.
 *
 * A video that three shorts were cut from used to know nothing about them:
 * the stretches lived on the shorts' cards, and the question worth asking
 * before cutting a fourth - which parts are already used - meant opening each
 * short in turn. This is the same track seen from the other side: the donor's
 * own length, every stretch anyone took out of it, and who took it.
 *
 * Read only. A stretch belongs to the short it was cut into and is edited
 * there; the name of each short leads to it.
 */
export function CutFromThis({ work, cuts, derived }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()

  const takers: Taker[] = []
  for (const cut of cuts) {
    const found = takers.find((taker) => taker.work_id === cut.work_id)
    if (found) found.cuts.push(cut)
    else takers.push({ work_id: cut.work_id, cuts: [cut] })
  }

  // Names from the works made from this one; a stretch whose link was taken
  // away since still names its short, so the rest are asked for one by one.
  const named = new Map(derived.map((made) => [made.work_id, made.title]))
  const unnamed = takers.map((taker) => taker.work_id).filter((id) => !named.has(id))
  const asked = useQueries({ queries: unnamed.map((id) => queries.work(id)) })
  const titleOf = (id: string): string => {
    const known = named.get(id)
    if (known !== undefined) return known
    const read = asked[unnamed.indexOf(id)]
    if (read === undefined || read.isPending) return '…'
    return read.data?.title ?? t('cuts.gone')
  }

  const scale = scaleOf(cuts[0]?.source_duration ?? null)

  return (
    <Panel className="flex flex-col gap-2.5 px-3 py-2.5">
      <header className="flex items-center gap-2">
        <SectionLabel>{t('cuts.fromThis')}</SectionLabel>
        <span className="ml-auto font-mono text-xs text-faint">
          {t('cuts.runs', { length: formatSeconds(totalLength(cuts)), count: cuts.length })}
        </span>
      </header>

      {scale === null ? (
        <p className="text-sm text-warn">{t('cuts.noLength', { title: work.title })}</p>
      ) : (
        <CutTrack
          cuts={cuts}
          duration={scale}
          label={t('cuts.track', { title: work.title })}
          nameOf={(cut) => titleOf(cut.work_id)}
        />
      )}

      <ul className="flex flex-col">
        {takers.map((taker) => (
          <li key={taker.work_id} className="flex min-w-0 items-center gap-2 py-1 text-sm">
            <Scissors aria-hidden className="size-3.5 shrink-0 text-faint" />
            <Button
              variant="link"
              className="min-w-0 truncate"
              onClick={() => void navigate(`/works/${taker.work_id}/cuts`)}
            >
              {titleOf(taker.work_id)}
            </Button>
            <span className="min-w-0 flex-1 truncate font-mono text-xs text-faint">
              {taker.cuts
                .map((cut) => `${formatSeconds(cut.starts_at)} – ${formatSeconds(cut.ends_at)}`)
                .join(', ')}
            </span>
            <span className="shrink-0 font-mono text-xs text-faint">
              {formatSeconds(totalLength(taker.cuts))}
            </span>
          </li>
        ))}
      </ul>
    </Panel>
  )
}
