import { useTranslation } from 'react-i18next'
import type { Mark, MarkColour, ProfileConfig, Stage, StyleType, WorkKind } from '@/lib/api/types'
import { Select } from '@/components/AppSelect'
import { NumberField } from '@/components/ui/number-field'
import { CoverIdeasEditor } from '@/features/settings/CoverIdeasEditor'
import { GuardEditor } from '@/features/settings/GuardEditor'
import { KeyedList } from '@/features/settings/KeyedList'
import { useColourOptions, WordsEditor } from '@/features/settings/KindWords'
import { LabelInput } from '@/features/settings/LabelInput'
import { RhythmEditor } from '@/features/settings/RhythmEditor'
import type { ProblemsAt } from '@/features/settings/useProfileProblems'

/** A kind of work newly made: a draft to start in and a text to write, which
 *  is the least a kind can be - a kind with no status cannot hold a work. */
function newWorkKind(key: string): WorkKind {
  return {
    key,
    label: key,
    version_roles: [{ key: 'text', label: 'Text' }],
    statuses: [{ key: 'draft', label: 'Draft', derive: 'draft' }],
  }
}

/**
 * What the craft says as a whole (v0.112): the kinds of work it makes and
 * the order they are offered in, the kinds of collection, the flags and the
 * stops of the stage dial, the order of the style dictionary's types - and
 * its pace, its cover ideas and its guard of repeats.
 */
export function CraftEditor({
  config,
  patch,
  problemsAt,
}: {
  config: ProfileConfig
  patch: (changes: Partial<ProfileConfig>) => void
  problemsAt: ProblemsAt
}) {
  const { t } = useTranslation()
  const colours = useColourOptions()

  return (
    <section className="flex flex-col gap-5">
      <WordsEditor
        label={t('editor.workKinds')}
        help={t('editor.workKindsHint')}
        entries={config.work_kinds}
        onChange={(work_kinds) => patch({ work_kinds })}
        make={newWorkKind}
        addLabel={t('editor.addWorkKind')}
      />
      <WordsEditor
        label={t('editor.collectionKinds')}
        entries={config.collection_kinds}
        onChange={(collection_kinds) => patch({ collection_kinds })}
        make={(key) => ({ key, label: key })}
      />

      <KeyedList<Mark>
        label={t('editor.marks')}
        help={t('editor.marksHint')}
        entries={config.marks ?? []}
        onChange={(marks) => patch({ marks })}
        make={(key) => ({ key, label: key, colour: 'plain' })}
      >
        {(mark, set) => (
          <>
            <LabelInput
              className="flex-1"
              label={mark.label}
              onChange={(label) => set({ label: label ?? '' })}
              aria-label={t('editor.wordOf', { key: mark.key })}
            />
            <Select
              className="w-32"
              aria-label={t('editor.colourOf', { key: mark.key })}
              value={mark.colour}
              onChange={(colour) => set({ colour: colour as MarkColour })}
              options={colours}
            />
          </>
        )}
      </KeyedList>

      <KeyedList<Stage>
        label={t('editor.stages')}
        help={t('editor.stagesHint')}
        entries={config.stages ?? []}
        onChange={(stages) => patch({ stages })}
        make={(key) => ({ key, label: key, percent: 0, colour: 'plain' })}
      >
        {(stage, set) => (
          <>
            <LabelInput
              className="flex-1"
              label={stage.label}
              onChange={(label) => set({ label: label ?? '' })}
              aria-label={t('editor.wordOf', { key: stage.key })}
            />
            <NumberField
              className="w-24"
              hideStepper
              min={0}
              max={100}
              step={1}
              unit="%"
              value={stage.percent}
              onValueChange={(percent) => set({ percent: Math.round(percent ?? 0) })}
              aria-label={t('editor.stagePercent', { key: stage.key })}
            />
            <Select
              className="w-32"
              aria-label={t('editor.colourOf', { key: stage.key })}
              value={stage.colour}
              onChange={(colour) => set({ colour: colour as MarkColour })}
              options={colours}
            />
          </>
        )}
      </KeyedList>

      {/* The types of the style dictionary, renamed and put in the order the
          dictionary reads them in. A type is not added here: what a type is -
          a picture, a phrase, a colour - decides which screens draw it. */}
      {(config.style_types ?? []).length > 0 && (
        <WordsEditor<StyleType>
          label={t('editor.styleTypes')}
          help={t('editor.styleTypesHint')}
          entries={config.style_types ?? []}
          onChange={(style_types) => patch({ style_types })}
          problems={(key) => problemsAt(['styleType', key])}
        />
      )}

      <RhythmEditor config={config} onChange={(rhythm) => patch({ rhythm })} />
      <CoverIdeasEditor config={config} onChange={(cover_ideas) => patch({ cover_ideas })} />
      <GuardEditor config={config} onChange={(guard) => patch({ guard })} />
    </section>
  )
}
