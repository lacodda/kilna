import { useTranslation } from 'react-i18next'
import { LANGUAGES, useLanguage } from '@/lib/language'
import { THEMES, useTheme } from '@/lib/theme'
import { FieldGroup } from '@/components/ui/field'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Select } from '@/components/AppSelect'

/**
 * Appearance: the theme and the language.
 *
 * Both are also cycled from the rail's footer, which is where they were found
 * until now; here they are named in full, as a settings screen names things,
 * so that "which language am I on" has a place to be read rather than guessed
 * from a button that shows only the next state.
 */
export function GeneralSection() {
  const { t } = useTranslation()
  const { theme, setTheme } = useTheme()
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
    </section>
  )
}
