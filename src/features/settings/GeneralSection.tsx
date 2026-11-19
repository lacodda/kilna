import { PackSources } from '@/features/settings/PackSources'
import { ReceiverSettingsField } from '@/features/settings/ReceiverSettingsField'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { resetCoachMarks } from '@/components/CoachMark'
import { useTranslation } from 'react-i18next'
import { LANGUAGES, useLanguage } from '@/lib/language'
import { THEMES, useTheme } from '@/lib/theme'
import { DENSITIES, useDensity } from '@/lib/density'
import { EDITOR_FONTS, EDITOR_SIZES, useEditorText } from '@/lib/editorText'
import { FieldGroup } from '@/components/ui/field'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Select } from '@/components/AppSelect'

/**
 * Appearance: the theme, the density and the language.
 *
 * Both are also cycled from the rail's footer, which is where they were found
 * until now; here they are named in full, as a settings screen names things,
 * so that "which language am I on" has a place to be read rather than guessed
 * from a button that shows only the next state.
 */
export function GeneralSection() {
  const { t } = useTranslation()
  const { theme, setTheme } = useTheme()
  const { density, setDensity } = useDensity()
  const editor = useEditorText()
  const { language, setLanguage } = useLanguage()

  return (
    // One field under another at the width of a field, as the mockup lays a
    // settings group out: side by side, a segment and a select read as one
    // control of two halves.
    <section className="flex max-w-105 flex-col gap-4">
      <h3 className="caption">{t('settings.appearance')}</h3>
      {/* Three options, all worth seeing at once: a segment, not a menu -
          the choice is one press, and switching back is the same press. */}
      <FieldGroup label={t('settings.theme')}>
        <SegmentedControl
          aria-label={t('settings.theme')}
          value={theme}
          onValueChange={(next) => setTheme(next as typeof theme)}
        >
          {THEMES.map((entry) => (
            <Segment key={entry} value={entry}>
              {t(`themeName.${entry}`)}
            </Segment>
          ))}
        </SegmentedControl>
      </FieldGroup>
      {/* How tight the rows sit (v0.163): a long catalogue on a laptop
          wants more of it on screen, a large monitor room to breathe. */}
      <FieldGroup label={t('settings.density')} help={t('settings.densityHelp')}>
        <SegmentedControl
          aria-label={t('settings.density')}
          value={density}
          onValueChange={(next) => setDensity(next as typeof density)}
        >
          {DENSITIES.map((entry) => (
            <Segment key={entry} value={entry}>
              {t(`densityName.${entry}`)}
            </Segment>
          ))}
        </SegmentedControl>
      </FieldGroup>
      {/* A version's text (v0.163): its size, and the face of a sung text -
          a style prompt stays monospace, its spacing is what is pasted. */}
      <FieldGroup label={t('settings.editorSize')}>
        <SegmentedControl
          aria-label={t('settings.editorSize')}
          value={editor.size}
          onValueChange={(next) => editor.setSize(next as typeof editor.size)}
        >
          {EDITOR_SIZES.map((entry) => (
            <Segment key={entry} value={entry}>
              {t(`editorSizeName.${entry}`)}
            </Segment>
          ))}
        </SegmentedControl>
      </FieldGroup>
      <FieldGroup label={t('settings.editorFont')} help={t('settings.editorFontHelp')}>
        <SegmentedControl
          aria-label={t('settings.editorFont')}
          value={editor.font}
          onValueChange={(next) => editor.setFont(next as typeof editor.font)}
        >
          {EDITOR_FONTS.map((entry) => (
            <Segment key={entry} value={entry}>
              {t(`editorFontName.${entry}`)}
            </Segment>
          ))}
        </SegmentedControl>
      </FieldGroup>
      <FieldGroup label={t('settings.language')}>
        <Select
          aria-label={t('settings.language')}
          value={language}
          onChange={(next) => setLanguage(next as typeof language)}
          options={LANGUAGES.map((entry) => ({
            value: entry,
            label: t(`languageName.${entry}`),
          }))}
        />
      </FieldGroup>
      <PackSources />
      <ReceiverSettingsField />
      <div className="flex flex-col gap-1">
        <h3 className="caption">{t('coach.title')}</h3>
        <div>
          <Button
            size="sm"
            onClick={() => {
              resetCoachMarks()
              say.ok(t('coach.back'))
            }}
          >
            {t('coach.again')}
          </Button>
        </div>
      </div>
    </section>
  )
}
