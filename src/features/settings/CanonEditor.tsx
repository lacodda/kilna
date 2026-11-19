import { Switch } from '@/components/ui/switch'
import { useTranslation } from 'react-i18next'
import type { CanonSection, NoteKind, SectionShape } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { Input } from '@/components/ui/input'
import { KeyedList } from '@/features/settings/KeyedList'
import { LabelInput } from '@/features/settings/LabelInput'
import type { ProblemsAt } from '@/features/settings/useProfileProblems'

const SHAPES: SectionShape[] = [
  'facts',
  'slots',
  'details',
  'palette',
  'marks',
  'branding',
  'quotas',
  'bank',
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
  // Off, a kind is not offered for a new card and a section is not offered
  // to add (v0.141, FR-18 18.1); what was made of them stays.
  const setOff = (key: string, off: boolean) =>
    onChange(kinds.map((kind) => (kind.key === key ? { ...kind, off: off || undefined } : kind)))

  return (
    <section className="flex flex-col gap-5">
      <p className="text-xs text-dim">{t('editor.canon.hint')}</p>
      {cards.map((kind) => (
        <div key={kind.key} className="flex flex-col gap-1.5">
          <label className="flex items-center gap-2 text-sm">
            <Switch
              checked={kind.off !== true}
              onCheckedChange={(on) => setOff(kind.key, !on)}
              aria-label={t('editor.canon.kindOfferedOf', { kind: sayLabel(kind.label) })}
            />
            <span>{t('editor.canon.kindOffered', { kind: sayLabel(kind.label) })}</span>
          </label>
          <KeyedList<CanonSection>
            boxed
            label={t('editor.canon.sectionsOf', { kind: sayLabel(kind.label) })}
            entries={kind.sections ?? []}
            onChange={(sections) => setSections(kind.key, sections)}
            make={(key) => ({ key, label: key, shape: 'facts' })}
            addLabel={t('editor.canon.addSection')}
            problems={(key) => problemsAt(['noteKind', kind.key], ['section', key])}
            below={(section, set) => (
              <>
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
                {/* The work field a bank's items are named by (v0.151): a
                    release going out marks the item its work names. */}
                {section.shape === 'bank' && (
                  <Input
                    value={section.binds ?? ''}
                    onChange={(event) => {
                      const binds = event.target.value.trim()
                      set({ binds: binds === '' ? undefined : binds })
                    }}
                    placeholder={t('editor.canon.bindsPlaceholder')}
                    aria-label={t('editor.canon.bindsOf', {
                      key: section.key,
                      kind: sayLabel(kind.label),
                    })}
                    className="font-mono text-xs"
                  />
                )}
              </>
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
                <Switch
                  checked={section.off !== true}
                  onCheckedChange={(on) => set({ off: on ? undefined : true })}
                  aria-label={t('editor.canon.sectionOfferedOf', {
                    key: section.key,
                    kind: sayLabel(kind.label),
                  })}
                />
              </>
            )}
          </KeyedList>
        </div>
      ))}
    </section>
  )
}
