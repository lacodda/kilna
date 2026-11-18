import { useTranslation } from 'react-i18next'
import type { CanonSection, NoteKind, SectionShape } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { KeyedList } from '@/features/settings/KeyedList'
import { LabelInput } from '@/features/settings/LabelInput'
import type { ProblemsAt } from '@/features/settings/useProfileProblems'

const SHAPES: SectionShape[] = [
  'facts',
  'slots',
  'details',
  'palette',
  'marks',
  'styles',
  'bans',
  'relations',
  'appearances',
]

/**
 * The structure of the canon (v0.129, FR-16): each kind of card and the
 * sections a card of it is filled in by - their words, their hints, and the
 * shape each section holds. A kind of note with no sections is a plain note,
 * not a card, and is not here.
 */
export function CanonEditor({
  kinds,
  onChange,
  problemsAt,
}: {
  kinds: NoteKind[]
  onChange: (kinds: NoteKind[]) => void
  problemsAt: ProblemsAt
}) {
  const { t } = useTranslation()
  const cards = kinds.filter((kind) => (kind.sections ?? []).length > 0)
  const shapes = SHAPES.map((shape) => ({ value: shape, label: t(`editor.canon.shape.${shape}`) }))

  if (cards.length === 0) {
    return <p className="text-xs text-dim">{t('editor.canon.none')}</p>
  }
  const setSections = (key: string, sections: CanonSection[]) =>
    onChange(kinds.map((kind) => (kind.key === key ? { ...kind, sections } : kind)))

  return (
    <section className="flex flex-col gap-5">
      <p className="text-xs text-dim">{t('editor.canon.hint')}</p>
      {cards.map((kind) => (
        <KeyedList<CanonSection>
          key={kind.key}
          boxed
          label={t('editor.canon.sectionsOf', { kind: sayLabel(kind.label) })}
          entries={kind.sections ?? []}
          onChange={(sections) => setSections(kind.key, sections)}
          make={(key) => ({ key, label: key, shape: 'facts' })}
          addLabel={t('editor.canon.addSection')}
          problems={(key) => problemsAt(['noteKind', kind.key], ['section', key])}
          below={(section, set) => (
            <LabelInput
              optional
              label={section.hint}
              placeholder={t('editor.canon.sectionHint')}
              aria-label={t('editor.canon.sectionHintOf', {
                key: section.key,
                kind: sayLabel(kind.label),
              })}
              onChange={(hint) => set({ hint })}
            />
          )}
        >
          {(section, set) => (
            <>
              <LabelInput
                className="flex-1"
                label={section.label}
                onChange={(label) => set({ label: label ?? '' })}
                aria-label={t('editor.canon.wordOf', {
                  key: section.key,
                  kind: sayLabel(kind.label),
                })}
              />
              <Select
                className="w-40"
                aria-label={t('editor.canon.shapeOf', {
                  key: section.key,
                  kind: sayLabel(kind.label),
                })}
                value={section.shape ?? 'facts'}
                onChange={(shape) => set({ shape: shape as SectionShape })}
                options={shapes}
              />
            </>
          )}
        </KeyedList>
      ))}
    </section>
  )
}
