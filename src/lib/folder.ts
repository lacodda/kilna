import type { TFunction } from 'i18next'

/*
 * A kind's folder on disk, as the profile names it (ADR 0057): a path under
 * the media folder, with `{title}`, a field's `{key}`, or either read off what
 * the work is made from after `origin.`.
 *
 * The same rules the backend holds the profile to (`folder::problem`), checked
 * as the template is typed so the box says what is wrong before the bar does.
 */

/** A character no folder name may hold on Windows, where the rules are the
 *  tightest: a template written on one machine names folders on all of them. */
// eslint-disable-next-line no-control-regex
const FORBIDDEN = /[<>:"|?*\u0000-\u001f]/

/** What is wrong with a folder template, or `null` when it can name a folder
 *  (an empty one removes it). */
export function folderProblem(text: string, t: TFunction): string | null {
  const template = text.trim()
  if (template === '') return null
  if (/^[\\/]/.test(template) || /^[A-Za-z]:/.test(template)) return t('editor.folderAbsolute')
  for (const raw of template.split(/[\\/]/)) {
    const segment = raw.trim()
    if (segment === '') return t('editor.folderEmptyStep')
    if (segment === '.' || segment === '..') return t('editor.folderClimbs')
    let rest = segment
    for (;;) {
      const open = rest.indexOf('{')
      const words = open === -1 ? rest : rest.slice(0, open)
      if (words.includes('}')) return t('editor.folderBraces')
      if (FORBIDDEN.test(words)) return t('editor.folderForbidden', { text: words })
      if (open === -1) break
      const close = rest.indexOf('}', open)
      if (close === -1) return t('editor.folderBraces')
      const name = rest.slice(open + 1, close)
      if (!/^(origin\.)?[A-Za-z0-9_-]+$/.test(name)) {
        return t('editor.folderPlaceholder', { name: `{${name}}` })
      }
      rest = rest.slice(close + 1)
    }
  }
  return null
}
