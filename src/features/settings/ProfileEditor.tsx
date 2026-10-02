import { Fragment } from 'react'
import { useTranslation } from 'react-i18next'
import type { ProfileConfig, WorkKind } from '@/lib/api/types'
import { allOf, useProfile } from '@/lib/useProfile'
import { Divider } from '@/components/ui/divider'
import { ActionsEditor } from '@/features/settings/ActionsEditor'
import { KindVocabulary } from '@/features/settings/KindVocabulary'
import { CoverIdeasEditor } from '@/features/settings/CoverIdeasEditor'
import { GuardEditor } from '@/features/settings/GuardEditor'
import { RhythmEditor } from '@/features/settings/RhythmEditor'
import { Vocabulary } from '@/features/settings/Vocabulary'
import { useProfileDraft } from '@/features/settings/useProfileDraft'

// Editing the scenario, not designing a schema: the tables never change, only
// the vocabulary and the criteria. Axis keys are deliberately not editable —
// past score snapshots are keyed by them, and renaming a key would orphan them.
//
// What is edited is the profile's draft (`useProfileDraft`), not a copy in
// this component: it outlives the section, and it is saved or discarded from
// the bar at the foot of the pane rather than from the end of this form.
export function ProfileEditor() {
  const { t } = useTranslation()
  const profile = useProfile()
  const { config, edit } = useProfileDraft()

  const patch = (changes: Partial<ProfileConfig>) => {
    edit((current) => ({ ...current, ...changes }))
  }

  // Writes go back into the same `work_kinds[i]` entry: copy the config,
  // replace the one kind, keep `format` and everything else untouched.
  const setKind = (index: number, changes: Partial<WorkKind>) => {
    edit((current) => ({
      ...current,
      work_kinds: current.work_kinds.map((kind, i) =>
        i === index ? { ...kind, ...changes } : kind,
      ),
    }))
  }

  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <header className="flex flex-col gap-1">
        <h3 className="caption">{profile.name}</h3>
        {profile.description === null ? null : (
          <p className="text-xs text-dim">{profile.description}</p>
        )}
      </header>

      {/* Since v0.57 the vocabulary belongs to the kind, not the profile: a
          song and a video are judged on different axes and go out through
          different doors. One group per `work_kinds[]` entry, headed by the
          kind's own label. */}
      {config.work_kinds.map((kind, kindIndex) => (
        <Fragment key={kind.key}>
          <Divider />
          <KindVocabulary kind={kind} onChange={(changes) => setKind(kindIndex, changes)} />
        </Fragment>
      ))}

      <Divider />
      <RhythmEditor rhythm={config.rhythm} onChange={(rhythm) => patch({ rhythm })} />
      <CoverIdeasEditor config={config} onChange={(cover_ideas) => patch({ cover_ideas })} />
      <GuardEditor config={config} onChange={(guard) => patch({ guard })} />
      <Vocabulary
        label={t('editor.workKinds')}
        help={t('editor.keysHint')}
        entries={config.work_kinds}
        onChange={(work_kinds) => patch({ work_kinds })}
      />

      <Divider />
      <ActionsEditor
        actions={config.prompts}
        kinds={config.work_kinds}
        roles={allOf(config, 'version_roles')}
        onChange={(prompts) => patch({ prompts })}
      />
    </div>
  )
}
