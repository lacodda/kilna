import { useTranslation } from 'react-i18next'
import type { VibeAxes, VibeCharacter, VibeComplexity, VibeGlyph, VibeMood } from '@/lib/api/types'
import { Select } from '@/components/AppSelect'
import { Input } from '@/components/ui/input'
import { KeyedList } from '@/features/settings/KeyedList'
import { LabelInput } from '@/features/settings/LabelInput'

const GLYPHS: VibeGlyph[] = ['calm', 'playful', 'steady', 'edgy', 'drive']

/**
 * The three lines a work's feel is said along (v0.129, FR-02, FR-16): the
 * moods and the colour each is drawn in, the characters and their shape, the
 * complexities and their strokes. Renamed, recoloured, added to and taken
 * from here; what a line holds is checked as the rest of the profile is.
 */
export function VibeEditor({
  vibe,
  onChange,
}: {
  vibe: VibeAxes | null | undefined
  onChange: (vibe: VibeAxes) => void
}) {
  const { t } = useTranslation()
  if (vibe == null) {
    return <p className="text-xs text-dim">{t('editor.vibe.none')}</p>
  }
  const glyphs = GLYPHS.map((glyph) => ({ value: glyph, label: t(`editor.vibe.glyph.${glyph}`) }))
  const strokes = [1, 2, 3].map((n) => ({
    value: String(n),
    label: t('editor.vibe.strokes', { count: n }),
  }))

  return (
    <section className="flex flex-col gap-5">
      <p className="text-xs text-dim">{t('editor.vibe.hint')}</p>
      <KeyedList<VibeMood>
        label={t('vibe.line.mood')}
        entries={vibe.moods}
        onChange={(moods) => onChange({ ...vibe, moods })}
        // A new mood starts in the colour of the first, to be told apart by hand.
        make={(key) => ({ key, label: key, color: vibe.moods[0]?.color ?? '' })}
      >
        {(mood, set) => (
          <>
            <LabelInput
              className="flex-1"
              label={mood.label}
              onChange={(label) => set({ label: label ?? '' })}
              aria-label={t('editor.wordOf', { key: mood.key })}
            />
            <Input
              className="w-28 font-mono text-xs"
              value={mood.color}
              aria-label={t('editor.vibe.colourOf', { key: mood.key })}
              onChange={(event) => set({ color: event.target.value })}
            />
          </>
        )}
      </KeyedList>
      <KeyedList<VibeCharacter>
        label={t('vibe.line.character')}
        entries={vibe.characters}
        onChange={(characters) => onChange({ ...vibe, characters })}
        make={(key) => ({ key, label: key, glyph: 'steady' })}
      >
        {(character, set) => (
          <>
            <LabelInput
              className="flex-1"
              label={character.label}
              onChange={(label) => set({ label: label ?? '' })}
              aria-label={t('editor.wordOf', { key: character.key })}
            />
            <Select
              className="w-36"
              aria-label={t('editor.vibe.glyphOf', { key: character.key })}
              value={character.glyph}
              onChange={(glyph) => set({ glyph: glyph as VibeGlyph })}
              options={glyphs}
            />
          </>
        )}
      </KeyedList>
      <KeyedList<VibeComplexity>
        label={t('vibe.line.complexity')}
        entries={vibe.complexities}
        onChange={(complexities) => onChange({ ...vibe, complexities })}
        make={(key) => ({ key, label: key, strokes: 2 })}
      >
        {(complexity, set) => (
          <>
            <LabelInput
              className="flex-1"
              label={complexity.label}
              onChange={(label) => set({ label: label ?? '' })}
              aria-label={t('editor.wordOf', { key: complexity.key })}
            />
            <Select
              className="w-36"
              aria-label={t('editor.vibe.strokesOf', { key: complexity.key })}
              value={String(complexity.strokes)}
              onChange={(value) => set({ strokes: Number(value) })}
              options={strokes}
            />
          </>
        )}
      </KeyedList>
    </section>
  )
}
