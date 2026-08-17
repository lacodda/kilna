import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { latestScore, listCollections, type Work } from '@/lib/api'
import { coverFor } from '@/lib/cover'
import { keys } from '@/lib/query'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Badge } from '@/components/ui/Badge'
import { TabBar } from '@/components/card/TabBar'

interface Props {
  work: Work
  /** Shown on the Releases tab; the card already knows the count. */
  releases: number
}

/**
 * The top of a work's card: what this is, at a glance, wherever you have
 * scrolled to.
 *
 * The cover is a gradient derived from the work's id (see `lib/cover`) until
 * real covers arrive; it is what makes one card distinguishable from another
 * before a single word is read.
 */
export function CardHeader({ work, releases }: Props) {
  const profile = useProfile()

  // The tier and total belong here rather than only on the Score tab: they are
  // the verdict, and the verdict is what someone opens a card to check.
  const score = useQuery({
    queryKey: keys.latestScore(work.id),
    queryFn: () => latestScore(work.id),
  })
  const collections = useQuery({
    queryKey: keys.collections,
    queryFn: listCollections,
    // Only fetched when the work is actually in one.
    enabled: work.collection_id !== null,
  })

  const collection = collections.data?.find((item) => item.id === work.collection_id)
  const latest = score.data ?? null

  // Two siblings rather than one header, because a sticky element can never
  // leave its own parent's box: wrapped together, the bar would unstick the
  // moment the (short) header scrolled past, which is exactly when it is needed.
  // As siblings of the tab body, both are bounded by the whole scrolling column.
  return (
    <>
      {/* The cover and the profile's own fields, read-only here — they are
          edited on the Overview tab. A header that can be typed into is a header
          that shifts under the cursor while it saves. Both scroll away: they are
          reference, not navigation. */}
      <div className="rounded-t-[18px] border border-b-0 border-line">
        <div className="h-[118px] rounded-t-[17px]" style={{ background: coverFor(work.id) }} />
        <MetaStrip work={work} />
      </div>

      {/* What stays: whose card this is, where it stands, and the way between
          tabs. A long version otherwise leaves you reading with no idea whose
          words they are. `top-0` is relative to the scrolling screen area. */}
      <header className="sticky top-0 z-20 rounded-b-[18px] border border-t-0 border-line bg-raise">
        <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1.5 px-[18px] pt-3 pb-2.5">
          <h2 className="text-[21px] font-[650] tracking-[-0.01em]">{work.title}</h2>

          <Badge>{labelOf(profile.config.work_kinds, work.kind)}</Badge>
          <Badge variant="accent">{labelOf(profile.config.statuses, work.status)}</Badge>

          {latest !== null && (
            <Badge variant="soft">
              {latest.tier !== null && `${labelOf(profile.config.tiers, latest.tier)} · `}
              <span className="font-mono tabular-nums">{Math.round(latest.total * 10) / 10}</span>
            </Badge>
          )}

          {collection !== undefined && <Badge>{collection.title}</Badge>}
        </div>

        <TabBar workId={work.id} releases={releases} />
      </header>
    </>
  )
}

function MetaStrip({ work }: { work: Work }) {
  const { i18n } = useTranslation()
  const profile = useProfile()

  const filled = profile.config.work_meta_fields.filter(
    (field) => work.meta[field.key] !== undefined && work.meta[field.key] !== '',
  )
  if (filled.length === 0) return null

  return (
    <div className="flex flex-wrap gap-[22px] bg-raise px-[18px] pt-3 pb-3">
      {filled.map((field) => {
        const value = work.meta[field.key]
        return (
          <span key={field.key}>
            <label className="block text-[10px] uppercase tracking-[0.08em] text-faint">
              {field.label}
            </label>
            <b className="font-mono text-[12.5px] font-medium">
              {field.type === 'boolean'
                ? i18n.t(value === true ? 'work.yes' : 'work.no')
                : String(value)}
            </b>
          </span>
        )
      })}
    </div>
  )
}
