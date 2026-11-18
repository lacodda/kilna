import { CanonEditor } from '@/features/settings/CanonEditor'
import { VibeEditor } from '@/features/settings/VibeEditor'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ProfileConfig, WorkKind } from '@/lib/api/types'
import { sayReason } from '@/lib/errors'
import { hasRow, partOf, type ProfilePart } from '@/lib/profileProblems'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { StatusDot } from '@/components/ui/status-dot'
import { CraftEditor } from '@/features/settings/CraftEditor'
import { KindVocabulary } from '@/features/settings/KindVocabulary'
import { MetaFieldsEditor } from '@/features/settings/MetaFieldsEditor'
import { ProfileTools } from '@/features/settings/ProfileTools'
import { useProfileDraft } from '@/features/settings/useProfileDraft'
import { useProfileProblems, useSaidProblems } from '@/features/settings/useProfileProblems'

// Editing the scenario, not designing a schema: the tables never change, only
// the vocabulary and the criteria. Keys are typed once and then stand — past
// scores, versions and releases are stored under them, and renaming a key
// would orphan them.
//
// What is edited is the profile's draft (`useProfileDraft`), not a copy in
// this component: it outlives the section, and it is saved or discarded from
// the bar at the foot of the pane rather than from the end of this form.
//
// Since v0.112 the whole document is edited here, and it is laid out in
// parts rather than down one page: the craft as a whole, each kind of work,
// the fields, the actions. A part with something that will not save carries
// a mark, and the problem is said beside the row it is about.
// The part last open, kept while the window lives: leaving the section for
// the catalogue and coming back finds the kind that was being edited, as it
// finds the draft.
let lastOpen: ProfilePart = 'craft'

export function ProfileEditor() {
  const { t } = useTranslation()
  const profile = useProfile()
  const { config, edit, dirty } = useProfileDraft()
  // What is wrong with an action is said in Skills, where actions are
  // edited (v0.126).
  const problems = useProfileProblems(config, dirty).filter(
    (problem) => partOf(problem) !== 'actions',
  )
  const problemsAt = useSaidProblems(problems)
  const [chosen, setChosenPart] = useState<ProfilePart>(lastOpen)
  const setChosen = (part: ProfilePart) => {
    lastOpen = part
    setChosenPart(part)
  }

  const patch = (changes: Partial<ProfileConfig>) => {
    edit((current) => ({ ...current, ...changes }))
  }

  // Writes go back into the same `work_kinds` entry: copy the config,
  // replace the one kind, keep `format` and everything else untouched.
  const setKind = (key: string, changes: Partial<WorkKind>) => {
    edit((current) => ({
      ...current,
      work_kinds: current.work_kinds.map((kind) =>
        kind.key === key ? { ...kind, ...changes } : kind,
      ),
    }))
  }

  const parts: { id: ProfilePart; label: string }[] = [
    { id: 'craft', label: t('editor.part.craft') },
    ...config.work_kinds.map((kind) => ({
      id: `kind:${kind.key}` as const,
      label: sayLabel(kind.label) || kind.key,
    })),
    { id: 'fields', label: t('editor.part.fields') },
    { id: 'canon', label: t('editor.part.canon') },
    { id: 'vibe', label: t('editor.part.vibe') },
  ]
  // A kind taken out while its part was open leaves the craft open instead.
  const part = parts.some((entry) => entry.id === chosen) ? chosen : 'craft'
  const kind = part.startsWith('kind:')
    ? config.work_kinds.find((entry) => `kind:${entry.key}` === part)
    : undefined
  // A problem of a kind the draft no longer has is the craft's to show.
  const shownIn = (problem: (typeof problems)[number]): ProfilePart => {
    const where = partOf(problem)
    return parts.some((entry) => entry.id === where) ? where : 'craft'
  }
  const marked = new Set(problems.map(shownIn))
  // What is wrong in the open part somewhere no row of it stands for - the
  // profile as a whole, a list with a key twice, a kind with no status: said
  // at the head of the part. The rest is said under its row.
  const loose = problems.filter((problem) => shownIn(problem) === part && !hasRow(problem, config))

  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <header className="flex flex-col gap-2">
        <div className="flex flex-col gap-1">
          <h3 className="caption">{profile.name}</h3>
          {profile.description === null ? null : (
            <p className="text-xs text-dim">{profile.description}</p>
          )}
        </div>
        <ProfileTools dirty={dirty} />
      </header>

      <ChipGroup
        aria-label={t('editor.parts')}
        value={[part]}
        // A group that holds one: pressing the open part again keeps it open.
        onValueChange={(next) => {
          const picked = next.find((id) => id !== part) ?? next[0]
          if (picked !== undefined) setChosen(picked as ProfilePart)
        }}
      >
        {parts.map((entry) => (
          <Chip key={entry.id} value={entry.id}>
            {entry.label}
            {marked.has(entry.id) && (
              <StatusDot status="bad" size="sm" label={t('editor.partWrong')} className="ml-1.5" />
            )}
          </Chip>
        ))}
      </ChipGroup>

      {loose.length > 0 && (
        <ul role="alert" className="flex flex-col gap-1 text-xs text-bad">
          {loose.map((problem, index) => (
            <li key={index}>{sayReason(problem)}</li>
          ))}
        </ul>
      )}

      {part === 'craft' && <CraftEditor config={config} patch={patch} problemsAt={problemsAt} />}
      {kind !== undefined && (
        <KindVocabulary
          // A part of its own per kind: what was typed for one kind's new
          // axis must not stand in the box of the next.
          key={kind.key}
          kind={kind}
          onChange={(changes) => setKind(kind.key, changes)}
          problemsAt={problemsAt}
        />
      )}
      {part === 'canon' && (
        <CanonEditor
          kinds={config.note_kinds ?? []}
          onChange={(note_kinds) => patch({ note_kinds })}
          problemsAt={problemsAt}
        />
      )}
      {part === 'vibe' && <VibeEditor vibe={config.vibe} onChange={(vibe) => patch({ vibe })} />}
      {part === 'fields' && (
        <MetaFieldsEditor
          fields={config.work_meta_fields}
          kinds={config.work_kinds}
          onChange={(work_meta_fields) => patch({ work_meta_fields })}
          problems={(key) => problemsAt(['metaField', key])}
        />
      )}
    </div>
  )
}
