import { useTranslation } from 'react-i18next'
import type { WorkKind } from '@/lib/api/types'
import { fileRuleProblem, folderProblem } from '@/lib/folder'
import { AxesEditor } from '@/features/settings/AxesEditor'
import { CheckedInput } from '@/features/settings/CheckedInput'
import {
  ReleaseKindsEditor,
  RolesEditor,
  StatusesEditor,
  WordsEditor,
} from '@/features/settings/KindWords'
import { PublicationEditor } from '@/features/settings/PublicationEditor'
import { ReadinessEditor } from '@/features/settings/ReadinessEditor'
import { ReleaseFieldsEditor } from '@/features/settings/ReleaseFieldsEditor'
import { TiersEditor } from '@/features/settings/TiersEditor'
import type { ProblemsAt } from '@/features/settings/useProfileProblems'

/**
 * One kind's own vocabulary, whole (v0.112): what its works are judged on and
 * by which thresholds, the statuses they move through, the bodies they carry,
 * the doors they go out through and how each weighs the score - every list
 * added to, taken from and put in order - and what its works are called and
 * shaped like when made from another.
 *
 * Keys stand once made, here as everywhere on this screen: past scores,
 * versions and releases are stored under them.
 */
export function KindVocabulary({
  kind,
  onChange,
  problemsAt,
}: {
  kind: WorkKind
  onChange: (changes: Partial<WorkKind>) => void
  /** What is wrong, by place - said under the row it is about. The places
   *  with a row of their own are the ones `ROWS` in the editor names. */
  problemsAt: ProblemsAt
}) {
  const { t } = useTranslation()
  const at = (what: string) => (key: string) => problemsAt(['workKind', kind.key], [what, key])
  const hasBoard = (kind.shot_types ?? []).length > 0 || (kind.scene_blocks ?? []).length > 0

  return (
    <section className="flex flex-col gap-5">
      <StatusesEditor
        statuses={kind.statuses ?? []}
        onChange={(statuses) => onChange({ statuses })}
      />
      <RolesEditor
        roles={kind.version_roles ?? []}
        onChange={(version_roles) => onChange({ version_roles })}
        problems={at('versionRole')}
      />
      <AxesEditor
        axes={kind.axes ?? []}
        onChange={(axes) => onChange({ axes })}
        problems={at('axis')}
      />
      <TiersEditor
        tiers={kind.tiers ?? []}
        onChange={(tiers) => onChange({ tiers })}
        problems={at('tier')}
      />
      <ReleaseKindsEditor
        kinds={kind.release_kinds ?? []}
        axes={kind.axes ?? []}
        onChange={(release_kinds) => onChange({ release_kinds })}
        problems={at('releaseKind')}
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
      {/* What each door will not let a release out without (v0.97): the
          checklist the calendar's marks and the release's own row read. */}
      {(kind.release_kinds ?? []).length > 0 && <ReadinessEditor kind={kind} onChange={onChange} />}
      {/* After the doors, since a cover's shape is a door's: the place
          decides the shape of the picture a release goes out with. */}
      <PublicationEditor kind={kind} onChange={onChange} />
      {/* Where a work of the kind keeps its files on disk, under the media
          folder this machine names (v0.93, ADR 0057). Checked as it is
          typed, by the rules the profile is saved under. */}
      <CheckedInput
        label={t('editor.folder')}
        help={t('editor.folderHint')}
        className="font-mono"
        placeholder="songs/{title}"
        value={kind.folder ?? ''}
        check={(text) => folderProblem(text, t)}
        onCommit={(text) => onChange({ folder: text.trim() === '' ? null : text.trim() })}
      />
      {/* Which file in that folder the work is finished as (v0.101, ADR
          0070): the newest the pattern matches, unless one is chosen in the
          Files tab. What a cutter cuts a short from. */}
      <CheckedInput
        label={t('editor.file')}
        help={t('editor.fileHint')}
        className="font-mono"
        placeholder="video/*.mp4"
        value={kind.file ?? ''}
        check={(text) => fileRuleProblem(text, t)}
        onCommit={(text) => onChange({ file: text.trim() === '' ? null : text.trim() })}
      />
      {/* The storyboard's words, for a kind that has a board: a song has
          none and shows nothing here. A board itself is a fact of the kind's
          shape - its works have a Scenes tab - and is given in the profile
          document; its words are edited here. */}
      {hasBoard && (
        <>
          <WordsEditor
            label={t('editor.shotTypes')}
            entries={kind.shot_types ?? []}
            onChange={(shot_types) => onChange({ shot_types })}
            make={(key) => ({ key, label: key })}
          />
          <WordsEditor
            label={t('editor.layers')}
            help={t('editor.layersHint')}
            entries={kind.layers ?? []}
            onChange={(layers) => onChange({ layers })}
            make={(key) => ({ key, label: key })}
          />
          {(kind.scene_blocks ?? []).length > 0 && (
            <WordsEditor
              label={t('editor.sceneBlocks')}
              entries={kind.scene_blocks ?? []}
              onChange={(scene_blocks) => onChange({ scene_blocks })}
            />
          )}
        </>
      )}
    </section>
  )
}
