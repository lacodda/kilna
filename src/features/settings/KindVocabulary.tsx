import { useTranslation } from 'react-i18next'
import type { WorkKind } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { AxesEditor } from '@/features/settings/AxesEditor'
import { PublicationEditor } from '@/features/settings/PublicationEditor'
import { ReleaseFieldsEditor } from '@/features/settings/ReleaseFieldsEditor'
import { TiersEditor } from '@/features/settings/TiersEditor'
import { Vocabulary } from '@/features/settings/Vocabulary'

/**
 * One kind's own vocabulary: its axes, tiers, statuses and release kinds, and
 * what its works are called and shaped like when made from another.
 *
 * Axis keys are deliberately not editable here either — the same past-score
 * reasoning applies per kind now, not just per profile.
 */
export function KindVocabulary({
  kind,
  onChange,
}: {
  kind: WorkKind
  onChange: (changes: Partial<WorkKind>) => void
}) {
  const { t } = useTranslation()

  return (
    <section className="flex flex-col gap-4">
      <h3 className="caption">{sayLabel(kind.label)}</h3>

      <AxesEditor axes={kind.axes ?? []} onChange={(axes) => onChange({ axes })} />
      <TiersEditor tiers={kind.tiers ?? []} onChange={(tiers) => onChange({ tiers })} />
      <Vocabulary
        label={t('editor.statuses')}
        entries={kind.statuses ?? []}
        onChange={(statuses) => onChange({ statuses })}
      />
      <Vocabulary
        label={t('editor.releaseKinds')}
        entries={kind.release_kinds ?? []}
        onChange={(release_kinds) => onChange({ release_kinds })}
      />
      {/* What a release of each kind says about itself. Under the release
          kinds because that is what it belongs to, and only for the kinds
          that have any: a profile that says nothing about its releases
          should show an empty screen, not an invitation. */}
      {(kind.release_kinds ?? []).some((entry) => (entry.fields ?? []).length > 0) && (
        <ReleaseFieldsEditor
          kinds={kind.release_kinds ?? []}
          onChange={(release_kinds) => onChange({ release_kinds })}
        />
      )}
      {/* After the doors, since a cover's shape is a door's: the place
          decides the shape of the picture a release goes out with. */}
      <PublicationEditor kind={kind} onChange={onChange} />
      {/* The storyboard's words, for a kind that has any: a song lists none
          and shows nothing here. Keys, as everywhere on this screen, come
          from the document; the labels are what is renamed. */}
      {(kind.shot_types ?? []).length > 0 && (
        <Vocabulary
          label={t('editor.shotTypes')}
          entries={kind.shot_types ?? []}
          onChange={(shot_types) => onChange({ shot_types })}
        />
      )}
      {(kind.scene_blocks ?? []).length > 0 && (
        <Vocabulary
          label={t('editor.sceneBlocks')}
          entries={kind.scene_blocks ?? []}
          onChange={(scene_blocks) => onChange({ scene_blocks })}
        />
      )}
    </section>
  )
}
