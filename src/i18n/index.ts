import i18n, { type FormatterModule } from 'i18next'
import { initReactI18next } from 'react-i18next'
import { isLabelMap, resolveLabel } from '@/lib/label'
import en from './locales/en.json'
import ru from './locales/ru.json'

// English is the source language, never a fallback for a missing translation.
// `tools/check-locales.mjs` holds every locale to the same shape at build time,
// so there is nothing to fall back *to*: a gap fails the build instead of
// reaching a person as a stray English line in a Russian window.
const resources = {
  en: { translation: en },
  ru: { translation: ru },
} as const

/*
 * A word of the profile's vocabulary handed to `t()` as a value - a tier, an
 * axis, a kind of work - is either text or a `{ en, ru }` map. Left to i18next
 * a map is stringified, and the window read "12.4 to [object Object]". Every
 * interpolated value passes through here instead (`alwaysFormat`), and a map
 * is said in the language the sentence is being built in, so no call site has
 * to remember to resolve it first.
 *
 * A formatter module rather than `interpolation.format`: i18next installs its
 * own formatter at init and that option is overwritten. The app uses none of
 * the built-in named formats (`{{n, number}}` and the like), so this one
 * replaces it whole; one that starts using them has to route them here.
 */
const labels: FormatterModule = {
  type: 'formatter',
  init: () => {},
  add: () => {},
  addCached: () => {},
  format: (value, format, lng) => {
    // `{{spent, usd}}`: an amount the history keeps as a bare number (what
    // the CLI priced a run at), said as money in the sentence's language.
    // `Intl` directly rather than `lib/format`, which imports this module.
    if (format === 'usd' && typeof value === 'number') {
      return new Intl.NumberFormat(lng ?? 'en', { style: 'currency', currency: 'USD' }).format(
        value,
      )
    }
    return isLabelMap(value) ? resolveLabel(value, lng ?? 'en') : value
  },
}

void i18n
  .use(labels)
  .use(initReactI18next)
  .init({
    resources,
    // The real language is chosen in `lib/language.ts` before the first paint;
    // this is only what exists until then.
    lng: 'en',
    fallbackLng: false,
    interpolation: { escapeValue: false, alwaysFormat: true },
  })

export default i18n
