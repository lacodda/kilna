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
    <div className="flex max-w-3xl flex-col gap-6">
      <section className="flex flex-col gap-3">
        <h3 className="text-sm font-semibold">{t('settings.appearance')}</h3>
        <div className="grid gap-3 sm:grid-cols-2">
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
        </div>
      </section>
    </div>
  )
}
