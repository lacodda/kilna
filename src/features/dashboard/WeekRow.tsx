import type { Slot } from '@/lib/dashboard'
import { coverImageFor } from '@/lib/cover'
import { formatDay } from '@/lib/format'
import { stageAt } from '@/lib/stages'
import { useCovers } from '@/lib/useCovers'
import { allOf, labelOf, useProfile } from '@/lib/useProfile'
import { ReadyMarks } from '@/components/ReadyMarks'
import { WorkRepeatMark } from '@/components/RepeatMark'
import { StageDial } from '@/components/StageDial'
import { Badge } from '@/components/ui/badge'
import { RowButton } from '@/components/ui/list-row'

/** Anything holding a slot inside the week, ready or not. */
export function WeekRow({
  entry,
  onSelect,
}: {
  entry: Slot
  onSelect: (workId: string, tab?: string) => void
}) {
  const covers = useCovers()
  const profile = useProfile()
  const { release, daysLeft } = entry

  return (
    <RowButton
      onClick={() => onSelect(release.work_id)}
      className="rounded-none"
      start={
        <span
          aria-hidden
          className="size-5.5 shrink-0 rounded-sm"
          style={{ background: coverImageFor(release.work_id, covers.get(release.work_id)) }}
        />
      }
      end={
        <>
          {/* What it is and what goes out — a video's YouTube release and a
              song's clip are told apart by the first word, once the profile
              has more than one kind of work. */}
          <Badge variant="soft">
            {profile.config.work_kinds.length > 1
              ? `${labelOf(profile.config.work_kinds, release.work_kind)} · ${labelOf(allOf(profile.config, 'release_kinds'), release.kind)}`
              : labelOf(allOf(profile.config, 'release_kinds'), release.kind)}
          </Badge>
          {/* How finished the work is, beside how ready the release is: the
              marks say whether it could go out, the dial says whether it is
              done. */}
          {release.work_stage !== null && (
            <StageDial
              percent={release.work_stage}
              stage={stageAt(profile.config, release.work_stage)}
            />
          )}
          <ReadyMarks readiness={release.readiness} released={false} daysLeft={daysLeft} />
          {/* Whether its song repeats one out or booked nearby (ADR 0054):
              a week about to go out is the last place to notice. */}
          <WorkRepeatMark workId={release.work_id} />
          {/* The day itself at the end, as the mockup has it: inside a week
              the date is what gets said aloud, and the time when there is
              one - a premiere is booked to the hour. */}
          <time dateTime={release.scheduled_at ?? undefined} className="font-mono">
            {formatDay(release.scheduled_at as string)}
            {release.scheduled_time === null ? '' : ` · ${release.scheduled_time}`}
          </time>
        </>
      }
    >
      {release.work_title}
    </RowButton>
  )
}
