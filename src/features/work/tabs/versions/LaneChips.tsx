import { useTranslation } from 'react-i18next'
import type { VersionRole } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { Chip, ChipGroup } from '@/components/ui/chip'

interface Props {
  roles: VersionRole[]
  /** How many versions each role holds, by role key. */
  counts: Record<string, number>
  /** The role whose versions are listed. */
  value: string
  onPick: (role: string) => void
}

/**
 * Every role of the work as a chip, with how many versions it holds.
 *
 * All of them, commentary included - the decision of 22.09 (the first one):
 * "Lyrics 10 · Style 3 · Review 1 · Critique 0". Until v0.80 a review was left
 * out, on the grounds that it belongs beside the text it discusses - which it
 * still is, beside the revision it reviews - but a lane nobody could pick
 * meant a link to a review opened with no chip pressed, and the history of
 * reviews could only be walked one revision of the text at a time. A role
 * with nothing in it shows its zero: that is the answer to "has this been
 * critiqued", and hiding it would hide the question.
 *
 * Chips rather than a segment: a craft ships four roles and a two-way switch
 * does not stretch to four.
 */
export function LaneChips({ roles, counts, value, onPick }: Props) {
  const { t } = useTranslation()

  return (
    <ChipGroup
      aria-label={t('versions.lanes')}
      value={[value]}
      // One lane is always open: pressing the open one again lets go of it in
      // the group, and that is not a lane to switch to.
      onValueChange={([next]) => {
        if (next !== undefined) onPick(next)
      }}
    >
      {roles.map((lane) => (
        <Chip key={lane.key} value={lane.key} count={counts[lane.key] ?? 0}>
          {sayLabel(lane.label)}
        </Chip>
      ))}
    </ChipGroup>
  )
}
