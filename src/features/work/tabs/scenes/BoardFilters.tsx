import { useTranslation } from 'react-i18next'
import type { SceneNote } from '@/lib/api/types'
import { say as sayLabel, type Vocabulary } from '@/lib/useProfile'
import { Chip, ChipGroup } from '@/components/ui/chip'

/** The "All" chip's value in the row of kinds of shot. A chip in a group needs
    a value of its own, and an empty one is not taken as a value at all. */
const ALL_SHOTS = '*'

interface Props {
  vocabulary: Vocabulary
  /** How many scenes the whole board has, and how many of each kind of shot. */
  total: number
  counts: ReadonlyMap<string, number>
  shotType: string | undefined
  onShotType: (shot: string | undefined) => void
  /** The notes the board's scenes name that the board can be narrowed to. */
  named: SceneNote[]
  withNote: string | undefined
  onWithNote: (noteId: string | undefined) => void
}

/**
 * What the board is narrowed to: a kind of shot, and who or where.
 *
 * The kind of shot as a row of chips, the way the catalogue narrows to a
 * kind: "show me every detail" is one click, and the chip that is on turns
 * off - which lets go of the group and puts "All" back on. Hidden while the
 * kind names no kinds of shot.
 *
 * Who and where beside it: "every scene with her in it" is one click. Only
 * the characters and places some scene of this board actually names — a cast
 * list of everyone in the workspace would be a list nobody reads.
 */
export function BoardFilters({
  vocabulary,
  total,
  counts,
  shotType,
  onShotType,
  named,
  withNote,
  onWithNote,
}: Props) {
  const { t } = useTranslation()
  const shots = vocabulary.shot_types.length > 0

  if (!shots && named.length === 0) return null

  return (
    <div className="flex w-full flex-wrap items-center gap-x-4 gap-y-2">
      {shots && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="caption">{t('scenes.shotType')}</span>
          <ChipGroup
            aria-label={t('scenes.shotType')}
            value={[shotType ?? ALL_SHOTS]}
            onValueChange={(next) => {
              const picked = next[0]
              onShotType(picked === undefined || picked === ALL_SHOTS ? undefined : picked)
            }}
          >
            {[
              { key: ALL_SHOTS, label: t('scenes.allShots'), count: total },
              ...vocabulary.shot_types.map((shot) => ({
                key: shot.key,
                label: sayLabel(shot.label),
                count: counts.get(shot.key) ?? 0,
              })),
            ].map((entry) => (
              <Chip key={entry.key} value={entry.key} count={entry.count}>
                {entry.label}
              </Chip>
            ))}
          </ChipGroup>
        </div>
      )}

      {named.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="caption">{t('scenes.about')}</span>
          <ChipGroup
            aria-label={t('scenes.about')}
            value={withNote === undefined ? [] : [withNote]}
            onValueChange={(next) => onWithNote(next[0])}
          >
            {named.map((link) => (
              <Chip key={link.note_id} value={link.note_id}>
                {link.note_title ?? t('scenes.untitledNote')}
              </Chip>
            ))}
          </ChipGroup>
        </div>
      )}
    </div>
  )
}
