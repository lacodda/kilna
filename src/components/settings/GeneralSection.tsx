import { useTranslation } from 'react-i18next'
import { LANGUAGES, useLanguage } from '@/lib/language'
import { THEMES, useTheme } from '@/lib/theme'
import { Field } from '@/components/ui/Field'
import { Select } from '@/components/ui/AppSelect'

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
          <Field label={t('settings.theme')}>
            <Select
              value={theme}
              onChange={(next) => setTheme(next as typeof theme)}
              options={THEMES.map((entry) => ({ value: entry, label: t(`themeName.${entry}`) }))}
            />
          </Field>
          <Field label={t('settings.language')}>
            <Select
              value={language}
              onChange={(next) => setLanguage(next as typeof language)}
              options={LANGUAGES.map((entry) => ({
                value: entry,
                label: t(`languageName.${entry}`),
              }))}
            />
          </Field>
        </div>
      </section>
    </div>
  )
}
